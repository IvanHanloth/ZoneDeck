use std::ffi::c_void;

use bosskey_common::{NO_TITLE, WindowInfo};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, SW_HIDE, SW_SHOW, ShowWindow,
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

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        let sink = &mut *(lparam.0 as *mut Vec<WindowInfo>);
        let title = window_title(hwnd);
        let pid = window_pid(hwnd);
        let path = process_path(pid);
        let process = process_name_from_path(&path);
        sink.push(WindowInfo::new(
            title,
            hwnd_to_i64(hwnd),
            process,
            pid,
            path,
        ));
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
            assert!(
                !wm.enumerate().iter().any(|w| w.hwnd == id),
                "隐藏后不应枚举到测试窗口"
            );

            let _ = DestroyWindow(hwnd);
        }
    }
}
