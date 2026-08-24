//! 窗口 / 进程规则匹配引擎。
//!
//! 窗口规则（细）按句柄 + 标题锁定单个窗口，句柄失效时按「标题 + 进程路径」追溯；
//! 进程规则（粗）按可执行文件路径隐藏该程序的所有窗口；白名单反向声明某个程序在
//! 哪些模式下应被跳过。除内部的正则编译缓存外均为纯函数。

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use regex::Regex;

use crate::NO_TITLE;
use crate::model::{ProcessRule, WhitelistRule, WindowInfo, WindowRule};

/// 已编译正则的缓存，避免每次匹配都重新编译。
static REGEX_CACHE: LazyLock<RwLock<HashMap<String, Regex>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 缓存条目上限，超过即整体清空重建。
const REGEX_CACHE_LIMIT: usize = 512;

/// 编译并缓存一条正则；编译失败返回 `None` 且不占用缓存位。
fn cached_regex(pattern: &str) -> Option<Regex> {
    if let Ok(cache) = REGEX_CACHE.read()
        && let Some(re) = cache.get(pattern)
    {
        return Some(re.clone());
    }
    let re = Regex::new(pattern).ok()?;
    if let Ok(mut cache) = REGEX_CACHE.write() {
        if cache.len() >= REGEX_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(pattern.to_string(), re.clone());
    }
    Some(re)
}

/// 比较路径 / 映像名；Windows 上两者都大小写不敏感。
fn path_eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// 一条窗口规则针对当前存活窗口的解析结果。
#[derive(Debug, PartialEq, Eq)]
pub enum WindowResolution<'a> {
    /// 句柄仍命中原窗口（无需回填）。
    Live(&'a WindowInfo),
    /// 句柄失效，但按「标题 + 进程路径」追溯到新窗口，需回填 hwnd/title。
    Reacquired(&'a WindowInfo),
    /// 精确规则：句柄失效且追溯失败（窗口已关闭 / 进程已重启且标题不符）。
    Missing,
    /// 高级正则规则：标题匹配到的所有窗口（可能为空）。
    Regex(Vec<&'a WindowInfo>),
}

/// 校验正则是否可编译，并把编译结果预热进缓存。
pub fn regex_is_valid(pattern: &str) -> bool {
    cached_regex(pattern).is_some()
}

/// 标题是否可用于追溯（非空且非「无标题窗口」占位符）。
fn usable_title(title: &str) -> bool {
    !title.is_empty() && title != NO_TITLE
}

/// 窗口是否落在规则声明的「匹配范围」内。默认只看可见且有标题的窗口。
pub fn in_scope(w: &WindowInfo, include_untitled: bool, include_background: bool) -> bool {
    if !include_background && !w.visible {
        return false;
    }
    if !include_untitled && !usable_title(&w.title) {
        return false;
    }
    true
}

/// 规则的「位置」是否与窗口一致：优先按路径，路径为空时退回进程名。
fn location_matches(rule: &WindowRule, w: &WindowInfo) -> bool {
    if !rule.path.is_empty() {
        path_eq(&w.path, &rule.path)
    } else {
        !rule.process.is_empty() && path_eq(&w.process, &rule.process)
    }
}

/// 解析一条窗口规则命中的存活窗口。见 [`WindowResolution`]。
pub fn resolve_window_rule<'a>(
    rule: &WindowRule,
    windows: &'a [WindowInfo],
) -> WindowResolution<'a> {
    if let Some(pattern) = &rule.regex {
        let Some(re) = cached_regex(pattern) else {
            return WindowResolution::Regex(Vec::new());
        };
        return WindowResolution::Regex(
            windows
                .iter()
                .filter(|w| in_scope(w, rule.include_untitled, rule.include_background))
                .filter(|w| re.is_match(&w.title))
                .collect(),
        );
    }

    // 精确规则：先按句柄命中，并校验位置一致。
    if rule.hwnd != 0
        && let Some(w) = windows.iter().find(|w| w.hwnd == rule.hwnd)
        && (rule.path.is_empty() || path_eq(&w.path, &rule.path))
    {
        return WindowResolution::Live(w);
    }

    // 追溯：按「标题 + 进程路径」找回同一逻辑窗口。
    if usable_title(&rule.title)
        && let Some(w) = windows
            .iter()
            .find(|w| w.title == rule.title && location_matches(rule, w))
    {
        return WindowResolution::Reacquired(w);
    }

    WindowResolution::Missing
}

/// 一条进程规则命中的所有存活窗口。`by_name` 为 true 时按文件名匹配，否则按完整路径。
pub fn match_process_rule<'a>(
    rule: &ProcessRule,
    windows: &'a [WindowInfo],
) -> Vec<&'a WindowInfo> {
    let subject = |w: &WindowInfo| -> String {
        if rule.by_name {
            w.process.clone()
        } else {
            w.path.clone()
        }
    };

    let candidates = windows
        .iter()
        .filter(|w| in_scope(w, rule.include_untitled, rule.include_background));

    match &rule.regex {
        Some(pattern) => match cached_regex(pattern) {
            Some(re) => candidates.filter(|w| re.is_match(&subject(w))).collect(),
            None => Vec::new(),
        },
        None => {
            let want = if rule.by_name {
                &rule.process
            } else {
                &rule.path
            };
            if want.is_empty() {
                return Vec::new();
            }
            candidates.filter(|w| path_eq(&subject(w), want)).collect()
        }
    }
}

/// 白名单声明的忽略模式，与配置界面「白名单」页的三个开关一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreMode {
    /// 隐藏时跳过该程序的窗口。
    Hide,
    /// 隐藏后不冻结该程序的进程。
    Freeze,
    /// 隐藏后不静音该程序的进程。
    Mute,
}

/// 一条内置的强制忽略冻结项：ZoneDeck 自身的一个角色及其全部映像名。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinGuard {
    /// 稳定标识，供界面查对应的本地化角色名。
    pub key: &'static str,
    /// 该角色可能的映像名：生产名与开发构建名各一条。
    pub names: &'static [&'static str],
}

/// 永不冻结的自有进程；不可由用户关闭，由 [`is_ignored`] 内部兜底。
pub const BUILTIN_FREEZE_GUARDS: [BuiltinGuard; 2] = [
    BuiltinGuard {
        key: "core",
        names: &["ZoneDeck.exe", "core.exe"],
    },
    BuiltinGuard {
        key: "config",
        names: &["config.exe", "zonedeck-config.exe"],
    },
];

/// 映像名是否属于内置强制忽略冻结项；不区分大小写。
pub fn is_builtin_freeze_guarded(process: &str) -> bool {
    BUILTIN_FREEZE_GUARDS
        .iter()
        .flat_map(|g| g.names)
        .any(|n| n.eq_ignore_ascii_case(process))
}

/// 一条白名单条目是否命中该进程；`path` 为空时按路径匹配的条目一律不命中。
fn whitelist_rule_matches(rule: &WhitelistRule, path: &str, process: &str) -> bool {
    let subject = if rule.by_name { process } else { path };
    if subject.is_empty() {
        return false;
    }
    match &rule.regex {
        Some(pattern) => cached_regex(pattern).is_some_and(|re| re.is_match(subject)),
        None => {
            let want = if rule.by_name {
                &rule.process
            } else {
                &rule.path
            };
            !want.is_empty() && path_eq(want, subject)
        }
    }
}

/// 进程（由完整路径 + 映像名标识）是否被声明忽略该模式。
/// [`IgnoreMode::Freeze`] 先过 [`BUILTIN_FREEZE_GUARDS`]。
pub fn is_ignored(rules: &[WhitelistRule], path: &str, process: &str, mode: IgnoreMode) -> bool {
    if mode == IgnoreMode::Freeze && is_builtin_freeze_guarded(process) {
        return true;
    }
    rules
        .iter()
        .filter(|r| match mode {
            IgnoreMode::Hide => r.ignore_hide,
            IgnoreMode::Freeze => r.ignore_freeze,
            IgnoreMode::Mute => r.ignore_mute,
        })
        .any(|r| whitelist_rule_matches(r, path, process))
}

/// 白名单里是否存在按路径（含路径正则）声明忽略该模式的条目。
/// 调用方据此决定要不要逐 PID 查完整路径。
pub fn whitelist_needs_paths(rules: &[WhitelistRule], mode: IgnoreMode) -> bool {
    rules.iter().any(|r| {
        !r.by_name
            && match mode {
                IgnoreMode::Hide => r.ignore_hide,
                IgnoreMode::Freeze => r.ignore_freeze,
                IgnoreMode::Mute => r.ignore_mute,
            }
    })
}

/// 过宽判定用的样本条数。
pub const BREADTH_SAMPLES: usize = 200;
/// 命中数超过它即判定「可能过宽」。
pub const BREADTH_LIMIT: usize = BREADTH_SAMPLES / 2;

/// [`breadth_samples`] 的随机种子；定种保证同一条正则每次都得到同一结论。
const BREADTH_SEED: u64 = 0x5A17_C0DE_1234_9E77;

/// 样本用字符集：ASCII 字母数字、空格、常见标点与中日文字符混排。
const SAMPLE_ALPHABET: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', ' ', '-', '_', '.', ',', '(', ')', '[', ']', '—', '·', '文', '档',
    '窗', '口', '设', '置', '浏', '览', '器', '音', '乐', '视', '频', '聊', '天',
];

/// 路径形样本的目录段，凑出 `C:\Program Files\…\xxx.exe` 的形状。
const SAMPLE_DIRS: &[&str] = &[
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "C:\\Windows\\System32",
    "D:\\Games",
    "E:\\我的程序",
    "C:\\Users\\Public\\AppData\\Local",
];

/// xorshift64*，避免为生成样本引入 rand 依赖。
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// `[0, n)` 内的伪随机数；`n` 为 0 时返回 0。
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }

    /// 由 [`SAMPLE_ALPHABET`] 拼出的 `len` 字符随机串。
    fn word(&mut self, len: usize) -> String {
        (0..len)
            .map(|_| SAMPLE_ALPHABET[self.below(SAMPLE_ALPHABET.len())])
            .collect()
    }
}

/// [`BREADTH_SAMPLES`] 条伪随机样本串：约三成为 Windows 路径形状，其余是
/// 长度 1–40 的自由文本。种子固定，只生成一次。
static BREADTH_SAMPLES_CACHE: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut rng = Rng(BREADTH_SEED);
    (0..BREADTH_SAMPLES)
        .map(|i| {
            if i % 10 < 3 {
                let dir = SAMPLE_DIRS[rng.below(SAMPLE_DIRS.len())];
                let len = 3 + rng.below(9);
                format!("{dir}\\{}.exe", rng.word(len))
            } else {
                let len = 1 + rng.below(40);
                rng.word(len)
            }
        })
        .collect()
});

/// 见 [`BREADTH_SAMPLES_CACHE`]。
pub fn breadth_samples() -> &'static [String] {
    &BREADTH_SAMPLES_CACHE
}

/// 一条正则命中 [`breadth_samples`] 的条数；正则编译失败时返回 `None`。
pub fn regex_breadth(pattern: &str) -> Option<usize> {
    let re = cached_regex(pattern)?;
    Some(breadth_samples().iter().filter(|s| re.is_match(s)).count())
}

/// 正则是否「可能过宽」：命中随机样本超过 [`BREADTH_LIMIT`] 条。
pub fn regex_is_broad(pattern: &str) -> bool {
    regex_breadth(pattern).is_some_and(|n| n > BREADTH_LIMIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(title: &str, hwnd: i64, process: &str, pid: u32, path: &str) -> WindowInfo {
        WindowInfo::new(title, hwnd, process, pid, path)
    }

    #[test]
    fn window_rule_live_matches_by_hwnd() {
        let rule = WindowRule::from_window(&win("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe"));
        let windows = vec![
            win("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe"),
            win("记事本", 20, "notepad.exe", 200, "C:\\notepad.exe"),
        ];
        match resolve_window_rule(&rule, &windows) {
            WindowResolution::Live(w) => assert_eq!(w.hwnd, 10),
            other => panic!("应命中 Live，实际 {other:?}"),
        }
    }

    #[test]
    fn window_rule_hwnd_recycled_to_other_process_is_not_live() {
        let rule = WindowRule::from_window(&win("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe"));
        let windows = vec![win("别的窗口", 10, "other.exe", 200, "C:\\other.exe")];
        assert_eq!(
            resolve_window_rule(&rule, &windows),
            WindowResolution::Missing
        );
    }

    #[test]
    fn window_rule_reacquires_by_title_and_path_when_hwnd_changed() {
        let rule = WindowRule::from_window(&win("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe"));
        let windows = vec![win("微信", 99, "WeChat.exe", 300, "C:\\WeChat.exe")];
        match resolve_window_rule(&rule, &windows) {
            WindowResolution::Reacquired(w) => assert_eq!(w.hwnd, 99, "应回填新句柄"),
            other => panic!("应命中 Reacquired，实际 {other:?}"),
        }
    }

    #[test]
    fn window_rule_missing_when_closed() {
        let rule = WindowRule::from_window(&win("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe"));
        let windows = vec![win("记事本", 20, "notepad.exe", 200, "C:\\notepad.exe")];
        assert_eq!(
            resolve_window_rule(&rule, &windows),
            WindowResolution::Missing
        );
    }

    #[test]
    fn window_rule_no_title_does_not_reacquire_on_title() {
        let rule = WindowRule::from_window(&win(NO_TITLE, 10, "a.exe", 100, "C:\\a.exe"));
        let windows = vec![win(NO_TITLE, 77, "a.exe", 200, "C:\\a.exe")];
        assert_eq!(
            resolve_window_rule(&rule, &windows),
            WindowResolution::Missing
        );
    }

    #[test]
    fn window_regex_matches_multiple_titles() {
        let rule = WindowRule::from_regex("^项目.*");
        let windows = vec![
            win("项目 A - 编辑器", 1, "code.exe", 1, "C:\\code.exe"),
            win("项目 B - 编辑器", 2, "code.exe", 2, "C:\\code.exe"),
            win("音乐", 3, "music.exe", 3, "C:\\music.exe"),
        ];
        match resolve_window_rule(&rule, &windows) {
            WindowResolution::Regex(hits) => {
                assert_eq!(hits.len(), 2, "应命中两个以「项目」开头的窗口");
            }
            other => panic!("应命中 Regex，实际 {other:?}"),
        }
    }

    #[test]
    fn window_regex_invalid_pattern_matches_nothing() {
        let rule = WindowRule::from_regex("(");
        let windows = vec![win("任意", 1, "a.exe", 1, "C:\\a.exe")];
        assert_eq!(
            resolve_window_rule(&rule, &windows),
            WindowResolution::Regex(Vec::new())
        );
    }

    #[test]
    fn process_rule_exact_hides_all_windows_of_path() {
        let rule = ProcessRule::from_window(&win("窗口一", 1, "game.exe", 1, "C:\\game.exe"));
        let windows = vec![
            win("窗口一", 1, "game.exe", 1, "C:\\game.exe"),
            win("窗口二", 2, "game.exe", 2, "C:\\game.exe"),
            win("记事本", 3, "notepad.exe", 3, "C:\\notepad.exe"),
        ];
        let hits = match_process_rule(&rule, &windows);
        assert_eq!(hits.len(), 2, "应命中同一 exe 的两个窗口");
    }

    #[test]
    fn process_rule_empty_path_matches_nothing() {
        let orphan = win("无路径窗口", 1, "", 1, "");
        let rule = ProcessRule::from_window(&orphan);
        let windows = vec![orphan.clone()];
        assert!(
            match_process_rule(&rule, &windows).is_empty(),
            "空路径规则不应命中任何窗口"
        );
    }

    #[test]
    fn process_rule_exact_match_ignores_case() {
        let rule = ProcessRule::from_window(&win("窗口", 1, "Game.exe", 1, "C:\\Games\\Game.exe"));
        let windows = vec![win("窗口", 1, "GAME.EXE", 1, "c:\\games\\GAME.EXE")];
        assert_eq!(
            match_process_rule(&rule, &windows).len(),
            1,
            "盘符与文件名的大小写差异不应让规则失配"
        );
    }

    #[test]
    fn process_rule_by_name_match_ignores_case() {
        let mut rule = ProcessRule::from_window(&win("窗口", 1, "Game.exe", 1, "C:\\Game.exe"));
        rule.by_name = true;
        let windows = vec![win("窗口", 1, "GAME.EXE", 1, "D:\\别处\\GAME.EXE")];
        assert_eq!(match_process_rule(&rule, &windows).len(), 1);
    }

    #[test]
    fn window_rule_live_check_ignores_path_case() {
        let rule = WindowRule::from_window(&win("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe"));
        let windows = vec![win("微信", 10, "WECHAT.EXE", 100, "c:\\WECHAT.EXE")];
        match resolve_window_rule(&rule, &windows) {
            WindowResolution::Live(w) => assert_eq!(w.hwnd, 10),
            other => panic!("大小写差异不应让句柄校验失败，实际 {other:?}"),
        }
    }

    #[test]
    fn window_rule_reacquire_ignores_path_case() {
        let rule = WindowRule::from_window(&win("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe"));
        let windows = vec![win("微信", 99, "WECHAT.EXE", 300, "c:\\WECHAT.EXE")];
        match resolve_window_rule(&rule, &windows) {
            WindowResolution::Reacquired(w) => assert_eq!(w.hwnd, 99),
            other => panic!("大小写差异不应让追溯失败，实际 {other:?}"),
        }
    }

    #[test]
    fn window_rule_reacquire_by_process_name_ignores_case() {
        let mut rule =
            WindowRule::from_window(&win("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe"));
        rule.path = String::new();
        let windows = vec![win("微信", 99, "WECHAT.EXE", 300, "C:\\WeChat.exe")];
        assert!(
            matches!(
                resolve_window_rule(&rule, &windows),
                WindowResolution::Reacquired(_)
            ),
            "进程名的大小写差异不应让追溯失败"
        );
    }

    #[test]
    fn regex_results_are_stable_across_repeated_calls() {
        let mut rule = ProcessRule::from_window(&win("窗口", 1, "a.exe", 1, "C:\\a.exe"));
        rule.regex = Some("^C:\\\\a\\.exe$".to_string());
        let windows = vec![
            win("窗口", 1, "a.exe", 1, "C:\\a.exe"),
            win("别的", 2, "b.exe", 2, "C:\\b.exe"),
        ];
        let first = match_process_rule(&rule, &windows).len();
        assert_eq!(first, 1);
        for _ in 0..3 {
            assert_eq!(
                match_process_rule(&rule, &windows).len(),
                first,
                "缓存命中不得改变匹配结果"
            );
        }
    }

    #[test]
    fn invalid_regex_stays_invalid_across_calls() {
        assert!(!regex_is_valid("(unclosed"));
        assert!(!regex_is_valid("(unclosed"));
        assert_eq!(regex_breadth("(unclosed"), None);
    }

    #[test]
    fn scope_excludes_background_and_untitled_by_default() {
        let visible_titled = win("有标题", 1, "a.exe", 1, "C:\\a.exe");
        let untitled = win(NO_TITLE, 2, "a.exe", 1, "C:\\a.exe");
        let background = win("后台", 3, "a.exe", 1, "C:\\a.exe").with_visibility(false);

        assert!(in_scope(&visible_titled, false, false));
        assert!(!in_scope(&untitled, false, false), "默认应排除无标题窗口");
        assert!(!in_scope(&background, false, false), "默认应排除后台窗口");
        assert!(in_scope(&untitled, true, false), "放开后应纳入无标题窗口");
        assert!(in_scope(&background, false, true), "放开后应纳入后台窗口");
    }

    #[test]
    fn window_regex_respects_scope() {
        let windows = vec![
            win("项目 A", 1, "code.exe", 1, "C:\\code.exe"),
            win("项目 B", 2, "code.exe", 2, "C:\\code.exe").with_visibility(false),
        ];
        let mut rule = WindowRule::from_regex("^项目");
        assert_eq!(
            resolve_window_rule(&rule, &windows),
            WindowResolution::Regex(vec![&windows[0]]),
            "默认只匹配可见窗口"
        );

        rule.include_background = true;
        match resolve_window_rule(&rule, &windows) {
            WindowResolution::Regex(hits) => assert_eq!(hits.len(), 2, "放开后台后应命中两个"),
            other => panic!("应为 Regex，实际 {other:?}"),
        }
    }

    #[test]
    fn process_rule_includes_untitled_windows_by_default() {
        let windows = vec![
            win("主窗口", 1, "game.exe", 1, "C:\\game.exe"),
            win(NO_TITLE, 2, "game.exe", 1, "C:\\game.exe"),
        ];
        let rule = ProcessRule::from_window(&windows[0]);
        assert_eq!(match_process_rule(&rule, &windows).len(), 2);
    }

    #[test]
    fn process_rule_excludes_background_windows_by_default() {
        let windows = vec![
            win("主窗口", 1, "game.exe", 1, "C:\\game.exe"),
            win("隐着的", 2, "game.exe", 1, "C:\\game.exe").with_visibility(false),
        ];
        let rule = ProcessRule::from_window(&windows[0]);
        let hits = match_process_rule(&rule, &windows);
        assert_eq!(hits.len(), 1, "默认不纳入后台窗口，避免恢复时误显示");
        assert_eq!(hits[0].hwnd, 1);
    }

    #[test]
    fn process_rule_by_name_ignores_install_path() {
        let windows = vec![
            win(
                "微信 A",
                1,
                "WeChat.exe",
                1,
                "C:\\Program Files\\WeChat.exe",
            ),
            win("微信 B", 2, "WeChat.exe", 2, "D:\\另一个位置\\WeChat.exe"),
            win("记事本", 3, "notepad.exe", 3, "C:\\notepad.exe"),
        ];
        let mut rule = ProcessRule::from_window(&windows[0]);
        assert_eq!(
            match_process_rule(&rule, &windows).len(),
            1,
            "按路径匹配只命中同一路径"
        );

        rule.by_name = true;
        let hits = match_process_rule(&rule, &windows);
        assert_eq!(hits.len(), 2, "按文件名匹配应命中不同目录下的同名程序");
    }

    #[test]
    fn process_regex_by_name_matches_file_name_only() {
        let windows = vec![
            win("微信", 1, "WeChat.exe", 1, "D:\\Program Files\\WeChat.exe"),
            win("记事本", 2, "notepad.exe", 2, "C:\\notepad.exe"),
        ];
        let mut rule = ProcessRule::from_regex("(?i)^wechat");
        rule.by_name = true;
        let hits = match_process_rule(&rule, &windows);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].process, "WeChat.exe");
    }

    #[test]
    fn process_regex_matches_by_path() {
        let rule = ProcessRule::from_regex(r"(?i)\\wechat\.exe$");
        let windows = vec![
            win("微信", 1, "WeChat.exe", 1, "D:\\Program Files\\WeChat.exe"),
            win("记事本", 2, "notepad.exe", 2, "C:\\notepad.exe"),
        ];
        let hits = match_process_rule(&rule, &windows);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].process, "WeChat.exe");
    }

    #[test]
    fn regex_validity_helper() {
        assert!(regex_is_valid("^foo.*$"));
        assert!(!regex_is_valid("("));
    }

    /// 便捷构造：一条按文件名匹配、只开指定模式的白名单条目。
    fn allow(process: &str, mode: IgnoreMode) -> WhitelistRule {
        let w = win("", 0, process, 0, &format!("C:\\{process}"));
        let mut r = WhitelistRule::from_window(&w);
        match mode {
            IgnoreMode::Hide => r.ignore_hide = true,
            IgnoreMode::Freeze => r.ignore_freeze = true,
            IgnoreMode::Mute => r.ignore_mute = true,
        }
        r
    }

    #[test]
    fn ignore_modes_are_independent() {
        let rules = [allow("explorer.exe", IgnoreMode::Hide)];
        let (path, name) = ("C:\\Windows\\explorer.exe", "explorer.exe");
        assert!(is_ignored(&rules, path, name, IgnoreMode::Hide));
        assert!(
            !is_ignored(&rules, path, name, IgnoreMode::Mute),
            "只勾了忽略隐藏，不应连带忽略静音"
        );
        assert!(
            !is_ignored(&rules, path, name, IgnoreMode::Freeze),
            "只勾了忽略隐藏，不应连带忽略冻结"
        );
    }

    #[test]
    fn ignore_by_name_is_case_insensitive_and_path_agnostic() {
        let rules = [allow("Explorer.EXE", IgnoreMode::Hide)];
        assert!(
            is_ignored(
                &rules,
                "D:\\另一个位置\\explorer.exe",
                "explorer.exe",
                IgnoreMode::Hide
            ),
            "Windows 文件名大小写不敏感，且按文件名匹配应忽略安装目录"
        );
    }

    #[test]
    fn ignore_by_path_needs_the_exact_location() {
        let mut rule = allow("explorer.exe", IgnoreMode::Hide);
        rule.by_name = false;
        rule.path = "C:\\Windows\\explorer.exe".to_string();
        let rules = [rule];
        assert!(is_ignored(
            &rules,
            "c:\\windows\\explorer.exe",
            "explorer.exe",
            IgnoreMode::Hide
        ));
        assert!(
            !is_ignored(
                &rules,
                "D:\\别处\\explorer.exe",
                "explorer.exe",
                IgnoreMode::Hide
            ),
            "按路径匹配不应命中别处的同名程序"
        );
        assert!(
            !is_ignored(&rules, "", "explorer.exe", IgnoreMode::Hide),
            "查不到路径时按路径匹配的条目不命中"
        );
    }

    #[test]
    fn ignore_regex_matches_chosen_subject() {
        let mut by_name = WhitelistRule::from_regex("(?i)^wechat");
        by_name.ignore_mute = true;
        assert!(is_ignored(
            &[by_name],
            "D:\\Program Files\\WeChat.exe",
            "WeChat.exe",
            IgnoreMode::Mute
        ));

        let mut by_path = WhitelistRule::from_regex(r"(?i)^c:\\windows\\");
        by_path.by_name = false;
        by_path.ignore_hide = true;
        let rules = [by_path];
        assert!(is_ignored(
            &rules,
            "C:\\Windows\\explorer.exe",
            "explorer.exe",
            IgnoreMode::Hide
        ));
        assert!(!is_ignored(
            &rules,
            "D:\\Games\\a.exe",
            "a.exe",
            IgnoreMode::Hide
        ));
    }

    #[test]
    fn invalid_ignore_regex_matches_nothing() {
        let mut rule = WhitelistRule::from_regex("(");
        rule.ignore_hide = true;
        assert!(
            !is_ignored(&[rule], "C:\\a.exe", "a.exe", IgnoreMode::Hide),
            "写坏的正则不应意外放行一切"
        );
    }

    /// 这条保护不依赖任何用户配置。
    #[test]
    fn builtin_guards_block_freezing_ourselves_with_empty_whitelist() {
        for name in [
            "ZoneDeck.exe",
            "config.exe",
            "core.exe",
            "zonedeck-config.exe",
        ] {
            assert!(
                is_ignored(&[], "D:\\安装目录\\", name, IgnoreMode::Freeze),
                "{name} 必须恒被排除在冻结之外"
            );
            assert!(is_builtin_freeze_guarded(&name.to_ascii_uppercase()));
        }
        assert!(!is_builtin_freeze_guarded("explorer.exe"));
    }

    /// 内置保护只挡冻结。
    #[test]
    fn builtin_guards_do_not_leak_into_other_modes() {
        assert!(!is_ignored(
            &[],
            "C:\\a\\ZoneDeck.exe",
            "ZoneDeck.exe",
            IgnoreMode::Hide
        ));
        assert!(!is_ignored(
            &[],
            "C:\\a\\ZoneDeck.exe",
            "ZoneDeck.exe",
            IgnoreMode::Mute
        ));
    }

    #[test]
    fn needs_paths_only_when_a_relevant_rule_matches_by_path() {
        let by_name = [allow("a.exe", IgnoreMode::Freeze)];
        assert!(!whitelist_needs_paths(&by_name, IgnoreMode::Freeze));

        let mut by_path = allow("a.exe", IgnoreMode::Freeze);
        by_path.by_name = false;
        assert!(whitelist_needs_paths(
            &[by_path.clone()],
            IgnoreMode::Freeze
        ));
        assert!(
            !whitelist_needs_paths(&[by_path], IgnoreMode::Hide),
            "该条目没勾忽略隐藏，隐藏模式无需查路径"
        );
    }

    #[test]
    fn breadth_samples_are_deterministic_and_well_shaped() {
        let a = breadth_samples();
        assert_eq!(a.len(), BREADTH_SAMPLES);
        assert_eq!(a, breadth_samples(), "定种：两次生成必须完全一致");
        assert!(a.iter().all(|s| !s.is_empty()), "样本不得为空串");

        let unique: std::collections::HashSet<&String> = a.iter().collect();
        assert!(
            unique.len() > BREADTH_SAMPLES * 9 / 10,
            "样本应基本互不相同"
        );
        let paths = a.iter().filter(|s| s.contains('\\')).count();
        assert!(
            (40..=80).contains(&paths),
            "约三成样本应为路径形状，实际 {paths}"
        );
    }

    #[test]
    fn broad_patterns_are_flagged_and_specific_ones_are_not() {
        for broad in [".*", "", "^", ".", "(?s).*", "[\\s\\S]*"] {
            assert!(
                regex_is_broad(broad),
                "{broad:?} 能命中几乎一切，应判为过宽"
            );
        }
        for narrow in [
            ".*微信.*",
            "(?i)^wechat",
            r"(?i)\\chrome\.exe$",
            "^项目.*",
            r".*\.exe$",
        ] {
            assert!(
                !regex_is_broad(narrow),
                "{narrow:?} 只命中特定目标，不应判为过宽"
            );
        }
    }

    #[test]
    fn breadth_reports_counts_and_rejects_invalid_patterns() {
        assert_eq!(regex_breadth(".*"), Some(BREADTH_SAMPLES), "命中全部样本");
        assert_eq!(regex_breadth("(").as_ref(), None, "写坏的正则无从判定");
        assert!(!regex_is_broad("("), "无法编译时不判过宽");
        assert_eq!(
            regex_breadth("这段文字不可能出现在样本里"),
            Some(0),
            "命中不到任何样本"
        );
        assert_eq!(BREADTH_LIMIT, 100, "阈值为样本数的一半");
    }
}
