use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    HICON, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadIconW, LoadImageW,
};
use windows::core::PCWSTR;

use crate::util::to_wide_null;

const TRAY_ID: u32 = 1;

fn fill_wide(dst: &mut [u16], src: &str) {
    let wide: Vec<u16> = src.encode_utf16().take(dst.len() - 1).collect();
    dst[..wide.len()].copy_from_slice(&wide);
    dst[wide.len()..].fill(0);
}

pub(crate) fn load_app_icon() -> HICON {
    let ico_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("icon.ico")));

    if let Some(path) = ico_path
        && path.exists()
        && let Some(path_str) = path.to_str()
    {
        let wide = to_wide_null(path_str);
        let loaded = unsafe {
            LoadImageW(
                None,
                PCWSTR(wide.as_ptr()),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            )
        };
        if let Ok(handle) = loaded {
            return HICON(handle.0);
        }
    }

    unsafe { LoadIconW(None, IDI_APPLICATION).unwrap_or_default() }
}

pub struct TrayIcon {
    hwnd: HWND,
    visible: bool,
    icon: HICON,
    tip: String,
    callback_msg: u32,
}

impl TrayIcon {
    pub fn new(hwnd: HWND, callback_msg: u32, tip: &str) -> Self {
        let mut tray = TrayIcon {
            hwnd,
            visible: false,
            icon: load_app_icon(),
            tip: tip.to_string(),
            callback_msg,
        };
        tray.show();
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
        if self.visible {
            return;
        }
        let data = self.base_data();
        unsafe {
            if Shell_NotifyIconW(NIM_ADD, &data).as_bool() {
                self.visible = true;
            }
        }
    }

    pub fn hide(&mut self) {
        if !self.visible {
            return;
        }
        let data = self.base_data();
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
        }
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
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
    fn fill_wide_handles_chinese() {
        let mut buf = [0u16; 16];
        fill_wide(&mut buf, "老板键");
        let s = String::from_utf16_lossy(&buf[..3]);
        assert_eq!(s, "老板键");
        assert_eq!(buf[3], 0);
    }
}
