//! 窗口 / 进程规则匹配引擎。
//!
//! 窗口规则（细）按句柄 + 标题锁定单个窗口，句柄失效时按「标题 + 进程路径」追溯；
//! 进程规则（粗）按可执行文件路径隐藏该程序的所有窗口。均为纯函数。

use regex::Regex;

use crate::NO_TITLE;
use crate::model::{ProcessRule, WindowInfo, WindowRule};

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

/// 校验正则是否可编译（供核心侧对非法正则记录 WARN，界面侧给出反馈）。
pub fn regex_is_valid(pattern: &str) -> bool {
    Regex::new(pattern).is_ok()
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
        w.path == rule.path
    } else {
        !rule.process.is_empty() && w.process == rule.process
    }
}

/// 解析一条窗口规则命中的存活窗口。见 [`WindowResolution`]。
pub fn resolve_window_rule<'a>(
    rule: &WindowRule,
    windows: &'a [WindowInfo],
) -> WindowResolution<'a> {
    if let Some(pattern) = &rule.regex {
        let Ok(re) = Regex::new(pattern) else {
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
        && (rule.path.is_empty() || w.path == rule.path)
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
        Some(pattern) => match Regex::new(pattern) {
            Ok(re) => candidates.filter(|w| re.is_match(&subject(w))).collect(),
            Err(_) => Vec::new(),
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
            candidates.filter(|w| &subject(w) == want).collect()
        }
    }
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
}
