//! 修饰键状态。
//!
//! 有键盘钩子时按事件跟踪按住的修饰键；没有钩子时回退 `GetAsyncKeyState`。
//! 事件跟踪不能拿系统状态校准（被吞掉的按键不会更新系统状态），也不按时间清理——
//! 按住修饰键停顿再按主键是常见操作。残位只由确定时机重置（唤醒、会话解锁）或
//! 有证据的当场修正（见 [`clear_mask`]）处理。

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL,
    VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
};

use crate::hotkey::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

/// [`TRACKED`] 里各具体修饰键占的位；左右分开跟踪。
mod bits {
    pub const LCTRL: u32 = 1 << 0;
    pub const RCTRL: u32 = 1 << 1;
    pub const LALT: u32 = 1 << 2;
    pub const RALT: u32 = 1 << 3;
    pub const LSHIFT: u32 = 1 << 4;
    pub const RSHIFT: u32 = 1 << 5;
    pub const LWIN: u32 = 1 << 6;
    pub const RWIN: u32 = 1 << 7;
    /// 不分左右的通用码。
    pub const CTRL: u32 = 1 << 8;
    pub const ALT: u32 = 1 << 9;
    pub const SHIFT: u32 = 1 << 10;
}

/// 当前按住的具体修饰键位集，由 [`note_key`] 按事件维护。
static TRACKED: AtomicU32 = AtomicU32::new(0);
/// 键盘钩子是否在运行。false 时 [`current_modifiers`] 回退系统即时状态。
static TRACKING: AtomicBool = AtomicBool::new(false);
/// 只用系统状态播种过一次。重装钩子时若再播种，会把按住的修饰键抹掉。
static SEEDED: AtomicBool = AtomicBool::new(false);

/// 虚拟键码对应的位；不是修饰键则为 `None`。
fn bit_of(vk: u16) -> Option<u32> {
    Some(match vk {
        v if v == VK_LCONTROL.0 => bits::LCTRL,
        v if v == VK_RCONTROL.0 => bits::RCTRL,
        v if v == VK_LMENU.0 => bits::LALT,
        v if v == VK_RMENU.0 => bits::RALT,
        v if v == VK_LSHIFT.0 => bits::LSHIFT,
        v if v == VK_RSHIFT.0 => bits::RSHIFT,
        v if v == VK_LWIN.0 => bits::LWIN,
        v if v == VK_RWIN.0 => bits::RWIN,
        v if v == VK_CONTROL.0 => bits::CTRL,
        v if v == VK_MENU.0 => bits::ALT,
        v if v == VK_SHIFT.0 => bits::SHIFT,
        _ => return None,
    })
}

/// 该虚拟键码是不是修饰键。
pub fn is_modifier(vk: u16) -> bool {
    bit_of(vk).is_some()
}
/// 把具体键位集折算成 [`crate::hotkey`] 的修饰键掩码。
fn mask_of(tracked: u32) -> u32 {
    let has = |group: u32| tracked & group != 0;
    let mut m = 0;
    if has(bits::LCTRL | bits::RCTRL | bits::CTRL) {
        m |= MOD_CONTROL;
    }
    if has(bits::LALT | bits::RALT | bits::ALT) {
        m |= MOD_ALT;
    }
    if has(bits::LSHIFT | bits::RSHIFT | bits::SHIFT) {
        m |= MOD_SHIFT;
    }
    if has(bits::LWIN | bits::RWIN) {
        m |= MOD_WIN;
    }
    m
}

/// 某个虚拟键此刻是否按着。只用于播种：被吞键时这里读到的是「没按下」。
fn is_key_down(vk: u16) -> bool {
    unsafe { (GetAsyncKeyState(i32::from(vk)) as u16 & 0x8000) != 0 }
}

/// 系统状态对应的位集。
fn snapshot_tracked() -> u32 {
    let mut t = 0;
    for (vk, bit) in [
        (VK_LCONTROL.0, bits::LCTRL),
        (VK_RCONTROL.0, bits::RCTRL),
        (VK_LMENU.0, bits::LALT),
        (VK_RMENU.0, bits::RALT),
        (VK_LSHIFT.0, bits::LSHIFT),
        (VK_RSHIFT.0, bits::RSHIFT),
        (VK_LWIN.0, bits::LWIN),
        (VK_RWIN.0, bits::RWIN),
    ] {
        if is_key_down(vk) {
            t |= bit;
        }
    }
    t
}

pub fn pressed_modifiers() -> u32 {
    mask_of(snapshot_tracked())
}

/// 判定热键用的掩码：钩子在跑就用跟踪位集，否则回退系统状态。
pub fn current_modifiers() -> u32 {
    if TRACKING.load(Relaxed) {
        mask_of(TRACKED.load(Relaxed))
    } else {
        pressed_modifiers()
    }
}

pub fn note_key(vk: u16, down: bool) {
    let Some(bit) = bit_of(vk) else { return };
    if down {
        TRACKED.fetch_or(bit, Relaxed);
    } else {
        TRACKED.fetch_and(!bit, Relaxed);
    }
}

/// 丢掉 `extra` 覆盖到的位，返回清掉的位集；只在有证据时调用。
pub fn clear_mask(extra: u32) -> u32 {
    let mut tracked = TRACKED.load(Relaxed);
    let before = tracked;
    for (bit, group) in [
        (bits::LCTRL, MOD_CONTROL),
        (bits::RCTRL, MOD_CONTROL),
        (bits::CTRL, MOD_CONTROL),
        (bits::LALT, MOD_ALT),
        (bits::RALT, MOD_ALT),
        (bits::ALT, MOD_ALT),
        (bits::LSHIFT, MOD_SHIFT),
        (bits::RSHIFT, MOD_SHIFT),
        (bits::SHIFT, MOD_SHIFT),
        (bits::LWIN, MOD_WIN),
        (bits::RWIN, MOD_WIN),
    ] {
        if extra & group != 0 {
            tracked &= !bit;
        }
    }
    if tracked != before {
        TRACKED.store(tracked, Relaxed);
    }
    before & !tracked
}

/// 开始自跟踪；只有第一次装钩子时播种，重装沿用已有位集。
pub fn begin_tracking() {
    TRACKING.store(true, Relaxed);
    if SEEDED.swap(true, Relaxed) {
        return;
    }
    TRACKED.store(snapshot_tracked(), Relaxed);
}

/// 停止自跟踪，保留位集。
pub fn end_tracking() {
    TRACKING.store(false, Relaxed);
}

/// 按系统状态重新播种，只在确定没人按键的时刻调用。
pub fn reset_tracking() {
    TRACKED.store(snapshot_tracked(), Relaxed);
    SEEDED.store(true, Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        TRACKED.store(0, Relaxed);
    }

    fn mask() -> u32 {
        mask_of(TRACKED.load(Relaxed))
    }

    #[test]
    fn a_pressed_modifier_shows_up_in_the_mask() {
        reset();
        note_key(VK_LCONTROL.0, true);
        assert_eq!(mask(), MOD_CONTROL);
        note_key(VK_LCONTROL.0, false);
        assert_eq!(mask(), 0);
    }

    #[test]
    fn left_and_right_are_tracked_separately() {
        reset();
        note_key(VK_LCONTROL.0, true);
        note_key(VK_RCONTROL.0, true);
        note_key(VK_LCONTROL.0, false);
        assert_eq!(mask(), MOD_CONTROL, "右 Ctrl 还按着");
        note_key(VK_RCONTROL.0, false);
        assert_eq!(mask(), 0);
    }

    #[test]
    fn generic_codes_are_accepted_alongside_sided_ones() {
        reset();
        note_key(VK_CONTROL.0, true);
        assert_eq!(mask(), MOD_CONTROL);
        note_key(VK_CONTROL.0, false);
        assert_eq!(mask(), 0);
    }

    #[test]
    fn a_generic_release_does_not_clear_a_sided_press() {
        reset();
        note_key(VK_LCONTROL.0, true);
        note_key(VK_CONTROL.0, false);
        assert_eq!(mask(), MOD_CONTROL, "左 Ctrl 的按下不该被通用码的抬起抹掉");
    }

    #[test]
    fn repeated_downs_need_only_one_up() {
        reset();
        for _ in 0..5 {
            note_key(VK_LSHIFT.0, true);
        }
        assert_eq!(mask(), MOD_SHIFT);
        note_key(VK_LSHIFT.0, false);
        assert_eq!(mask(), 0);
    }

    #[test]
    fn combinations_accumulate() {
        reset();
        note_key(VK_LCONTROL.0, true);
        note_key(VK_LSHIFT.0, true);
        note_key(VK_LMENU.0, true);
        note_key(VK_LWIN.0, true);
        assert_eq!(mask(), MOD_CONTROL | MOD_SHIFT | MOD_ALT | MOD_WIN);
    }

    #[test]
    fn non_modifier_keys_leave_the_mask_alone() {
        reset();
        note_key(VK_LCONTROL.0, true);
        note_key(b'Q' as u16, true);
        note_key(b'Q' as u16, false);
        assert_eq!(mask(), MOD_CONTROL, "主键的按下抬起不影响修饰键位集");
    }

    #[test]
    fn win_key_has_no_generic_code_and_uses_both_sides() {
        reset();
        note_key(VK_LWIN.0, true);
        assert_eq!(mask(), MOD_WIN);
        note_key(VK_RWIN.0, true);
        note_key(VK_LWIN.0, false);
        assert_eq!(mask(), MOD_WIN, "右 Win 还按着");
    }
}
