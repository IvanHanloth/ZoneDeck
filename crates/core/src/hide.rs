use std::collections::HashSet;
use std::time::Duration;

use bosskey_common::matching::{WindowResolution, match_process_rule, resolve_window_rule};
use bosskey_common::{Config, Setting, WindowInfo};
use serde::{Deserialize, Serialize};

use crate::effects::Effects;
use crate::platform::WindowManager;
use crate::recovery::Snapshot;

const SEND_PAUSE_DELAY: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub hwnd: i64,
    pub pid: u32,
}

/// 一条窗口规则的解析结果摘要。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleOutcome {
    Live,
    Reacquired,
    Missing,
    Regex(usize),
}

/// 依据窗口/进程规则解析出要隐藏的目标句柄集合，并对精确规则做追溯回填。
/// 返回值 `outcomes` 与 `config.window_rules` 一一对应。
pub fn resolve_targets(
    config: &mut Config,
    windows: &[WindowInfo],
    foreground: i64,
) -> (Vec<Target>, Vec<RuleOutcome>) {
    let mut result: Vec<Target> = Vec::new();
    let mut outcomes: Vec<RuleOutcome> = Vec::with_capacity(config.window_rules.len());

    for rule in &mut config.window_rules {
        let outcome = match resolve_window_rule(rule, windows) {
            WindowResolution::Live(w) => {
                rule.title = w.title.clone();
                result.push(Target {
                    hwnd: w.hwnd,
                    pid: w.pid,
                });
                RuleOutcome::Live
            }
            WindowResolution::Reacquired(w) => {
                rule.hwnd = w.hwnd;
                rule.pid = w.pid;
                rule.title = w.title.clone();
                result.push(Target {
                    hwnd: w.hwnd,
                    pid: w.pid,
                });
                RuleOutcome::Reacquired
            }
            WindowResolution::Missing => RuleOutcome::Missing,
            WindowResolution::Regex(hits) => {
                for w in &hits {
                    result.push(Target {
                        hwnd: w.hwnd,
                        pid: w.pid,
                    });
                }
                RuleOutcome::Regex(hits.len())
            }
        };
        outcomes.push(outcome);
    }

    for rule in &config.process_rules {
        for w in match_process_rule(rule, windows) {
            result.push(Target {
                hwnd: w.hwnd,
                pid: w.pid,
            });
        }
    }

    if config.setting.hide_current && foreground != 0 {
        let pid = windows
            .iter()
            .find(|w| w.hwnd == foreground)
            .map(|w| w.pid)
            .unwrap_or(0);
        result.push(Target {
            hwnd: foreground,
            pid,
        });
    }

    let mut seen = HashSet::new();
    result.retain(|t| seen.insert(t.hwnd));
    (result, outcomes)
}

/// 可以安全冻结的进程 PID 集合：仅当某进程的全部可见窗口都在隐藏目标里时才纳入。
pub fn freezable_pids(targets: &[Target], windows: &[WindowInfo]) -> Vec<u32> {
    let hidden: HashSet<i64> = targets.iter().map(|t| t.hwnd).collect();
    let mut pids: Vec<u32> = targets
        .iter()
        .map(|t| t.pid)
        .filter(|pid| *pid != 0)
        .collect();
    pids.sort_unstable();
    pids.dedup();

    pids.retain(|pid| {
        let mut visible = windows
            .iter()
            .filter(|w| w.pid == *pid && w.visible)
            .peekable();
        // 没有可见窗口时不冻结。
        visible.peek().is_some() && visible.all(|w| hidden.contains(&w.hwnd))
    });
    pids
}

/// 把一组根 PID 展开为「根 ∪ 全部后代」：按 `edges`（`(pid, 父 pid)`）建父子关系，
/// 从每个根 BFS 收集整棵子进程树。用 visited 去重并防环 / 自指（如 pid 0 的父是 0）。
/// 返回排序去重后的 PID 列表。
pub fn expand_descendants(roots: &[u32], edges: &[(u32, u32)]) -> Vec<u32> {
    use std::collections::VecDeque;

    let mut children: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for &(pid, ppid) in edges {
        // 忽略自指边，避免把自己当成自己的子进程。
        if pid != ppid {
            children.entry(ppid).or_default().push(pid);
        }
    }

    let mut visited: HashSet<u32> = HashSet::new();
    let mut queue: VecDeque<u32> = VecDeque::new();
    for &r in roots {
        if r != 0 && visited.insert(r) {
            queue.push_back(r);
        }
    }
    while let Some(pid) = queue.pop_front() {
        if let Some(kids) = children.get(&pid) {
            for &kid in kids {
                if kid != 0 && visited.insert(kid) {
                    queue.push_back(kid);
                }
            }
        }
    }

    let mut out: Vec<u32> = visited.into_iter().collect();
    out.sort_unstable();
    out
}

pub struct HideController<W: WindowManager, E: Effects> {
    wm: W,
    effects: E,
    hidden: Vec<Target>,
    frozen: Vec<u32>,
    muted: Vec<u32>,
    used_enhanced: bool,
}

impl<W: WindowManager, E: Effects> HideController<W, E> {
    pub fn new(wm: W, effects: E) -> Self {
        Self {
            wm,
            effects,
            hidden: Vec::new(),
            frozen: Vec::new(),
            muted: Vec::new(),
            used_enhanced: false,
        }
    }

    pub fn is_hidden(&self) -> bool {
        !self.hidden.is_empty()
    }

    /// 当前被隐藏的窗口数（供日志如实记录恢复了多少个窗口）。
    pub fn hidden_count(&self) -> usize {
        self.hidden.len()
    }

    /// 枚举当前窗口（供上层解析目标；封装 wm 依赖）。
    pub fn enumerate(&self) -> Vec<WindowInfo> {
        self.wm.enumerate()
    }

    pub fn foreground(&self) -> i64 {
        self.wm.foreground()
    }

    /// 隐藏已解析出的目标：隐藏窗口并按设置应用静音 / 冻结 / 发送暂停键。
    /// `freeze_pids` 为最终要冻结的 PID 集（调用方算好，可能已含递归展开的子进程树，
    /// 见 [`freezable_pids`] 与 [`expand_descendants`]）；仅在 `freeze_after_hide` 开启时生效。
    pub fn apply_hide(&mut self, setting: &Setting, targets: &[Target], freeze_pids: &[u32]) {
        // 隐藏前先发一次媒体「播放/暂停」键，暂停正在播放的音视频。
        // 只发一次：多目标时重复发送会把「播放/暂停」这个切换键又切回播放。
        // 仅当确有音视频在播放时才真正发键（由 effects 判断），此时才需等待其生效。
        if setting.send_before_hide && !targets.is_empty() && self.effects.send_pause() {
            std::thread::sleep(SEND_PAUSE_DELAY);
        }

        let mut muted = Vec::new();

        for t in targets {
            self.wm.hide(t.hwnd);
            if setting.mute_after_hide && t.pid != 0 {
                self.effects.mute(t.pid, true);
                muted.push(t.pid);
            }
        }
        muted.sort_unstable();
        muted.dedup();

        // 冻结与目标窗口解耦：freeze_pids 已是最终集合（含子进程树），逐个挂起。
        let mut frozen = Vec::new();
        if setting.freeze_after_hide {
            frozen = freeze_pids.to_vec();
            frozen.retain(|pid| *pid != 0);
            // 挂起是计数式的，去重以匹配解冻次数。
            frozen.sort_unstable();
            frozen.dedup();
            self.used_enhanced = setting.enhanced_freeze;
            for pid in &frozen {
                self.effects.suspend(*pid, self.used_enhanced);
            }
        }

        self.frozen = frozen;
        self.muted = muted;
        self.hidden = targets.to_vec();
    }

    pub fn show(&mut self) {
        for pid in &self.frozen {
            self.effects.resume(*pid, self.used_enhanced);
        }
        self.frozen.clear();

        for t in &self.hidden {
            self.wm.show(t.hwnd);
        }

        for pid in &self.muted {
            self.effects.mute(*pid, false);
        }
        self.muted.clear();
        self.hidden.clear();
    }

    /// 释放指定进程的隐藏状态：显示窗口、解冻、取消静音，并从隐藏集移除。返回被释放的窗口数。
    pub fn release_pids(&mut self, pids: &[u32]) -> usize {
        if pids.is_empty() {
            return 0;
        }

        // 先解冻，否则窗口显示出来仍是卡死的。
        let (thaw, keep): (Vec<u32>, Vec<u32>) = self
            .frozen
            .iter()
            .copied()
            .partition(|pid| pids.contains(pid));
        for pid in &thaw {
            self.effects.resume(*pid, self.used_enhanced);
        }
        self.frozen = keep;

        let (show, keep): (Vec<Target>, Vec<Target>) = self
            .hidden
            .iter()
            .copied()
            .partition(|t| pids.contains(&t.pid));
        for t in &show {
            self.wm.show(t.hwnd);
        }
        self.hidden = keep;

        let (unmute, keep): (Vec<u32>, Vec<u32>) = self
            .muted
            .iter()
            .copied()
            .partition(|pid| pids.contains(pid));
        for pid in &unmute {
            self.effects.mute(*pid, false);
        }
        self.muted = keep;

        show.len()
    }

    /// 当前隐藏状态的快照，用于崩溃恢复落盘。
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            hidden: self.hidden.clone(),
            frozen: self.frozen.clone(),
            muted: self.muted.clone(),
            enhanced: self.used_enhanced,
        }
    }

    /// 从崩溃前的快照恢复：显示窗口、解冻进程、取消静音。
    pub fn restore_from(&mut self, snapshot: Snapshot) {
        self.hidden = snapshot.hidden;
        self.frozen = snapshot.frozen;
        self.muted = snapshot.muted;
        self.used_enhanced = snapshot.enhanced;
        self.show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    use bosskey_common::{ProcessRule, WindowRule};

    fn win(title: &str, hwnd: i64, process: &str, path: &str) -> WindowInfo {
        WindowInfo::new(title, hwnd, process, hwnd as u32, path)
    }

    fn wrule(title: &str, hwnd: i64, process: &str, path: &str) -> WindowRule {
        WindowRule::from_window(&win(title, hwnd, process, path))
    }

    /// 复刻 agent 的隐藏编排：解析目标（含追溯回填）后交给控制器应用副作用。
    fn do_hide<W: WindowManager, E: Effects>(
        controller: &mut HideController<W, E>,
        config: &mut Config,
    ) {
        let windows = controller.enumerate();
        let foreground = controller.foreground();
        let (targets, _) = resolve_targets(config, &windows, foreground);
        let freezable = freezable_pids(&targets, &windows);
        controller.apply_hide(&config.setting, &targets, &freezable);
    }

    #[test]
    fn resolves_window_rule_to_its_target() {
        let mut config = Config {
            window_rules: vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")],
            ..Default::default()
        };
        let windows = vec![
            win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
            win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
        ];
        let (targets, outcomes) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(targets, vec![Target { hwnd: 10, pid: 10 }]);
        assert_eq!(outcomes, vec![RuleOutcome::Live]);
    }

    #[test]
    fn window_rule_syncs_title_on_hide() {
        let mut config = Config {
            window_rules: vec![wrule("旧标题", 10, "app.exe", "C:\\app.exe")],
            ..Default::default()
        };
        let windows = vec![win("新标题", 10, "app.exe", "C:\\app.exe")];
        let (targets, _) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(targets, vec![Target { hwnd: 10, pid: 10 }]);
        assert_eq!(
            config.window_rules[0].title, "新标题",
            "隐藏时应同步最新标题"
        );
    }

    #[test]
    fn window_rule_reacquires_and_backfills_hwnd() {
        let mut config = Config {
            window_rules: vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")],
            ..Default::default()
        };
        let windows = vec![win("微信", 99, "WeChat.exe", "C:\\WeChat.exe")];
        let (targets, outcomes) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(targets, vec![Target { hwnd: 99, pid: 99 }]);
        assert_eq!(outcomes, vec![RuleOutcome::Reacquired]);
        assert_eq!(config.window_rules[0].hwnd, 99, "应回填新句柄");
    }

    #[test]
    fn window_rule_missing_when_window_gone() {
        let mut config = Config {
            window_rules: vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")],
            ..Default::default()
        };
        let windows = vec![win("记事本", 20, "notepad.exe", "C:\\notepad.exe")];
        let (targets, outcomes) = resolve_targets(&mut config, &windows, 0);
        assert!(targets.is_empty());
        assert_eq!(outcomes, vec![RuleOutcome::Missing]);
    }

    #[test]
    fn process_rule_hides_all_windows_of_same_executable() {
        let mut config = Config {
            process_rules: vec![ProcessRule::from_window(&win(
                "窗口一",
                10,
                "game.exe",
                "C:\\game.exe",
            ))],
            ..Default::default()
        };
        let windows = vec![
            win("窗口一", 10, "game.exe", "C:\\game.exe"),
            win("窗口二", 11, "game.exe", "C:\\game.exe"),
            win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
        ];
        let (targets, _) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(
            targets,
            vec![Target { hwnd: 10, pid: 10 }, Target { hwnd: 11, pid: 11 }]
        );
    }

    /// 与 `win` 相同，但可指定 PID（用于构造同一进程的多个窗口）。
    fn win_pid(title: &str, hwnd: i64, process: &str, pid: u32, path: &str) -> WindowInfo {
        WindowInfo::new(title, hwnd, process, pid, path)
    }

    #[test]
    fn freeze_skipped_when_process_still_has_a_visible_window() {
        let windows = vec![
            win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
            win_pid("文件传输助手", 11, "WeChat.exe", 500, "C:\\WeChat.exe"),
        ];
        let targets = vec![Target { hwnd: 10, pid: 500 }];
        assert!(
            freezable_pids(&targets, &windows).is_empty(),
            "还有窗口开着时不应冻结"
        );
    }

    #[test]
    fn freeze_allowed_when_all_visible_windows_hidden() {
        let windows = vec![
            win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
            win_pid("文件传输助手", 11, "WeChat.exe", 500, "C:\\WeChat.exe"),
        ];
        let targets = vec![Target { hwnd: 10, pid: 500 }, Target { hwnd: 11, pid: 500 }];
        assert_eq!(freezable_pids(&targets, &windows), vec![500]);
    }

    #[test]
    fn freeze_ignores_already_invisible_windows() {
        let windows = vec![
            win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
            win_pid("后台窗口", 11, "WeChat.exe", 500, "C:\\WeChat.exe").with_visibility(false),
        ];
        let targets = vec![Target { hwnd: 10, pid: 500 }];
        assert_eq!(freezable_pids(&targets, &windows), vec![500]);
    }

    #[test]
    fn hiding_one_window_of_a_multi_window_app_does_not_freeze_it() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.freeze_after_hide = true;
        config.window_rules = vec![WindowRule::from_window(&win_pid(
            "微信",
            10,
            "WeChat.exe",
            500,
            "C:\\WeChat.exe",
        ))];

        let wm = MockWm::new(
            vec![
                win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
                win_pid("文件传输助手", 11, "WeChat.exe", 500, "C:\\WeChat.exe"),
            ],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        do_hide(&mut controller, &mut config);

        assert!(!controller.wm.is_visible(10), "目标窗口应被隐藏");
        assert!(controller.wm.is_visible(11), "同进程的另一个窗口应保持可见");
        assert!(
            controller.effects.suspends.borrow().is_empty(),
            "同进程还有窗口开着，不应冻结"
        );
    }

    #[test]
    fn release_pids_restores_hidden_and_frozen_windows_of_a_process() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.freeze_after_hide = true;
        config.setting.mute_after_hide = true;
        config.process_rules = vec![ProcessRule::from_window(&win_pid(
            "Boss Key 设置",
            10,
            "Boss Key.exe",
            700,
            "C:\\Boss Key.exe",
        ))];

        let wm = MockWm::new(
            vec![win_pid(
                "Boss Key 设置",
                10,
                "Boss Key.exe",
                700,
                "C:\\Boss Key.exe",
            )],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        do_hide(&mut controller, &mut config);
        assert!(!controller.wm.is_visible(10), "配置窗口已被隐藏");
        assert_eq!(*controller.effects.suspends.borrow(), vec![700], "已被冻结");

        assert_eq!(controller.release_pids(&[700]), 1);
        assert!(controller.wm.is_visible(10), "配置窗口应被放出来");
        assert_eq!(*controller.effects.resumes.borrow(), vec![700], "应先解冻");
        assert!(!controller.is_hidden(), "释放后已无隐藏内容");
    }

    #[test]
    fn hide_current_appends_foreground_and_dedups() {
        let mut config = Config::default();
        config.setting.hide_current = true;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];
        let windows = vec![
            win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
            win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
        ];
        let (targets, _) = resolve_targets(&mut config.clone(), &windows, 20);
        assert_eq!(
            targets,
            vec![Target { hwnd: 10, pid: 10 }, Target { hwnd: 20, pid: 20 }]
        );

        let (same, _) = resolve_targets(&mut config, &windows, 10);
        assert_eq!(
            same,
            vec![Target { hwnd: 10, pid: 10 }],
            "前台与已命中窗口相同应去重"
        );
    }

    struct MockWm {
        windows: Vec<WindowInfo>,
        foreground: i64,
        visible: RefCell<HashSet<i64>>,
    }

    impl MockWm {
        fn new(windows: Vec<WindowInfo>, foreground: i64) -> Self {
            let visible = windows.iter().map(|w| w.hwnd).collect();
            Self {
                windows,
                foreground,
                visible: RefCell::new(visible),
            }
        }
    }

    impl WindowManager for MockWm {
        fn enumerate(&self) -> Vec<WindowInfo> {
            let visible = self.visible.borrow();
            self.windows
                .iter()
                .filter(|w| visible.contains(&w.hwnd))
                .cloned()
                .collect()
        }
        fn hide(&self, hwnd: i64) {
            self.visible.borrow_mut().remove(&hwnd);
        }
        fn show(&self, hwnd: i64) {
            self.visible.borrow_mut().insert(hwnd);
        }
        fn is_visible(&self, hwnd: i64) -> bool {
            self.visible.borrow().contains(&hwnd)
        }
        fn foreground(&self) -> i64 {
            self.foreground
        }
    }

    #[derive(Default)]
    struct MockEffects {
        mutes: RefCell<Vec<(u32, bool)>>,
        suspends: RefCell<Vec<u32>>,
        resumes: RefCell<Vec<u32>>,
        pauses: RefCell<u32>,
    }

    impl Effects for MockEffects {
        fn mute(&self, pid: u32, mute: bool) {
            self.mutes.borrow_mut().push((pid, mute));
        }
        fn suspend(&self, pid: u32, _enhanced: bool) {
            self.suspends.borrow_mut().push(pid);
        }
        fn resume(&self, pid: u32, _enhanced: bool) {
            self.resumes.borrow_mut().push(pid);
        }
        fn send_pause(&self) -> bool {
            *self.pauses.borrow_mut() += 1;
            true
        }
    }

    #[test]
    fn toggle_applies_mute_and_freeze_then_restores() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = true;
        config.setting.send_before_hide = true;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(
            vec![
                win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
                win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
            ],
            10,
        );
        let mut controller = HideController::new(wm, MockEffects::default());

        do_hide(&mut controller, &mut config);
        assert!(controller.is_hidden());
        assert!(!controller.wm.is_visible(10), "微信应被隐藏");
        assert!(controller.wm.is_visible(20), "记事本不在绑定内应保持可见");
        assert_eq!(*controller.effects.mutes.borrow(), vec![(10, true)]);
        assert_eq!(*controller.effects.suspends.borrow(), vec![10]);
        assert_eq!(*controller.effects.pauses.borrow(), 1, "应发送一次暂停键");

        controller.show();
        assert!(!controller.is_hidden());
        assert!(controller.wm.is_visible(10), "恢复后微信应可见");
        assert_eq!(*controller.effects.resumes.borrow(), vec![10], "应解冻");
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(10, true), (10, false)],
            "恢复后应取消静音"
        );
    }

    #[test]
    fn send_before_hide_fires_once_regardless_of_target_count() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = false;
        config.setting.freeze_after_hide = false;
        config.setting.send_before_hide = true;
        config.window_rules = vec![
            wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
            wrule("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
        ];

        let wm = MockWm::new(
            vec![
                win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
                win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
            ],
            10,
        );
        let mut controller = HideController::new(wm, MockEffects::default());

        do_hide(&mut controller, &mut config);
        assert!(!controller.wm.is_visible(10) && !controller.wm.is_visible(20));
        assert_eq!(
            *controller.effects.pauses.borrow(),
            1,
            "多个目标也只应发送一次暂停键，否则「播放/暂停」会被切回播放"
        );
    }

    #[test]
    fn snapshot_reflects_hidden_state_and_clears_after_show() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = true;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        let mut controller = HideController::new(wm, MockEffects::default());

        assert!(controller.snapshot().is_empty(), "初始快照应为空");

        do_hide(&mut controller, &mut config);
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.hidden, vec![Target { hwnd: 10, pid: 10 }]);
        assert_eq!(snapshot.frozen, vec![10]);
        assert_eq!(snapshot.muted, vec![10]);

        controller.show();
        assert!(controller.snapshot().is_empty(), "显示后快照应清空");
    }

    #[test]
    fn restore_from_snapshot_shows_windows_and_reverts_effects() {
        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        wm.hide(10);
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.restore_from(Snapshot {
            hidden: vec![Target { hwnd: 10, pid: 10 }],
            frozen: vec![10],
            muted: vec![10],
            enhanced: false,
        });

        assert!(!controller.is_hidden(), "恢复完成后应回到未隐藏状态");
        assert!(controller.wm.is_visible(10), "崩溃前隐藏的窗口应被找回");
        assert_eq!(*controller.effects.resumes.borrow(), vec![10], "应解冻进程");
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(10, false)],
            "应取消静音"
        );
        assert!(controller.snapshot().is_empty());
    }

    #[test]
    fn expand_descendants_collects_multi_level_tree() {
        // 1 → 2 → 4, 1 → 3；另有无关的 9 → 10。
        let edges = [(2, 1), (3, 1), (4, 2), (10, 9)];
        let got = expand_descendants(&[1], &edges);
        assert_eq!(got, vec![1, 2, 3, 4], "应收集根及全部后代，不含无关分支");
    }

    #[test]
    fn expand_descendants_handles_cycles_and_self_reference() {
        // 自指 (0,0) 与环 1→2→1 都不能导致死循环。
        let edges = [(0, 0), (1, 2), (2, 1)];
        let got = expand_descendants(&[1], &edges);
        assert_eq!(got, vec![1, 2]);
    }

    #[test]
    fn expand_descendants_dedups_overlapping_roots_and_skips_zero() {
        let edges = [(2, 1), (3, 1)];
        // 根含 0（应跳过）与重叠子树。
        let got = expand_descendants(&[1, 2, 0], &edges);
        assert_eq!(got, vec![1, 2, 3]);
    }

    #[test]
    fn expand_descendants_leaf_root_returns_itself() {
        let edges = [(2, 1)];
        assert_eq!(expand_descendants(&[2], &edges), vec![2]);
        assert!(expand_descendants(&[], &edges).is_empty());
    }

    #[test]
    fn disabled_effects_are_not_applied() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = false;
        config.setting.freeze_after_hide = false;
        config.setting.send_before_hide = false;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        let mut controller = HideController::new(wm, MockEffects::default());

        do_hide(&mut controller, &mut config);
        assert!(controller.effects.mutes.borrow().is_empty());
        assert!(controller.effects.suspends.borrow().is_empty());
        assert_eq!(*controller.effects.pauses.borrow(), 0);
    }
}
