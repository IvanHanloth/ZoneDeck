use std::collections::HashSet;

use bosskey_common::{Config, WindowInfo, matching::matches_binding};

use crate::platform::WindowManager;

pub fn windows_to_hide(
    bindings: &[WindowInfo],
    windows: &[WindowInfo],
    path_match: bool,
    hide_current: bool,
    foreground: i64,
) -> Vec<i64> {
    let mut result: Vec<i64> = Vec::new();

    for w in windows {
        if bindings.iter().any(|b| matches_binding(b, w, path_match)) {
            result.push(w.hwnd);
        }
    }

    if hide_current && foreground != 0 {
        result.push(foreground);
    }

    let mut seen = HashSet::new();
    result.retain(|hwnd| seen.insert(*hwnd));
    result
}

pub struct HideController<W: WindowManager> {
    wm: W,
    hidden: Vec<i64>,
}

impl<W: WindowManager> HideController<W> {
    pub fn new(wm: W) -> Self {
        Self {
            wm,
            hidden: Vec::new(),
        }
    }

    pub fn is_hidden(&self) -> bool {
        !self.hidden.is_empty()
    }

    pub fn hide(&mut self, config: &Config) {
        let windows = self.wm.enumerate();
        let foreground = self.wm.foreground();
        let targets = windows_to_hide(
            &config.hide_binding,
            &windows,
            config.setting.path_match,
            config.setting.hide_current,
            foreground,
        );
        for hwnd in &targets {
            self.wm.hide(*hwnd);
        }
        self.hidden = targets;
    }

    pub fn show(&mut self) {
        for hwnd in &self.hidden {
            self.wm.show(*hwnd);
        }
        self.hidden.clear();
    }

    pub fn toggle(&mut self, config: &Config) {
        if self.is_hidden() {
            self.show();
        } else {
            self.hide(config);
        }
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
        let targets = windows_to_hide(&bindings, &windows, false, false, 0);
        assert_eq!(targets, vec![10]);
    }

    #[test]
    fn hide_current_appends_foreground_and_dedups() {
        let bindings = vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];
        let windows = vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];
        let targets = windows_to_hide(&bindings, &windows, false, true, 10);
        assert_eq!(targets, vec![10], "前台窗口与已匹配窗口相同应去重");

        let targets2 = windows_to_hide(&bindings, &windows, false, true, 99);
        assert_eq!(targets2, vec![10, 99], "不同的前台窗口应追加");
    }

    #[test]
    fn path_match_hides_all_windows_of_same_executable() {
        let bindings = vec![win("窗口一", 10, "game.exe", "C:\\game.exe")];
        let windows = vec![
            win("窗口一", 10, "game.exe", "C:\\game.exe"),
            win("窗口二", 11, "game.exe", "C:\\game.exe"),
            win("窗口三", 12, "game.exe", "C:\\game.exe"),
        ];
        let targets = windows_to_hide(&bindings, &windows, true, false, 0);
        assert_eq!(
            targets,
            vec![10, 11, 12],
            "路径匹配应隐藏同一可执行文件的所有窗口"
        );
    }

    #[test]
    fn no_bindings_and_no_hide_current_selects_nothing() {
        let windows = vec![win("记事本", 20, "notepad.exe", "C:\\notepad.exe")];
        assert!(windows_to_hide(&[], &windows, false, false, 0).is_empty());
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

    #[test]
    fn controller_toggle_hides_then_restores() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.hide_binding = vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(
            vec![
                win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
                win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
            ],
            10,
        );
        let mut controller = HideController::new(wm);

        assert!(!controller.is_hidden());

        controller.toggle(&config);
        assert!(controller.is_hidden());
        assert!(!controller.wm.is_visible(10), "微信应被隐藏");
        assert!(controller.wm.is_visible(20), "记事本不在绑定内应保持可见");

        controller.toggle(&config);
        assert!(!controller.is_hidden());
        assert!(controller.wm.is_visible(10), "再次切换应恢复微信");
    }
}
