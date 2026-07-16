use std::path::Path;

use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{PCWSTR, w};

use crate::util::to_wide_null;

/// 当前进程是否以管理员（提升的令牌）运行。
pub fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

/// 以管理员身份（UAC 提权）启动指定程序；返回用户是否同意提权。
pub fn relaunch_as_admin(exe: &Path, args: &str) -> bool {
    let file = to_wide_null(&exe.to_string_lossy());
    let params = to_wide_null(args);
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("runas"),
            PCWSTR(file.as_ptr()),
            if args.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(params.as_ptr())
            },
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute 约定：返回值 > 32 表示成功。
    result.0 as isize > 32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_elevated_returns_without_panicking() {
        let _ = is_elevated();
    }
}
