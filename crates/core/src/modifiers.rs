//! 修饰键状态。
//!
//! 键盘钩子运行期间按事件自行跟踪按下的修饰键，判定热键时读这份位集；
//! 钩子只在存在「不传递」热键时才安装（见 [`crate::input_hooks`]），
//! 未安装时回退 `GetAsyncKeyState`。

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

/// 按系统即时状态取一份位集，用于播种与校准。
fn snapshot_tracked() -> u32 {
    let down = |vk: u16| unsafe { (GetAsyncKeyState(i32::from(vk)) as u16 & 0x8000) != 0 };
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
        if down(vk) {
            t |= bit;
        }
    }
    t
}

/// 当前按下的修饰键位掩码，取自系统即时状态。
pub fn pressed_modifiers() -> u32 {
    mask_of(snapshot_tracked())
}

/// 判定热键时该用的修饰键掩码：键盘钩子在跑就用自跟踪位集，否则回退即时状态。
pub fn current_modifiers() -> u32 {
    if TRACKING.load(Relaxed) {
        mask_of(TRACKED.load(Relaxed))
    } else {
        pressed_modifiers()
    }
}

/// 钩子收到一条按键事件后更新位集；注入的按键同样计入。
pub fn note_key(vk: u16, down: bool) {
    let Some(bit) = bit_of(vk) else { return };
    if down {
        TRACKED.fetch_or(bit, Relaxed);
    } else {
        TRACKED.fetch_and(!bit, Relaxed);
    }
}

/// 开始自跟踪，并按系统即时状态播种初值。
pub fn begin_tracking() {
    TRACKED.store(snapshot_tracked(), Relaxed);
    TRACKING.store(true, Relaxed);
}

/// 停止自跟踪，回退到系统即时状态。
pub fn end_tracking() {
    TRACKING.store(false, Relaxed);
    TRACKED.store(0, Relaxed);
}

/// 重新按系统即时状态校准位集；自跟踪未开启时不做任何事。
pub fn resync() {
    if TRACKING.load(Relaxed) {
        TRACKED.store(snapshot_tracked(), Relaxed);
    }
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
