use std::sync::atomic::{AtomicI32, AtomicIsize, AtomicU32, AtomicU64, Ordering::Relaxed};

use bosskey_common::Setting;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetSystemMetrics, HHOOK, MSLLHOOKSTRUCT, PostMessageW, SM_CXSCREEN,
    SM_CYSCREEN, SetWindowsHookExW, UnhookWindowsHookEx, WH_MOUSE_LL, WM_APP, WM_MBUTTONDOWN,
    WM_MOUSEMOVE, WM_XBUTTONDOWN,
};
use windows::core::PCWSTR;

pub const WM_MOUSE_TRIGGER: u32 = WM_APP + 3;
pub const TRIGGER_BUTTON: usize = 0;
pub const TRIGGER_CORNER: usize = 1;

const F_MIDDLE: u32 = 1;
const F_SIDE1: u32 = 2;
const F_SIDE2: u32 = 4;
const F_TL: u32 = 8;
const F_TR: u32 = 16;
const F_BL: u32 = 32;
const F_BR: u32 = 64;
const CORNER_MASK: u32 = F_TL | F_TR | F_BL | F_BR;
const TRIGGER_MASK: u32 = F_MIDDLE | F_SIDE1 | F_SIDE2 | CORNER_MASK;

const CORNER_THRESHOLD: i32 = 10;
const CORNER_COOLDOWN_MS: u64 = 1000;

static HWND_RAW: AtomicIsize = AtomicIsize::new(0);
static FLAGS: AtomicU32 = AtomicU32::new(0);
static SCREEN_W: AtomicI32 = AtomicI32::new(0);
static SCREEN_H: AtomicI32 = AtomicI32::new(0);
static LAST_CORNER: AtomicU64 = AtomicU64::new(0);

fn flags_from(s: &Setting) -> u32 {
    let mut f = 0;
    if s.middle_button_hide {
        f |= F_MIDDLE;
    }
    if s.side_button1_hide {
        f |= F_SIDE1;
    }
    if s.side_button2_hide {
        f |= F_SIDE2;
    }
    if s.top_left_hide {
        f |= F_TL;
    }
    if s.top_right_hide {
        f |= F_TR;
    }
    if s.bottom_left_hide {
        f |= F_BL;
    }
    if s.bottom_right_hide {
        f |= F_BR;
    }
    f
}

pub fn wants_hook(s: &Setting) -> bool {
    flags_from(s) & TRIGGER_MASK != 0
}

pub fn set_flags(s: &Setting) {
    FLAGS.store(flags_from(s), Relaxed);
}

fn corner_hit(x: i32, y: i32, w: i32, h: i32, flags: u32) -> bool {
    let t = CORNER_THRESHOLD;
    let left = x <= t;
    let right = x >= w - t;
    let top = y <= t;
    let bottom = y >= h - t;
    (top && left && flags & F_TL != 0)
        || (top && right && flags & F_TR != 0)
        || (bottom && left && flags & F_BL != 0)
        || (bottom && right && flags & F_BR != 0)
}

fn post(source: usize) {
    let raw = HWND_RAW.load(Relaxed);
    if raw == 0 {
        return;
    }
    unsafe {
        let hwnd = HWND(raw as *mut std::ffi::c_void);
        let _ = PostMessageW(Some(hwnd), WM_MOUSE_TRIGGER, WPARAM(source), LPARAM(0));
    }
}

fn handle_event(msg: u32, data: &MSLLHOOKSTRUCT) {
    let flags = FLAGS.load(Relaxed);
    match msg {
        WM_MBUTTONDOWN if flags & F_MIDDLE != 0 => post(TRIGGER_BUTTON),
        WM_XBUTTONDOWN => {
            let xbutton = (data.mouseData >> 16) as u16;
            if (xbutton == 1 && flags & F_SIDE1 != 0) || (xbutton == 2 && flags & F_SIDE2 != 0) {
                post(TRIGGER_BUTTON);
            }
        }
        WM_MOUSEMOVE if flags & CORNER_MASK != 0 => {
            let now = unsafe { GetTickCount64() };
            if now.wrapping_sub(LAST_CORNER.load(Relaxed)) < CORNER_COOLDOWN_MS {
                return;
            }
            let (w, h) = (SCREEN_W.load(Relaxed), SCREEN_H.load(Relaxed));
            if corner_hit(data.pt.x, data.pt.y, w, h, flags) {
                LAST_CORNER.store(now, Relaxed);
                post(TRIGGER_CORNER);
            }
        }
        _ => {}
    }
}

unsafe extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if ncode == 0 {
            let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            handle_event(wparam.0 as u32, data);
        }
        CallNextHookEx(None, ncode, wparam, lparam)
    }
}

pub struct MouseHook {
    handle: HHOOK,
}

impl MouseHook {
    pub fn install(agent_hwnd: HWND, setting: &Setting) -> Option<MouseHook> {
        set_flags(setting);
        HWND_RAW.store(agent_hwnd.0 as isize, Relaxed);
        unsafe {
            SCREEN_W.store(GetSystemMetrics(SM_CXSCREEN), Relaxed);
            SCREEN_H.store(GetSystemMetrics(SM_CYSCREEN), Relaxed);
            let hinstance = GetModuleHandleW(PCWSTR::null()).ok()?;
            let handle =
                SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), Some(hinstance.into()), 0).ok()?;
            Some(MouseHook { handle })
        }
    }
}

impl Drop for MouseHook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWindowsHookEx(self.handle);
        }
        HWND_RAW.store(0, Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting_with(mutate: impl FnOnce(&mut Setting)) -> Setting {
        let mut s = Setting::default();
        mutate(&mut s);
        s
    }

    #[test]
    fn wants_hook_only_when_a_trigger_is_enabled() {
        assert!(!wants_hook(&Setting::default()));
        assert!(wants_hook(&setting_with(|s| s.middle_button_hide = true)));
        assert!(wants_hook(&setting_with(|s| s.bottom_right_hide = true)));
        assert!(
            !wants_hook(&setting_with(|s| s.allow_move_restore = true)),
            "仅开启移动恢复不应安装钩子"
        );
    }

    #[test]
    fn flags_map_each_setting_bit() {
        assert_eq!(
            flags_from(&setting_with(|s| s.middle_button_hide = true)),
            F_MIDDLE
        );
        assert_eq!(
            flags_from(&setting_with(|s| s.side_button1_hide = true)),
            F_SIDE1
        );
        assert_eq!(
            flags_from(&setting_with(|s| s.side_button2_hide = true)),
            F_SIDE2
        );
        assert_eq!(flags_from(&setting_with(|s| s.top_left_hide = true)), F_TL);
    }

    #[test]
    fn corner_hit_detects_each_enabled_corner() {
        let (w, h) = (1920, 1080);
        assert!(corner_hit(0, 0, w, h, F_TL));
        assert!(corner_hit(1919, 0, w, h, F_TR));
        assert!(corner_hit(0, 1079, w, h, F_BL));
        assert!(corner_hit(1919, 1079, w, h, F_BR));
    }

    #[test]
    fn corner_hit_ignores_disabled_corners_and_center() {
        let (w, h) = (1920, 1080);
        assert!(!corner_hit(0, 0, w, h, F_TR), "左上角未启用时不触发");
        assert!(!corner_hit(960, 540, w, h, CORNER_MASK), "屏幕中心不触发");
    }
}
