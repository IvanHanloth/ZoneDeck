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
    /// 进程的可执行文件完整路径；查不到返回空串。
    ///
    /// 枚举得到的 [`WindowInfo`] 已带路径，此方法是给「只拿到句柄」的目标补身份用的
    /// ——任务栏与桌面这类窗口带 `WS_EX_TOOLWINDOW`，压根不在枚举结果里。
    fn process_path(&self, pid: u32) -> String;
    /// 窗口当前标题；无标题或查不到时返回 `NO_TITLE` 占位。
    fn window_title(&self, hwnd: i64) -> String;
    /// 进程创建时刻（Unix 毫秒），用于识别 PID 复用；查不到返回 0。
    fn process_start_time(&self, pid: u32) -> i64;
}

pub fn manager() -> impl WindowManager {
    win32::WindowsWindowManager
}
