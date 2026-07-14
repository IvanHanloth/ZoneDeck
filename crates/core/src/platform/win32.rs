use std::ffi::c_void;

use bosskey_common::{NO_TITLE, WindowInfo};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, SW_HIDE,
    SW_SHOW, ShowWindow, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};
use windows::core::{BOOL, PWSTR};

use super::WindowManager;

const PATH_BUF_LEN: usize = 1024;

fn hwnd_from(raw: i64) -> HWND {
    HWND(raw as isize as *mut c_void)
}

fn hwnd_to_i64(hwnd: HWND) -> i64 {
    hwnd.0 as isize as i64
}

fn window_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return NO_TITLE.to_string();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let read = GetWindowTextW(hwnd, &mut buf);
        if read <= 0 {
            return NO_TITLE.to_string();
        }
        let title = String::from_utf16_lossy(&buf[..read as usize]);
        if title.is_empty() {
            NO_TITLE.to_string()
        } else {
            title
        }
    }
}

fn window_pid(hwnd: HWND) -> u32 {
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32));
    }
    pid
}

pub(crate) fn process_path(pid: u32) -> String {
    if pid == 0 {
        return String::new();
    }
    unsafe {
        let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => h,
            Err(_) => return String::new(),
        };
        let mut buf = vec![0u16; PATH_BUF_LEN];
        let mut size = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        match result {
            Ok(()) => String::from_utf16_lossy(&buf[..size as usize]),
            Err(_) => String::new(),
        }
    }
}

fn process_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// 是否应把该顶层窗口列入进程列表（类 Alt+Tab 的过滤，纯逻辑便于测试）：
/// - 排除工具窗口（`WS_EX_TOOLWINDOW`）；
/// - 只保留顶层应用窗口：无属主，或显式标记为 `WS_EX_APPWINDOW`。
///
/// 不按可见性和标题过滤——隐藏窗口、无标题窗口都会列出，由界面按 `visible`
/// 分组、并用「显示后台 / 显示无标题」开关决定是否展示。
fn is_listable_window(ex_style: u32, has_owner: bool) -> bool {
    if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
        return false;
    }
    if has_owner && (ex_style & WS_EX_APPWINDOW.0 == 0) {
        return false;
    }
    true
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let has_owner = GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid());
        if !is_listable_window(ex_style, has_owner) {
            return BOOL(1);
        }

        let sink = &mut *(lparam.0 as *mut Vec<WindowInfo>);
        let visible = IsWindowVisible(hwnd).as_bool();
        let title = window_title(hwnd);
        let pid = window_pid(hwnd);
        let path = process_path(pid);
        let process = process_name_from_path(&path);
        sink.push(
            WindowInfo::new(title, hwnd_to_i64(hwnd), process, pid, path).with_visibility(visible),
        );
    }
    BOOL(1)
}

pub struct WindowsWindowManager;

impl WindowManager for WindowsWindowManager {
    fn enumerate(&self) -> Vec<WindowInfo> {
        let mut result: Vec<WindowInfo> = Vec::new();
        unsafe {
            let _ = EnumWindows(
                Some(enum_proc),
                LPARAM(&mut result as *mut Vec<WindowInfo> as isize),
            );
        }
        result.sort_by(|a, b| a.title.cmp(&b.title));
        result
    }

    fn hide(&self, hwnd: i64) {
        unsafe {
            let _ = ShowWindow(hwnd_from(hwnd), SW_HIDE);
        }
    }

    fn show(&self, hwnd: i64) {
        unsafe {
            let _ = ShowWindow(hwnd_from(hwnd), SW_SHOW);
        }
    }

    fn is_visible(&self, hwnd: i64) -> bool {
        unsafe { IsWindowVisible(hwnd_from(hwnd)).as_bool() }
    }

    fn foreground(&self) -> i64 {
        unsafe { hwnd_to_i64(GetForegroundWindow()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_name_extracts_basename() {
        assert_eq!(
            process_name_from_path("C:\\Windows\\notepad.exe"),
            "notepad.exe"
        );
        assert_eq!(process_name_from_path("D:\\a\\b\\WeChat.exe"), "WeChat.exe");
        assert_eq!(process_name_from_path(""), "");
    }

    #[test]
    fn hwnd_round_trips_through_i64() {
        let h = hwnd_from(123456);
        assert_eq!(hwnd_to_i64(h), 123456);
    }

    #[test]
    fn is_listable_window_filters_like_alt_tab() {
        const TOOL: u32 = WS_EX_TOOLWINDOW.0;
        const APP: u32 = WS_EX_APPWINDOW.0;

        // 顶层窗口（无属主）保留（含无标题窗口，由界面过滤）。
        assert!(is_listable_window(0, false));
        // 工具窗口排除。
        assert!(!is_listable_window(TOOL, false));
        // 有属主的普通窗口排除（如对话框、子面板）。
        assert!(!is_listable_window(0, true));
        // 有属主但显式标记为 APPWINDOW 的仍保留。
        assert!(is_listable_window(APP, true));
    }

    use windows::Win32::UI::WindowsAndMessaging::{
        CW_USEDEFAULT, CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_OVERLAPPEDWINDOW,
    };
    use windows::core::w;

    #[test]
    fn window_show_hide_enumerate_round_trip() {
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("Static"),
                w!("BossKeyTestWindow"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                200,
                120,
                None,
                None,
                None,
                None,
            )
            .expect("创建测试窗口失败");

            let wm = WindowsWindowManager;
            let id = hwnd_to_i64(hwnd);

            assert!(!wm.is_visible(id), "新建窗口初始应不可见");

            wm.show(id);
            assert!(wm.is_visible(id), "显示后应可见");
            assert!(
                wm.enumerate().iter().any(|w| w.hwnd == id),
                "显示后应能枚举到测试窗口"
            );

            wm.hide(id);
            assert!(!wm.is_visible(id), "隐藏后应不可见");
            let hidden_entry = wm.enumerate().into_iter().find(|w| w.hwnd == id);
            assert!(
                hidden_entry.is_some(),
                "隐藏后仍应能枚举到窗口（供界面后台分组展示）"
            );
            assert!(
                !hidden_entry.unwrap().visible,
                "隐藏后枚举结果的 visible 应为 false"
            );

            let _ = DestroyWindow(hwnd);
        }
    }
}
