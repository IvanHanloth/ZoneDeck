use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{PCWSTR, w};

use crate::util::to_wide_null;

/// 用系统默认程序打开路径或 URL（等价于在资源管理器里双击）。
///
/// 走 ShellExecute 的 "open" 动词，而不是直接 `CreateProcess` 启动
/// explorer.exe：当本进程以管理员（高完整性令牌）运行时——例如配置程序
/// 由已提权的核心从托盘菜单拉起——直接启动 explorer.exe 会被系统拒绝，
/// 报「拒绝访问 / OS Error 5」。"open" 动词交由 Shell 处理，不受完整性
/// 级别限制。
pub fn open(target: &str) -> Result<(), String> {
    let wide = to_wide_null(target);
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute 约定：返回值 > 32 表示成功。
    let code = result.0 as isize;
    if code > 32 {
        Ok(())
    } else {
        Err(format!("打开失败（ShellExecute 返回 {code}）"))
    }
}
