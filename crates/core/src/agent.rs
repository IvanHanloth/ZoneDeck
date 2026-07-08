use bosskey_common::Config;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, PostQuitMessage, WM_HOTKEY};

use crate::hide::HideController;
use crate::hotkey::{MOD_NOREPEAT, ParsedHotkey, parse_hotkey};
use crate::platform::win32::WindowsWindowManager;

const HK_HIDE: i32 = 1;
const HK_CLOSE: i32 = 2;

unsafe fn register(id: i32, hk: &ParsedHotkey) -> bool {
    unsafe {
        RegisterHotKey(
            None,
            id,
            HOT_KEY_MODIFIERS(hk.modifiers | MOD_NOREPEAT),
            hk.vk as u32,
        )
        .is_ok()
    }
}

pub fn check(config: &Config) -> bool {
    let mut ok = true;
    for (id, label, raw) in [
        (HK_HIDE, "隐藏", &config.hotkey.hide_hotkey),
        (HK_CLOSE, "关闭", &config.hotkey.close_hotkey),
    ] {
        match parse_hotkey(raw) {
            Ok(hk) => unsafe {
                if register(id, &hk) {
                    let _ = UnregisterHotKey(None, id);
                } else {
                    eprintln!("{label}热键注册失败（可能已被占用）: {raw}");
                    ok = false;
                }
            },
            Err(e) => {
                eprintln!("{label}热键解析失败: {e}");
                ok = false;
            }
        }
    }
    ok
}

pub fn run(config: Config) {
    for (id, label, raw) in [
        (HK_HIDE, "隐藏", &config.hotkey.hide_hotkey),
        (HK_CLOSE, "关闭", &config.hotkey.close_hotkey),
    ] {
        match parse_hotkey(raw) {
            Ok(hk) => unsafe {
                if !register(id, &hk) {
                    eprintln!("{label}热键注册失败（可能已被占用）: {raw}");
                }
            },
            Err(e) => eprintln!("{label}热键解析失败: {e}"),
        }
    }

    let mut controller = HideController::new(WindowsWindowManager);

    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if ret.0 <= 0 {
                break;
            }
            if msg.message == WM_HOTKEY {
                match msg.wParam.0 as i32 {
                    HK_HIDE => controller.toggle(&config),
                    HK_CLOSE => {
                        controller.show();
                        PostQuitMessage(0);
                    }
                    _ => {}
                }
            }
        }

        let _ = UnregisterHotKey(None, HK_HIDE);
        let _ = UnregisterHotKey(None, HK_CLOSE);
    }
}
