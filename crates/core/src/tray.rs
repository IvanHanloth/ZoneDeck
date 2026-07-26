use std::sync::OnceLock;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    HICON, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadIconW, LoadImageW,
    RegisterWindowMessageW,
};
use windows::core::{PCWSTR, w};

use crate::logging;
use crate::util::to_wide_null;

const TRAY_ID: u32 = 1;

/// 定时重试挂载的上限：仅作为 TaskbarCreated 广播的兜底，超限后靠广播补挂。
const MAX_TRAY_RETRY: u32 = 30;

/// TaskbarCreated 广播消息 ID：explorer（重）建任务栏时向所有顶层窗口广播。
pub(crate) fn taskbar_created_msg() -> u32 {
    static MSG: OnceLock<u32> = OnceLock::new();
    *MSG.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}

/// 决定是否继续定时重试挂载托盘图标。
fn should_retry(desired: bool, visible: bool, attempts: u32) -> bool {
    desired && !visible && attempts < MAX_TRAY_RETRY
}

/// 嵌入 exe 的主图标资源 ID（tauri-winres 默认，即 IDI_APPLICATION）。
const APP_ICON_RESOURCE_ID: u16 = 32512;

fn fill_wide(dst: &mut [u16], src: &str) {
    let wide: Vec<u16> = src.encode_utf16().take(dst.len() - 1).collect();
    dst[..wide.len()].copy_from_slice(&wide);
    dst[wide.len()..].fill(0);
}

pub(crate) fn load_app_icon() -> HICON {
    // 优先 exe 内嵌图标，其次同目录 icon.ico，最后系统默认图标。
    if let Some(icon) = load_embedded_icon() {
        return icon;
    }

    if let Some(icon) = load_icon_from_file() {
        return icon;
    }

    unsafe { LoadIconW(None, IDI_APPLICATION).unwrap_or_default() }
}

fn load_embedded_icon() -> Option<HICON> {
    unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        let handle = LoadImageW(
            Some(hinst.into()),
            PCWSTR(APP_ICON_RESOURCE_ID as usize as *const u16),
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE,
        )
        .ok()?;
        if handle.is_invalid() {
            None
        } else {
            Some(HICON(handle.0))
        }
    }
}

fn load_icon_from_file() -> Option<HICON> {
    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("icon.ico")))?;
    if !path.exists() {
        return None;
    }
    let wide = to_wide_null(path.to_str()?);
    unsafe {
        let handle = LoadImageW(
            None,
            PCWSTR(wide.as_ptr()),
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        )
        .ok()?;
        if handle.is_invalid() {
            None
        } else {
            Some(HICON(handle.0))
        }
    }
}

pub struct TrayIcon {
    hwnd: HWND,
    /// 业务层期望的可见性（如「同时隐藏 Boss Key 托盘图标」时为 false）。
    desired: bool,
    /// 图标是否实际已挂到任务栏（NIM_ADD 成功）。
    visible: bool,
    /// 定时兜底重试的已尝试次数，挂载成功后清零。
    retry_attempts: u32,
    icon: HICON,
    tip: String,
    callback_msg: u32,
}

impl TrayIcon {
    pub fn new(hwnd: HWND, callback_msg: u32, tip: &str) -> Self {
        let mut tray = TrayIcon {
            hwnd,
            desired: true,
            visible: false,
            retry_attempts: 0,
            icon: load_app_icon(),
            tip: tip.to_string(),
            callback_msg,
        };
        if !tray.try_add() {
            logging::warn("托盘图标初始挂载失败（任务栏可能尚未就绪），将在任务栏就绪后自动补挂");
        }
        tray
    }

    fn base_data(&self) -> NOTIFYICONDATAW {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: self.callback_msg,
            hIcon: self.icon,
            ..Default::default()
        };
        fill_wide(&mut data.szTip, &self.tip);
        data
    }

    pub fn show(&mut self) {
        self.desired = true;
        self.try_add();
    }

    pub fn hide(&mut self) {
        self.desired = false;
        if !self.visible {
            return;
        }
        let data = self.base_data();
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
        }
        self.visible = false;
    }

    /// 尝试把图标挂到任务栏，返回挂载后是否可见。已可见时直接成功。
    fn try_add(&mut self) -> bool {
        if self.visible {
            return true;
        }
        let data = self.base_data();
        if unsafe { Shell_NotifyIconW(NIM_ADD, &data).as_bool() } {
            self.visible = true;
            self.retry_attempts = 0;
        }
        self.visible
    }

    /// 任务栏（重）建后重挂图标：覆盖 explorer 重启，以及开机计划任务先于任务栏启动的场景。
    pub fn on_taskbar_created(&mut self) {
        // 旧图标已随任务栏一起消失，重置实际状态后按期望重挂。
        self.visible = false;
        self.retry_attempts = 0;
        if self.desired && self.try_add() {
            logging::info("任务栏已就绪，托盘图标已重新挂载");
        }
    }

    /// 定时兜底重试：图标应显示但尚未挂上时再试一次。返回是否仍需继续重试。
    pub fn retry_pending(&mut self) -> bool {
        if !should_retry(self.desired, self.visible, self.retry_attempts) {
            return false;
        }
        self.retry_attempts += 1;
        if self.try_add() {
            logging::info("托盘图标重试挂载成功");
            return false;
        }
        if self.retry_attempts >= MAX_TRAY_RETRY {
            logging::warn("托盘图标多次挂载失败，停止定时重试（任务栏就绪后仍会自动补挂）");
            return false;
        }
        true
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 替换托盘图标（状态角标变化时调用）。已挂载时立即 NIM_MODIFY 生效，
    /// 未挂载时仅记下句柄，待补挂时一并带上。图标未变化时不重复提交。
    pub fn set_icon(&mut self, icon: HICON) {
        if self.icon == icon {
            return;
        }
        self.icon = icon;
        self.modify_if_visible();
    }

    /// 替换悬浮提示文字（空串 = 悬停不显示任何文字）。文字未变化时不重复提交。
    pub fn set_tip(&mut self, tip: &str) {
        if self.tip == tip {
            return;
        }
        self.tip = tip.to_string();
        self.modify_if_visible();
    }

    fn modify_if_visible(&self) {
        if !self.visible {
            return;
        }
        let data = self.base_data();
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }

    pub fn balloon(&self, title: &str, message: &str) {
        if !self.visible {
            return;
        }
        let mut data = self.base_data();
        data.uFlags |= NIF_INFO;
        data.dwInfoFlags = NIIF_INFO;
        fill_wide(&mut data.szInfoTitle, title);
        fill_wide(&mut data.szInfo, message);
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_wide_truncates_and_null_terminates() {
        let mut buf = [0xFFu16; 8];
        fill_wide(&mut buf, "ABCDEFGHIJ");
        assert_eq!(&buf[..7], &[0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47]);
        assert_eq!(buf[7], 0, "最后一位必须是 null 终止符");
    }

    #[test]
    fn should_retry_only_when_desired_and_not_visible() {
        assert!(should_retry(true, false, 0), "期望显示且未挂上时应重试");
        assert!(!should_retry(true, true, 0), "已挂上则无需重试");
        assert!(!should_retry(false, false, 0), "业务层不希望显示时不重试");
        assert!(!should_retry(false, true, 0));
    }

    #[test]
    fn should_retry_stops_after_max_attempts() {
        assert!(should_retry(true, false, MAX_TRAY_RETRY - 1));
        assert!(
            !should_retry(true, false, MAX_TRAY_RETRY),
            "达到上限后停止定时重试"
        );
        assert!(!should_retry(true, false, MAX_TRAY_RETRY + 1));
    }

    #[test]
    fn fill_wide_handles_chinese() {
        let mut buf = [0u16; 16];
        fill_wide(&mut buf, "老板键");
        let s = String::from_utf16_lossy(&buf[..3]);
        assert_eq!(s, "老板键");
        assert_eq!(buf[3], 0);
    }
}
