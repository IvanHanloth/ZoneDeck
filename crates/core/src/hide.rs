use std::collections::HashSet;
use std::time::Duration;

use bosskey_common::{Config, WindowInfo, matching::matches_binding};
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

pub fn select_targets(
    bindings: &[WindowInfo],
    windows: &[WindowInfo],
    path_match: bool,
    hide_current: bool,
    foreground: i64,
) -> Vec<Target> {
    let mut result: Vec<Target> = Vec::new();

    for w in windows {
        if bindings.iter().any(|b| matches_binding(b, w, path_match)) {
            result.push(Target {
                hwnd: w.hwnd,
                pid: w.pid,
            });
        }
    }

    if hide_current && foreground != 0 {
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
    result
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

    pub fn hide(&mut self, config: &Config) {
        let windows = self.wm.enumerate();
        let foreground = self.wm.foreground();
        let setting = &config.setting;
        let targets = select_targets(
            &config.hide_binding,
            &windows,
            setting.path_match,
            setting.hide_current,
            foreground,
        );

        let mut frozen = Vec::new();
        let mut muted = Vec::new();

        for t in &targets {
            if setting.send_before_hide {
                self.effects.send_pause();
                std::thread::sleep(SEND_PAUSE_DELAY);
            }
            self.wm.hide(t.hwnd);
            if setting.mute_after_hide && t.pid != 0 {
                self.effects.mute(t.pid, true);
                muted.push(t.pid);
            }
            if setting.freeze_after_hide && t.pid != 0 {
                frozen.push(t.pid);
            }
        }

        self.used_enhanced = setting.enhanced_freeze;
        for pid in &frozen {
            self.effects.suspend(*pid, self.used_enhanced);
        }

        self.frozen = frozen;
        self.muted = muted;
        self.hidden = targets;
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

    pub fn toggle(&mut self, config: &Config) {
        if self.is_hidden() {
            self.show();
        } else {
            self.hide(config);
        }
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

    fn win(title: &str, hwnd: i64, process: &str, path: &str) -> WindowInfo {
        WindowInfo::new(title, hwnd, process, hwnd as u32, path)
    }

    #[test]
    fn selects_windows_matching_a_binding() {
        let bindings = vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];
        let windows = vec![
            win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
            win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
        ];
        let targets = select_targets(&bindings, &windows, false, false, 0);
        assert_eq!(targets, vec![Target { hwnd: 10, pid: 10 }]);
    }

    #[test]
    fn hide_current_appends_foreground_with_its_pid_and_dedups() {
        let bindings = vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];
        let windows = vec![
            win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
            win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
        ];
        let targets = select_targets(&bindings, &windows, false, true, 20);
        assert_eq!(
            targets,
            vec![Target { hwnd: 10, pid: 10 }, Target { hwnd: 20, pid: 20 }]
        );

        let same = select_targets(&bindings, &windows, false, true, 10);
        assert_eq!(
            same,
            vec![Target { hwnd: 10, pid: 10 }],
            "前台与匹配窗口相同应去重"
        );
    }

    #[test]
    fn path_match_hides_all_windows_of_same_executable() {
        let bindings = vec![win("窗口一", 10, "game.exe", "C:\\game.exe")];
        let windows = vec![
            win("窗口一", 10, "game.exe", "C:\\game.exe"),
            win("窗口二", 11, "game.exe", "C:\\game.exe"),
        ];
        let targets = select_targets(&bindings, &windows, true, false, 0);
        assert_eq!(
            targets,
            vec![Target { hwnd: 10, pid: 10 }, Target { hwnd: 11, pid: 11 }]
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
        fn send_pause(&self) {
            *self.pauses.borrow_mut() += 1;
        }
    }

    #[test]
    fn toggle_applies_mute_and_freeze_then_restores() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = true;
        config.setting.send_before_hide = true;
        config.hide_binding = vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(
            vec![
                win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
                win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
            ],
            10,
        );
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.hide(&config);
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
    fn snapshot_reflects_hidden_state_and_clears_after_show() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = true;
        config.hide_binding = vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        let mut controller = HideController::new(wm, MockEffects::default());

        assert!(controller.snapshot().is_empty(), "初始快照应为空");

        controller.hide(&config);
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.hidden, vec![Target { hwnd: 10, pid: 10 }]);
        assert_eq!(snapshot.frozen, vec![10]);
        assert_eq!(snapshot.muted, vec![10]);

        controller.show();
        assert!(controller.snapshot().is_empty(), "显示后快照应清空");
    }

    #[test]
    fn restore_from_snapshot_shows_windows_and_reverts_effects() {
        // 模拟：上次崩溃前隐藏了窗口 10（已冻结+静音），窗口当前仍不可见。
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
    fn disabled_effects_are_not_applied() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = false;
        config.setting.freeze_after_hide = false;
        config.setting.send_before_hide = false;
        config.hide_binding = vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.hide(&config);
        assert!(controller.effects.mutes.borrow().is_empty());
        assert!(controller.effects.suspends.borrow().is_empty());
        assert_eq!(*controller.effects.pauses.borrow(), 0);
    }
}
