pub mod win32;

use bosskey_common::WindowInfo;

pub trait WindowManager {
    fn enumerate(&self) -> Vec<WindowInfo>;
    fn hide(&self, hwnd: i64);
    fn show(&self, hwnd: i64);
    fn is_visible(&self, hwnd: i64) -> bool;
    fn foreground(&self) -> i64;
}

pub fn manager() -> impl WindowManager {
    win32::WindowsWindowManager
}
