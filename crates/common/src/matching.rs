use crate::NO_TITLE;
use crate::model::WindowInfo;

const PROCESS_EXCEPT: [&str; 1] = ["explorer.exe"];

pub fn is_same_window(w1: &WindowInfo, w2: &WindowInfo, auto: bool, strict: bool) -> bool {
    if w1 == w2 {
        return true;
    }

    let hwnd_same = w1.hwnd == w2.hwnd;
    let title_same = w1.title == w2.title && w1.title != NO_TITLE;
    let process_name_same =
        w1.process == w2.process && !PROCESS_EXCEPT.contains(&w1.process.as_str());
    let process_path_same = w1.path == w2.path;
    let pid_same = w1.pid == w2.pid;
    let process_same = process_name_same || pid_same;

    if !strict && process_name_same && process_path_same {
        return true;
    }

    if !auto {
        if process_name_same && title_same {
            return true;
        }
        if hwnd_same {
            return true;
        }
    }

    if hwnd_same && process_same {
        return true;
    }

    if process_same && title_same {
        return true;
    }

    false
}

pub fn matches_binding(binding: &WindowInfo, window: &WindowInfo, path_match: bool) -> bool {
    is_same_window(binding, window, false, !path_match)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(title: &str, hwnd: i64, process: &str, pid: u32, path: &str) -> WindowInfo {
        WindowInfo::new(title, hwnd, process, pid, path)
    }

    #[test]
    fn identical_windows_match() {
        let a = win("记事本", 1, "notepad.exe", 10, "C:\\notepad.exe");
        assert!(is_same_window(&a, &a.clone(), false, true));
    }

    #[test]
    fn same_hwnd_matches_in_non_auto() {
        let a = win("A", 100, "a.exe", 1, "C:\\a.exe");
        let b = win("B", 100, "b.exe", 2, "C:\\b.exe");
        assert!(is_same_window(&a, &b, false, true));
    }

    #[test]
    fn same_process_and_title_matches_strict() {
        let a = win("聊天", 1, "wechat.exe", 10, "C:\\a\\wechat.exe");
        let b = win("聊天", 2, "wechat.exe", 20, "D:\\b\\wechat.exe");
        assert!(is_same_window(&a, &b, false, true));
    }

    #[test]
    fn path_match_ignores_title_in_non_strict() {
        let a = win("窗口一", 1, "game.exe", 10, "C:\\game.exe");
        let b = win("窗口二", 2, "game.exe", 20, "C:\\game.exe");
        assert!(
            !is_same_window(&a, &b, false, true),
            "严格模式下标题不同不应匹配"
        );
        assert!(
            is_same_window(&a, &b, false, false),
            "非严格(路径匹配)模式下同名同路径应匹配"
        );
    }

    #[test]
    fn matches_binding_wraps_path_match_flag() {
        let binding = win("窗口一", 1, "game.exe", 10, "C:\\game.exe");
        let live = win("窗口二", 2, "game.exe", 20, "C:\\game.exe");
        assert!(!matches_binding(&binding, &live, false));
        assert!(matches_binding(&binding, &live, true));
    }

    #[test]
    fn explorer_process_is_excluded() {
        let a = win("此电脑", 1, "explorer.exe", 10, "C:\\Windows\\explorer.exe");
        let b = win("此电脑", 2, "explorer.exe", 20, "C:\\Windows\\explorer.exe");
        assert!(
            !is_same_window(&a, &b, false, true),
            "explorer.exe 进程名相同不应视为同一窗口"
        );
    }

    #[test]
    fn no_title_windows_do_not_match_on_title_alone() {
        let a = win(NO_TITLE, 1, "a.exe", 10, "C:\\a.exe");
        let b = win(NO_TITLE, 2, "b.exe", 20, "C:\\b.exe");
        assert!(!is_same_window(&a, &b, false, true));
    }

    #[test]
    fn completely_different_windows_do_not_match() {
        let a = win("A", 1, "a.exe", 10, "C:\\a.exe");
        let b = win("B", 2, "b.exe", 20, "C:\\b.exe");
        assert!(!is_same_window(&a, &b, false, true));
    }

    #[test]
    fn same_pid_and_title_matches() {
        let a = win("同标题", 1, "a.exe", 500, "C:\\a.exe");
        let b = win("同标题", 2, "b.exe", 500, "C:\\b.exe");
        assert!(is_same_window(&a, &b, false, true));
    }
}
