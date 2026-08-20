pub mod win32;

use serde::{Deserialize, Serialize};
use zonedeck_common::WindowInfo;

/// 恢复一个隐藏记录时该怎么对待它的窗口：只逆转本程序造成的改变。
/// 取值在隐藏前算出，随 `recovery.json` 落盘。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Restore {
    /// 显示出来即可，不动大小；也是恢复文件缺该字段时的回落值。
    #[default]
    Show,
    /// 隐藏前是普通窗口，还原为普通大小。
    Normal,
    /// 隐藏前是最大化窗口，还原为最大化。
    Maximized,
    /// 隐藏前就已最小化：只让它重新可见，保持最小化。
    Minimized,
    /// 隐藏前它就不可见：恢复时不得把它弹出来，但副作用照常施加与撤销。
    Skip,
}

impl Restore {
    /// 该取值是否意味着窗口在隐藏前要先被最小化；[`Restore::Minimized`] 也算。
    pub fn wants_minimize(self) -> bool {
        matches!(self, Self::Normal | Self::Maximized | Self::Minimized)
    }
}

pub trait WindowManager {
    fn enumerate(&self) -> Vec<WindowInfo>;
    fn hide(&self, hwnd: i64);
    fn show(&self, hwnd: i64);
    /// 最小化窗口，但不激活它。
    fn minimize(&self, hwnd: i64);
    /// 窗口当前形态对应的恢复方式；只返回 `Normal` / `Maximized` / `Minimized`。
    fn restore_mode(&self, hwnd: i64) -> Restore;
    /// 按记录的方式恢复窗口；[`Restore::Skip`] 什么都不做。
    fn restore(&self, hwnd: i64, how: Restore);
    fn is_visible(&self, hwnd: i64) -> bool;
    fn foreground(&self) -> i64;
    /// 句柄当前是否仍指向一个存在的窗口（句柄值会被系统回收复用）。
    fn is_window(&self, hwnd: i64) -> bool;
    /// 句柄当前所属进程的 PID；查不到返回 0。
    fn window_pid(&self, hwnd: i64) -> u32;
    /// 进程的可执行文件完整路径；查不到返回空串。
    /// 给「只拿到句柄」的目标补身份用（任务栏、桌面不在枚举结果里）。
    fn process_path(&self, pid: u32) -> String;
    /// 窗口当前标题；无标题或查不到时返回 `NO_TITLE` 占位。
    fn window_title(&self, hwnd: i64) -> String;
    /// 进程创建时刻（Unix 毫秒），用于识别 PID 复用；查不到返回 0。
    fn process_start_time(&self, pid: u32) -> i64;
}

pub fn manager() -> impl WindowManager {
    win32::WindowsWindowManager
}
