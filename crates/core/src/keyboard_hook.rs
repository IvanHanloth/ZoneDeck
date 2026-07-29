//! 低级键盘钩子：承载开启「不传递」的热键。
//!
//! `RegisterHotKey` 虽然会吞掉主键，但修饰键的按下 / 抬起仍会到达前台程序。
//! 对开启「不传递」的热键改用 `WH_KEYBOARD_LL`：命中时吞掉主键的按下与抬起，
//! 不让它进入常规消息流，再把触发转发给代理窗口。
//! 直接读取 Raw Input 的程序不经过本钩子，仍可能观察到按键。

use std::sync::Mutex;
use std::sync::atomic::{AtomicIsize, Ordering::Relaxed};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, PostMessageW, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use windows::core::PCWSTR;

use crate::hotkey::ParsedHotkey;
use crate::util::pressed_modifiers;

/// 拦截热键命中时发给代理窗口的消息；`wparam` 为热键 id（与 `WM_HOTKEY` 一致）。
pub const WM_KEY_TRIGGER: u32 = WM_APP + 5;

static HWND_RAW: AtomicIsize = AtomicIsize::new(0);
static HOTKEYS: Mutex<Vec<InterceptState>> = Mutex::new(Vec::new());

/// 一条开启「不传递」的热键与它的按住状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InterceptState {
    id: i32,
    vk: u16,
    modifiers: u32,
    /// 主键当前被按住：按下已被吞掉，后续的重复按下与抬起也须吞掉。
    held: bool,
}

/// 钩子对一条键盘事件的处置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    /// 与拦截热键无关，放行给下一个钩子。
    Pass,
    /// 属于已按住的拦截热键（重复按下 / 抬起），吞掉但不再触发。
    Swallow,
    /// 命中热键：吞掉并向代理窗口转发该 id。
    Fire(i32),
}

/// 纯逻辑判定：`pressed` 为当前按下的修饰键位掩码。
///
/// 与 `RegisterHotKey` 的语义对齐：修饰键须完全吻合（多按不算），
/// 长按重复触发只算一次（对应 `MOD_NOREPEAT`）。
fn decide(msg: u32, vk: u16, pressed: u32, states: &mut [InterceptState]) -> Decision {
    let down = matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN);
    let up = matches!(msg, WM_KEYUP | WM_SYSKEYUP);

    if up {
        // 按下已被吞掉的话，抬起也须吞掉，否则前台会收到无配对的抬起事件。
        let mut was_held = false;
        for st in states.iter_mut().filter(|s| s.vk == vk) {
            was_held |= st.held;
            st.held = false;
        }
        return if was_held {
            Decision::Swallow
        } else {
            Decision::Pass
        };
    }
    if !down {
        return Decision::Pass;
    }

    let mut any_held = false;
    for st in states.iter_mut().filter(|s| s.vk == vk) {
        if st.held {
            any_held = true;
        } else if pressed == st.modifiers {
            st.held = true;
            return Decision::Fire(st.id);
        }
    }
    if any_held {
        Decision::Swallow
    } else {
        Decision::Pass
    }
}

/// 设置当前需要拦截的热键集合（覆盖式，清空按住状态）。
pub fn set_hotkeys(list: &[(i32, ParsedHotkey)]) {
    if let Ok(mut hotkeys) = HOTKEYS.lock() {
        *hotkeys = list
            .iter()
            .map(|(id, hk)| InterceptState {
                id: *id,
                vk: hk.vk,
                modifiers: hk.modifiers,
                held: false,
            })
            .collect();
    }
}

fn post(id: i32) {
    let raw = HWND_RAW.load(Relaxed);
    if raw == 0 {
        return;
    }
    unsafe {
        let hwnd = HWND(raw as *mut std::ffi::c_void);
        let _ = PostMessageW(Some(hwnd), WM_KEY_TRIGGER, WPARAM(id as usize), LPARAM(0));
    }
}

unsafe extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode == 0 {
        let data = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        // 注入的按键（自家的媒体键模拟等）不参与判定，避免反馈回路。
        if data.flags.0 & LLKHF_INJECTED.0 == 0 {
            // 在锁外取修饰键状态，锁内只剩纯内存判定。
            let pressed = pressed_modifiers();
            let decision = HOTKEYS
                .lock()
                .map(|mut states| decide(wparam.0 as u32, data.vkCode as u16, pressed, &mut states))
                .unwrap_or(Decision::Pass);
            match decision {
                Decision::Pass => {}
                Decision::Swallow => return LRESULT(1),
                Decision::Fire(id) => {
                    post(id);
                    return LRESULT(1);
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, ncode, wparam, lparam) }
}

pub struct KeyboardHook {
    handle: HHOOK,
}

impl KeyboardHook {
    /// 安装钩子；须在 [`crate::input_hooks`] 的专职线程上调用。
    pub fn install(agent_hwnd: HWND) -> Option<KeyboardHook> {
        HWND_RAW.store(agent_hwnd.0 as isize, Relaxed);
        unsafe {
            let hinstance = GetModuleHandleW(PCWSTR::null()).ok()?;
            let handle =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), Some(hinstance.into()), 0)
                    .ok()?;
            Some(KeyboardHook { handle })
        }
    }
}

impl Drop for KeyboardHook {
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
    use crate::hotkey::{MOD_CONTROL, MOD_SHIFT, MOD_WIN};

    const VK_Q: u16 = 0x51;
    const VK_ESC: u16 = 0x1B;

    /// Ctrl+Q 隐藏（id=1）、Win+Esc 关闭（id=2）的拦截集合。
    fn states() -> Vec<InterceptState> {
        vec![
            InterceptState {
                id: 1,
                vk: VK_Q,
                modifiers: MOD_CONTROL,
                held: false,
            },
            InterceptState {
                id: 2,
                vk: VK_ESC,
                modifiers: MOD_WIN,
                held: false,
            },
        ]
    }

    #[test]
    fn matching_combo_fires_and_swallows() {
        let mut s = states();
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s),
            Decision::Fire(1)
        );
        assert_eq!(
            decide(WM_KEYDOWN, VK_ESC, MOD_WIN, &mut s),
            Decision::Fire(2)
        );
    }

    #[test]
    fn modifiers_must_match_exactly() {
        let mut s = states();
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, 0, &mut s),
            Decision::Pass,
            "裸按 Q 不触发"
        );
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, MOD_CONTROL | MOD_SHIFT, &mut s),
            Decision::Pass,
            "多按修饰键不算命中，与 RegisterHotKey 一致"
        );
    }

    #[test]
    fn unrelated_keys_pass_through() {
        let mut s = states();
        assert_eq!(
            decide(WM_KEYDOWN, 0x41, MOD_CONTROL, &mut s),
            Decision::Pass
        );
        assert_eq!(decide(WM_KEYUP, 0x41, 0, &mut s), Decision::Pass);
    }

    #[test]
    fn holding_the_key_fires_only_once_but_keeps_swallowing() {
        let mut s = states();
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s),
            Decision::Fire(1)
        );
        // 长按产生的重复按下：吞掉但不重复触发（对应 MOD_NOREPEAT）。
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s),
            Decision::Swallow
        );
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s),
            Decision::Swallow
        );
    }

    #[test]
    fn keyup_after_fire_is_swallowed_then_state_resets() {
        let mut s = states();
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s),
            Decision::Fire(1)
        );
        assert_eq!(
            decide(WM_KEYUP, VK_Q, MOD_CONTROL, &mut s),
            Decision::Swallow,
            "按下被吞掉后抬起也须吞掉"
        );
        assert_eq!(
            decide(WM_KEYUP, VK_Q, MOD_CONTROL, &mut s),
            Decision::Pass,
            "状态已复位，再来的抬起与我们无关"
        );
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s),
            Decision::Fire(1),
            "松开后再按可再次触发"
        );
    }

    #[test]
    fn held_key_is_swallowed_even_if_modifiers_released_early() {
        let mut s = states();
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s),
            Decision::Fire(1)
        );
        // 先松 Ctrl 再长按 Q：按下仍被吞掉，直到 Q 抬起。
        assert_eq!(decide(WM_KEYDOWN, VK_Q, 0, &mut s), Decision::Swallow);
        assert_eq!(decide(WM_KEYUP, VK_Q, 0, &mut s), Decision::Swallow);
        assert_eq!(decide(WM_KEYDOWN, VK_Q, 0, &mut s), Decision::Pass);
    }

    #[test]
    fn syskey_messages_are_recognised() {
        // Alt 组合键走 WM_SYSKEYDOWN / WM_SYSKEYUP。
        let mut s = vec![InterceptState {
            id: 1,
            vk: VK_Q,
            modifiers: crate::hotkey::MOD_ALT,
            held: false,
        }];
        assert_eq!(
            decide(WM_SYSKEYDOWN, VK_Q, crate::hotkey::MOD_ALT, &mut s),
            Decision::Fire(1)
        );
        assert_eq!(
            decide(WM_SYSKEYUP, VK_Q, crate::hotkey::MOD_ALT, &mut s),
            Decision::Swallow
        );
    }

    #[test]
    fn set_hotkeys_replaces_the_set_and_resets_state() {
        set_hotkeys(&[(
            7,
            ParsedHotkey {
                modifiers: MOD_CONTROL,
                vk: VK_Q,
            },
        )]);
        {
            let hotkeys = HOTKEYS.lock().unwrap();
            assert_eq!(hotkeys.len(), 1);
            assert_eq!(hotkeys[0].id, 7);
            assert!(!hotkeys[0].held);
        }
        set_hotkeys(&[]);
        assert!(HOTKEYS.lock().unwrap().is_empty());
    }
}
