pub mod win32;

use zonedeck_common::WindowInfo;

pub trait WindowManager {
    fn enumerate(&self) -> Vec<WindowInfo>;
    fn hide(&self, hwnd: i64);
    fn show(&self, hwnd: i64);
    fn is_visible(&self, hwnd: i64) -> bool;
    fn foreground(&self) -> i64;
    /// 句柄当前是否仍指向一个存在的窗口（句柄值会被系统回收复用）。
    fn is_window(&self, hwnd: i64) -> bool;
    /// 句柄当前所属进程的 PID；查不到返回 0。
    fn window_pid(&self, hwnd: i64) -> u32;
    /// 窗口当前标题；无标题或查不到时返回 `NO_TITLE` 占位。
    fn window_title(&self, hwnd: i64) -> String;
    /// 进程创建时刻（Unix 毫秒），用于识别 PID 复用；查不到返回 0。
    fn process_start_time(&self, pid: u32) -> i64;
}

pub fn manager() -> impl WindowManager {
    win32::WindowsWindowManager
}
