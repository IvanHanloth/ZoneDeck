use std::ffi::c_void;

use windows::Win32::Foundation::{CloseHandle, FILETIME, HWND, LPARAM};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow,
    IsWindowVisible, IsZoomed, SW_HIDE, SW_RESTORE, SW_SHOW, SW_SHOWMAXIMIZED, SW_SHOWMINNOACTIVE,
    ShowWindow, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};
use windows::core::{BOOL, PWSTR};
use zonedeck_common::{NO_TITLE, WindowInfo};

use super::{Restore, WindowManager};

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

/// FILETIME（1601 起 100ns）→ Unix 毫秒。
fn filetime_to_unix_ms(ft: FILETIME) -> i64 {
    const EPOCH_DIFF_100NS: i64 = 116_444_736_000_000_000;
    let raw = ((ft.dwHighDateTime as i64) << 32) | ft.dwLowDateTime as i64;
    (raw - EPOCH_DIFF_100NS) / 10_000
}

/// 进程创建时刻（Unix 毫秒）；打不开进程时返回 0。
pub(crate) fn process_start_time(pid: u32) -> i64 {
    if pid == 0 {
        return 0;
    }
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return 0;
        };
        let mut created = FILETIME::default();
        let mut exited = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        let result = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user);
        let _ = CloseHandle(handle);
        match result {
            Ok(()) => filetime_to_unix_ms(created),
            Err(_) => 0,
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

/// 是否把该顶层窗口列入进程列表；类 Alt+Tab 过滤，排除工具窗口。
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

    fn minimize(&self, hwnd: i64) {
        unsafe {
            let _ = ShowWindow(hwnd_from(hwnd), SW_SHOWMINNOACTIVE);
        }
    }

    fn restore_mode(&self, hwnd: i64) -> Restore {
        unsafe {
            let hwnd = hwnd_from(hwnd);
            // 顺序要紧：最小化的最大化窗口两个判定都为真，应按最小化记。
            if IsIconic(hwnd).as_bool() {
                Restore::Minimized
            } else if IsZoomed(hwnd).as_bool() {
                Restore::Maximized
            } else {
                Restore::Normal
            }
        }
    }

    fn restore(&self, hwnd: i64, how: Restore) {
        // SW_SHOW 与 SW_SHOWMINNOACTIVE 只改可见性；SW_RESTORE 与 SW_SHOWMAXIMIZED
        // 对隐藏中的窗口也会一并显示出来，无须先 SW_SHOW。
        let cmd = match how {
            Restore::Skip => return,
            Restore::Show => SW_SHOW,
            Restore::Normal => SW_RESTORE,
            Restore::Maximized => SW_SHOWMAXIMIZED,
            Restore::Minimized => SW_SHOWMINNOACTIVE,
        };
        unsafe {
            let _ = ShowWindow(hwnd_from(hwnd), cmd);
        }
    }

    fn is_visible(&self, hwnd: i64) -> bool {
        unsafe { IsWindowVisible(hwnd_from(hwnd)).as_bool() }
    }

    fn foreground(&self) -> i64 {
        unsafe { hwnd_to_i64(GetForegroundWindow()) }
    }

    fn is_window(&self, hwnd: i64) -> bool {
        unsafe { IsWindow(Some(hwnd_from(hwnd))).as_bool() }
    }

    fn window_pid(&self, hwnd: i64) -> u32 {
        window_pid(hwnd_from(hwnd))
    }

    fn process_path(&self, pid: u32) -> String {
        process_path(pid)
    }

    fn window_title(&self, hwnd: i64) -> String {
        window_title(hwnd_from(hwnd))
    }

    fn process_start_time(&self, pid: u32) -> i64 {
        process_start_time(pid)
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

        assert!(is_listable_window(0, false));
        assert!(!is_listable_window(TOOL, false));
        assert!(!is_listable_window(0, true));
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
                w!("ZoneDeckTestWindow"),
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

    #[test]
    fn filetime_epoch_maps_to_zero() {
        let ft = FILETIME {
            dwLowDateTime: (116_444_736_000_000_000u64 & 0xFFFF_FFFF) as u32,
            dwHighDateTime: (116_444_736_000_000_000u64 >> 32) as u32,
        };
        assert_eq!(filetime_to_unix_ms(ft), 0);
    }

    #[test]
    fn dead_handle_and_pid_are_detected() {
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("Static"),
                w!("ZoneDeckIdentityTestWindow"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                100,
                80,
                None,
                None,
                None,
                None,
            )
            .expect("创建测试窗口失败");
            let wm = WindowsWindowManager;
            let id = hwnd_to_i64(hwnd);

            assert!(wm.is_window(id), "存活窗口应通过 IsWindow");
            assert_eq!(
                wm.window_pid(id),
                std::process::id(),
                "自建窗口应属于本进程"
            );

            let _ = DestroyWindow(hwnd);
            assert!(!wm.is_window(id), "销毁后句柄应判定失效");
            assert_eq!(wm.window_pid(id), 0, "失效句柄查不到 PID");
        }
    }

    #[test]
    fn own_process_start_time_is_recent_and_stable() {
        let wm = WindowsWindowManager;
        let t1 = wm.process_start_time(std::process::id());
        let t2 = wm.process_start_time(std::process::id());
        assert!(t1 > 0, "本进程创建时刻应可查询");
        assert_eq!(t1, t2, "同一进程两次查询应一致");
        assert_eq!(wm.process_start_time(0), 0, "PID 0 应返回 0");
        assert_eq!(wm.process_start_time(0xFFFF_FFF0), 0, "无效 PID 应返回 0");
    }

    /// 创建一个测试窗口并交给 `body`，收尾销毁。
    fn with_test_window(title: windows::core::PCWSTR, body: impl FnOnce(i64)) {
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("Static"),
                title,
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                300,
                200,
                None,
                None,
                None,
                None,
            )
            .expect("创建测试窗口失败");
            body(hwnd_to_i64(hwnd));
            let _ = DestroyWindow(hwnd);
        }
    }

    #[test]
    fn restore_mode_reports_the_three_window_shapes() {
        with_test_window(w!("ZoneDeckPlacementTestWindow"), |id| {
            let wm = WindowsWindowManager;
            wm.show(id);
            assert_eq!(wm.restore_mode(id), Restore::Normal, "普通窗口");

            unsafe {
                let _ = ShowWindow(hwnd_from(id), SW_SHOWMAXIMIZED);
            }
            assert_eq!(wm.restore_mode(id), Restore::Maximized, "最大化窗口");

            wm.minimize(id);
            assert_eq!(
                wm.restore_mode(id),
                Restore::Minimized,
                "最小化的最大化窗口应按最小化记，恢复时不该替它还原成最大化"
            );
        });
    }

    #[test]
    fn restore_brings_hidden_window_back_in_its_recorded_shape() {
        with_test_window(w!("ZoneDeckRestoreTestWindow"), |id| {
            let wm = WindowsWindowManager;

            // 普通窗口：最小化 + 隐藏后应能一步恢复。
            wm.show(id);
            wm.minimize(id);
            wm.hide(id);
            assert!(!wm.is_visible(id));
            wm.restore(id, Restore::Normal);
            assert!(wm.is_visible(id), "恢复后应可见");
            assert_eq!(wm.restore_mode(id), Restore::Normal, "应还原为普通大小");

            // 最大化窗口：恢复后仍是最大化。
            unsafe {
                let _ = ShowWindow(hwnd_from(id), SW_SHOWMAXIMIZED);
            }
            wm.minimize(id);
            wm.hide(id);
            wm.restore(id, Restore::Maximized);
            assert!(wm.is_visible(id));
            assert_eq!(wm.restore_mode(id), Restore::Maximized, "应还原为最大化");

            // 本就最小化的窗口：恢复后重新可见但保持最小化。
            wm.minimize(id);
            wm.hide(id);
            wm.restore(id, Restore::Minimized);
            assert!(wm.is_visible(id), "应重新可见");
            assert_eq!(
                wm.restore_mode(id),
                Restore::Minimized,
                "本就最小化的窗口不该被还原大小"
            );
        });
    }

    #[test]
    fn restore_skip_leaves_the_window_alone() {
        with_test_window(w!("ZoneDeckSkipTestWindow"), |id| {
            let wm = WindowsWindowManager;
            wm.hide(id);
            wm.restore(id, Restore::Skip);
            assert!(
                !wm.is_visible(id),
                "本程序没让它可见，恢复时也不得把它弹出来"
            );
        });
    }
}
