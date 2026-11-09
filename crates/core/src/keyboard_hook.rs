//! 低级键盘钩子：承载所有开启「低级键盘钩子」的热键。
//!
//! `RegisterHotKey` 只收「修饰键 + 单个主键」，且吞不掉修饰键的按下 / 抬起。
//! 改用 `WH_KEYBOARD_LL` 后既能表达纯修饰键与多主键组合，也能按热键各自的
//! 「不传递」开关决定要不要吞掉按键，再把触发转发给代理窗口。
//! 直接读取 Raw Input 的程序不经过本钩子。

use std::sync::Mutex;
use std::sync::atomic::{AtomicIsize, AtomicU64, Ordering::Relaxed};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, PostMessageW, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use windows::core::PCWSTR;

use crate::hotkey::ParsedHotkey;
use crate::modifiers::{self, current_modifiers, is_modifier};

/// 拦截热键命中时发给代理窗口的消息；`wparam` 为热键 id（与 `WM_HOTKEY` 一致）。
pub const WM_KEY_TRIGGER: u32 = WM_APP + 5;

static HWND_RAW: AtomicIsize = AtomicIsize::new(0);
/// 最近一次按键事件的时刻。
static LAST_EVENT: AtomicU64 = AtomicU64::new(0);
/// 累计按键事件数，供看门狗判断钩子是否还在派发。
static KEY_EVENTS: AtomicU64 = AtomicU64::new(0);
/// 失配日志节流。
static MISMATCH_LOG_MS: AtomicU64 = AtomicU64::new(0);
static HOTKEYS: Mutex<Vec<HookHotkey>> = Mutex::new(Vec::new());

/// 距上一条按键事件多久；从未收到过时返回 `u64::MAX`。
pub fn millis_since_last_event() -> u64 {
    let last = LAST_EVENT.load(Relaxed);
    if last == 0 {
        return u64::MAX;
    }
    unsafe { GetTickCount64() }.saturating_sub(last)
}

pub fn key_event_count() -> u64 {
    KEY_EVENTS.load(Relaxed)
}

/// 一条由钩子承载的热键与它的运行态。
#[derive(Debug, Clone, PartialEq, Eq)]
struct HookHotkey {
    id: i32,
    modifiers: u32,
    /// 主键；为空表示纯修饰键热键。
    keys: Vec<u16>,
    /// 命中时吞掉按键，不传给前台程序。
    swallow: bool,
    /// 第 i 位：`keys[i]` 当前被按住。
    held: u8,
    /// 第 i 位：`keys[i]` 的按下被吞掉了，抬起也须吞掉。
    eaten: u8,
    /// 本轮已触发，松开全部主键前不再重复触发。
    fired: bool,
    /// 纯修饰键：修饰键已按齐。
    armed: bool,
    /// 纯修饰键：本轮按过别的键或多按了修饰键，作废。
    poisoned: bool,
}

/// 钩子对一条键盘事件的处置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    /// 与热键无关，放行给下一个钩子。
    Pass,
    /// 属于某条热键但不触发（重复按下 / 抬起），吞掉。
    Swallow,
    /// 命中热键：向代理窗口转发该 id；`swallow` 决定是否同时吞掉本次按键。
    Fire { id: i32, swallow: bool },
}

impl HookHotkey {
    fn new(id: i32, hk: &ParsedHotkey, swallow: bool) -> Self {
        Self {
            id,
            modifiers: hk.modifiers,
            keys: hk.keys.clone(),
            swallow,
            held: 0,
            eaten: 0,
            fired: false,
            armed: false,
            poisoned: false,
        }
    }

    /// 推进带主键的热键，返回 `(是否触发, 是否吞掉本次按键)`。
    /// 与 `RegisterHotKey` 对齐：修饰键须完全吻合，长按重复触发只算一次。
    fn step_keys(&mut self, down: bool, vk: u16, pressed: u32) -> (bool, bool) {
        let Some(i) = self.keys.iter().position(|k| *k == vk) else {
            return (false, false);
        };
        let bit = 1u8 << i;

        if !down {
            self.held &= !bit;
            let swallow = self.eaten & bit != 0;
            self.eaten &= !bit;
            if self.held == 0 {
                self.fired = false;
            }
            return (false, swallow);
        }

        let all = (1u8 << self.keys.len()) - 1;
        let matched = pressed == self.modifiers;
        self.held |= bit;
        let fired = matched && self.held == all && !self.fired;
        if fired {
            self.fired = true;
        }
        // 修饰键吻合时的成员键、以及本轮触发后的后续按下，都算这条热键的按键。
        let swallow = self.swallow && (matched || self.fired);
        if swallow {
            self.eaten |= bit;
        }
        (fired, swallow)
    }

    /// 推进纯修饰键热键，返回是否触发。修饰键早已传给前台，故从不吞键。
    /// 按齐后不立即触发，等全部修饰键松开、且期间没按过别的键。
    fn step_modifiers_only(&mut self, down: bool, modifier: bool, pressed: u32) -> bool {
        if down {
            if !modifier {
                // 按住修饰键期间敲了主键，说明这是别的快捷键，本轮作废。
                self.poisoned |= pressed != 0;
            } else if pressed == self.modifiers {
                self.armed = true;
            } else if pressed & !self.modifiers != 0 {
                self.poisoned = true;
            }
            return false;
        }
        if !modifier || pressed != 0 {
            return false;
        }
        let fired = self.armed && !self.poisoned;
        self.armed = false;
        self.poisoned = false;
        fired
    }
}

/// 纯逻辑判定，`pressed` 为本次事件之后按下的修饰键位掩码。
fn decide(msg: u32, vk: u16, pressed: u32, states: &mut [HookHotkey]) -> Decision {
    let down = matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN);
    let up = matches!(msg, WM_KEYUP | WM_SYSKEYUP);
    if !down && !up {
        return Decision::Pass;
    }
    let modifier = is_modifier(vk);

    // 每条热键都要推进状态，命中与吞键的结论再合并。
    let mut fired: Option<i32> = None;
    let mut swallow = false;
    for st in states.iter_mut() {
        if st.keys.is_empty() {
            if st.step_modifiers_only(down, modifier, pressed) {
                fired.get_or_insert(st.id);
            }
        } else {
            let (hit, eat) = st.step_keys(down, vk, pressed);
            swallow |= eat;
            if hit {
                fired.get_or_insert(st.id);
            }
        }
    }

    match fired {
        Some(id) => Decision::Fire { id, swallow },
        None if swallow => Decision::Swallow,
        None => Decision::Pass,
    }
}

/// 设置当前由钩子承载的热键集合（覆盖式，清空运行态）。
/// 每项为 `(热键 id, 组合, 是否吞掉按键)`。
pub fn set_hotkeys(list: &[(i32, ParsedHotkey, bool)]) {
    if let Ok(mut hotkeys) = HOTKEYS.lock() {
        *hotkeys = list
            .iter()
            .map(|(id, hk, swallow)| HookHotkey::new(*id, hk, *swallow))
            .collect();
    }
}

fn clear_states(states: &mut [HookHotkey]) {
    for st in states.iter_mut() {
        st.held = 0;
        st.eaten = 0;
        st.fired = false;
        st.armed = false;
        st.poisoned = false;
    }
}

/// 无条件清空运行态：睡眠恢复、会话解锁后此前的按住状态作废。
pub fn reset_states() {
    if let Ok(mut states) = HOTKEYS.lock() {
        clear_states(&mut states);
    }
}

/// 彻底空闲时清空运行态。长按会产生自动重复事件，这么久没事件说明没键按着，
/// 残留的 `held` / `fired` 只能来自丢失的抬起事件。
pub fn expire_idle_states(max_idle_ms: u64) {
    let last = LAST_EVENT.load(Relaxed);
    if last == 0 || unsafe { GetTickCount64() }.saturating_sub(last) <= max_idle_ms {
        return;
    }
    reset_states();
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
    let mut swallow = false;
    if ncode == 0 {
        let data = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let msg = wparam.0 as u32;
        let down = matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN);
        let up = matches!(msg, WM_KEYUP | WM_SYSKEYUP);
        // 注入的按键既不计入修饰键，也不参与判定与重装，避免反馈回路。
        let injected = data.flags.0 & LLKHF_INJECTED.0 != 0;
        if down || up {
            LAST_EVENT.store(unsafe { GetTickCount64() }, Relaxed);
            KEY_EVENTS.fetch_add(1, Relaxed);
        }
        // 只按事件跟踪，不用 GetAsyncKeyState 校准：被吞掉的按键不会更新系统状态。
        if down {
            modifiers::note_key(data.vkCode as u16, true);
        } else if up {
            modifiers::note_key(data.vkCode as u16, false);
        }

        if !injected && (down || up) {
            if let Ok(mut states) = HOTKEYS.lock() {
                let pressed = current_modifiers();
                // 主键按下时先做一次凭证据的修正：多出来的修饰键是残位就清掉。
                let pressed = if down {
                    heal_extra_modifiers(&states, data.vkCode as u16, pressed)
                } else {
                    pressed
                };
                match decide(msg, data.vkCode as u16, pressed, &mut states) {
                    Decision::Pass => {
                        if down {
                            log_modifier_mismatch(&states, data.vkCode as u16, pressed);
                        }
                    }
                    Decision::Swallow => swallow = true,
                    Decision::Fire { id, swallow: also } => {
                        post(id);
                        swallow |= also;
                    }
                }
            }

            // 抢回链首：对手通常在这次按键沿链传到它之后才把自己重装到最前。这里只投
            // 一条消息，等钩子线程处理时再重装，正好排在对手那次重装之后。
            crate::input_hooks::InputHooks::request_rearm();
        }
    }
    if swallow {
        return LRESULT(1);
    }
    unsafe { CallNextHookEx(None, ncode, wparam, lparam) }
}

/// 主键按下时清掉多出来的修饰键，返回修正后的掩码。掩码包含某条用该主键的热键要求的
/// 全部修饰键、却又多出别的——多出来的只能是丢失抬起事件留下的残位。
fn heal_extra_modifiers(states: &[HookHotkey], vk: u16, pressed: u32) -> u32 {
    let expected = states
        .iter()
        .filter(|st| st.keys.contains(&vk))
        .map(|st| st.modifiers)
        .find(|m| *m != pressed && pressed & *m == *m);
    let Some(expected) = expected else {
        return pressed;
    };
    let extra = pressed & !expected;
    if modifiers::clear_mask(extra) == 0 {
        return pressed;
    }
    crate::logging::debug(&format!(
        "修正修饰键残位 0x{extra:X}（期望 0x{expected:X}，实际 0x{pressed:X}）"
    ));
    current_modifiers()
}

/// 修正后仍对不上时记一条（每 2 秒最多一次）：缺的是必需修饰键，说明按下没到钩子。
fn log_modifier_mismatch(states: &[HookHotkey], vk: u16, pressed: u32) {
    let Some(expected) = states
        .iter()
        .find(|st| st.keys.contains(&vk) && st.modifiers != pressed)
        .map(|st| st.modifiers)
    else {
        return;
    };
    // 一个修饰键都没按（比如单纯打字打到这个字母）不算异常，不记。
    if pressed == 0 {
        return;
    }
    let now = unsafe { GetTickCount64() };
    let last = MISMATCH_LOG_MS.swap(now, Relaxed);
    if now.saturating_sub(last) < 2_000 {
        return;
    }
    crate::logging::debug(&format!(
        "热键主键 0x{vk:02X} 的修饰键不匹配：期望 0x{expected:X}，实际 0x{pressed:X}（缺的修饰键没到钩子）"
    ));
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
            modifiers::begin_tracking();
            // 静默时长从装钩子算起，否则「还没见过按键」会被当成静默很久，启动时误重建。
            LAST_EVENT.store(GetTickCount64(), Relaxed);
            Some(KeyboardHook { handle })
        }
    }
}

impl Drop for KeyboardHook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWindowsHookEx(self.handle);
        }
        modifiers::end_tracking();
        HWND_RAW.store(0, Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkey::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

    const VK_Q: u16 = 0x51;
    const VK_W: u16 = 0x57;
    const VK_S: u16 = 0x53;
    const VK_ESC: u16 = 0x1B;
    const VK_LCONTROL: u16 = 0xA2;
    const VK_LSHIFT: u16 = 0xA0;
    const VK_LMENU: u16 = 0xA4;

    fn hotkey(id: i32, modifiers: u32, keys: &[u16], swallow: bool) -> HookHotkey {
        HookHotkey::new(
            id,
            &ParsedHotkey {
                modifiers,
                keys: keys.to_vec(),
            },
            swallow,
        )
    }

    fn fire(id: i32) -> Decision {
        Decision::Fire { id, swallow: true }
    }

    /// Ctrl+Q 隐藏（id=1）、Win+Esc 关闭（id=2），两条都吞键。
    fn states() -> Vec<HookHotkey> {
        vec![
            hotkey(1, MOD_CONTROL, &[VK_Q], true),
            hotkey(2, MOD_WIN, &[VK_ESC], true),
        ]
    }

    #[test]
    fn matching_combo_fires_and_swallows() {
        let mut s = states();
        assert_eq!(decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s), fire(1));
        assert_eq!(decide(WM_KEYDOWN, VK_ESC, MOD_WIN, &mut s), fire(2));
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
        assert_eq!(decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s), fire(1));
        // 长按产生的重复按下：吞掉但不重复触发。
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
        assert_eq!(decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s), fire(1));
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
            fire(1),
            "松开后再按可再次触发"
        );
    }

    #[test]
    fn held_key_is_swallowed_even_if_modifiers_released_early() {
        let mut s = states();
        assert_eq!(decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s), fire(1));
        // 先松 Ctrl 再长按 Q，按下仍被吞掉直到 Q 抬起。
        assert_eq!(decide(WM_KEYDOWN, VK_Q, 0, &mut s), Decision::Swallow);
        assert_eq!(decide(WM_KEYUP, VK_Q, 0, &mut s), Decision::Swallow);
        assert_eq!(decide(WM_KEYDOWN, VK_Q, 0, &mut s), Decision::Pass);
    }

    #[test]
    fn syskey_messages_are_recognised() {
        // Alt 组合键走 WM_SYSKEYDOWN / WM_SYSKEYUP。
        let mut s = vec![hotkey(1, MOD_ALT, &[VK_Q], true)];
        assert_eq!(decide(WM_SYSKEYDOWN, VK_Q, MOD_ALT, &mut s), fire(1));
        assert_eq!(
            decide(WM_SYSKEYUP, VK_Q, MOD_ALT, &mut s),
            Decision::Swallow
        );
    }

    #[test]
    fn a_hotkey_without_swallow_fires_but_lets_the_key_through() {
        let mut s = vec![hotkey(1, MOD_CONTROL, &[VK_Q], false)];
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, MOD_CONTROL, &mut s),
            Decision::Fire {
                id: 1,
                swallow: false
            }
        );
        assert_eq!(
            decide(WM_KEYUP, VK_Q, MOD_CONTROL, &mut s),
            Decision::Pass,
            "没吞按下，抬起也不该吞"
        );
    }

    #[test]
    fn multi_key_combo_fires_only_once_all_keys_are_down() {
        let mut s = vec![hotkey(1, 0, &[VK_Q, VK_W], true)];
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, 0, &mut s),
            Decision::Swallow,
            "组合未按齐，不触发；吞键开着故先吞掉"
        );
        assert_eq!(decide(WM_KEYDOWN, VK_W, 0, &mut s), fire(1));
    }

    #[test]
    fn multi_key_combo_needs_its_modifiers_too() {
        let mut s = vec![hotkey(1, MOD_CONTROL, &[VK_Q, VK_W], true)];
        assert_eq!(decide(WM_KEYDOWN, VK_Q, 0, &mut s), Decision::Pass);
        assert_eq!(
            decide(WM_KEYDOWN, VK_W, 0, &mut s),
            Decision::Pass,
            "缺 Ctrl 不算命中"
        );
    }

    #[test]
    fn multi_key_combo_refires_after_releasing_every_key() {
        let mut s = vec![hotkey(1, 0, &[VK_Q, VK_W], false)];
        decide(WM_KEYDOWN, VK_Q, 0, &mut s);
        assert_eq!(
            decide(WM_KEYDOWN, VK_W, 0, &mut s),
            Decision::Fire {
                id: 1,
                swallow: false
            }
        );
        decide(WM_KEYUP, VK_W, 0, &mut s);
        assert_eq!(
            decide(WM_KEYDOWN, VK_W, 0, &mut s),
            Decision::Pass,
            "Q 仍按着，本轮已触发过，不重复触发"
        );
        decide(WM_KEYUP, VK_W, 0, &mut s);
        decide(WM_KEYUP, VK_Q, 0, &mut s);
        decide(WM_KEYDOWN, VK_Q, 0, &mut s);
        assert_eq!(
            decide(WM_KEYDOWN, VK_W, 0, &mut s),
            Decision::Fire {
                id: 1,
                swallow: false
            },
            "全部松开后可再次触发"
        );
    }

    /// 纯修饰键热键 Ctrl+Shift。修饰键从不吞掉。
    fn modifiers_only() -> Vec<HookHotkey> {
        vec![hotkey(1, MOD_CONTROL | MOD_SHIFT, &[], true)]
    }

    #[test]
    fn modifier_only_hotkey_fires_on_full_release() {
        let mut s = modifiers_only();
        assert_eq!(
            decide(WM_KEYDOWN, VK_LCONTROL, MOD_CONTROL, &mut s),
            Decision::Pass
        );
        assert_eq!(
            decide(WM_KEYDOWN, VK_LSHIFT, MOD_CONTROL | MOD_SHIFT, &mut s),
            Decision::Pass,
            "按齐时先不触发"
        );
        assert_eq!(
            decide(WM_KEYUP, VK_LSHIFT, MOD_CONTROL, &mut s),
            Decision::Pass
        );
        assert_eq!(
            decide(WM_KEYUP, VK_LCONTROL, 0, &mut s),
            Decision::Fire {
                id: 1,
                swallow: false
            },
            "全松开才触发，且不吞修饰键"
        );
    }

    #[test]
    fn modifier_only_hotkey_is_cancelled_by_another_key() {
        let mut s = modifiers_only();
        decide(WM_KEYDOWN, VK_LCONTROL, MOD_CONTROL, &mut s);
        decide(WM_KEYDOWN, VK_LSHIFT, MOD_CONTROL | MOD_SHIFT, &mut s);
        decide(WM_KEYDOWN, VK_S, MOD_CONTROL | MOD_SHIFT, &mut s);
        decide(WM_KEYUP, VK_S, MOD_CONTROL | MOD_SHIFT, &mut s);
        decide(WM_KEYUP, VK_LSHIFT, MOD_CONTROL, &mut s);
        assert_eq!(
            decide(WM_KEYUP, VK_LCONTROL, 0, &mut s),
            Decision::Pass,
            "Ctrl+Shift+S 是别的快捷键，不该触发"
        );
    }

    #[test]
    fn modifier_only_hotkey_is_cancelled_by_an_extra_modifier() {
        let mut s = modifiers_only();
        decide(WM_KEYDOWN, VK_LCONTROL, MOD_CONTROL, &mut s);
        decide(WM_KEYDOWN, VK_LSHIFT, MOD_CONTROL | MOD_SHIFT, &mut s);
        decide(
            WM_SYSKEYDOWN,
            VK_LMENU,
            MOD_CONTROL | MOD_SHIFT | MOD_ALT,
            &mut s,
        );
        decide(WM_SYSKEYUP, VK_LMENU, MOD_CONTROL | MOD_SHIFT, &mut s);
        decide(WM_KEYUP, VK_LSHIFT, MOD_CONTROL, &mut s);
        assert_eq!(decide(WM_KEYUP, VK_LCONTROL, 0, &mut s), Decision::Pass);
    }

    #[test]
    fn modifier_only_hotkey_recovers_after_a_cancelled_round() {
        let mut s = modifiers_only();
        decide(WM_KEYDOWN, VK_LCONTROL, MOD_CONTROL, &mut s);
        decide(WM_KEYDOWN, VK_S, MOD_CONTROL, &mut s);
        decide(WM_KEYUP, VK_S, MOD_CONTROL, &mut s);
        decide(WM_KEYUP, VK_LCONTROL, 0, &mut s);
        // 上一轮作废不该拖累下一轮。
        decide(WM_KEYDOWN, VK_LCONTROL, MOD_CONTROL, &mut s);
        decide(WM_KEYDOWN, VK_LSHIFT, MOD_CONTROL | MOD_SHIFT, &mut s);
        decide(WM_KEYUP, VK_LSHIFT, MOD_CONTROL, &mut s);
        assert_eq!(
            decide(WM_KEYUP, VK_LCONTROL, 0, &mut s),
            Decision::Fire {
                id: 1,
                swallow: false
            }
        );
    }

    #[test]
    fn typing_without_modifiers_does_not_poison_the_next_round() {
        let mut s = modifiers_only();
        decide(WM_KEYDOWN, VK_S, 0, &mut s);
        decide(WM_KEYUP, VK_S, 0, &mut s);
        decide(WM_KEYDOWN, VK_LCONTROL, MOD_CONTROL, &mut s);
        decide(WM_KEYDOWN, VK_LSHIFT, MOD_CONTROL | MOD_SHIFT, &mut s);
        decide(WM_KEYUP, VK_LSHIFT, MOD_CONTROL, &mut s);
        assert_eq!(
            decide(WM_KEYUP, VK_LCONTROL, 0, &mut s),
            Decision::Fire {
                id: 1,
                swallow: false
            }
        );
    }

    #[test]
    fn a_main_key_hotkey_and_a_modifier_only_hotkey_coexist() {
        let mut s = vec![
            hotkey(1, MOD_CONTROL | MOD_SHIFT, &[], false),
            hotkey(2, MOD_CONTROL | MOD_SHIFT, &[VK_Q], true),
        ];
        decide(WM_KEYDOWN, VK_LCONTROL, MOD_CONTROL, &mut s);
        decide(WM_KEYDOWN, VK_LSHIFT, MOD_CONTROL | MOD_SHIFT, &mut s);
        assert_eq!(
            decide(WM_KEYDOWN, VK_Q, MOD_CONTROL | MOD_SHIFT, &mut s),
            fire(2)
        );
        decide(WM_KEYUP, VK_Q, MOD_CONTROL | MOD_SHIFT, &mut s);
        decide(WM_KEYUP, VK_LSHIFT, MOD_CONTROL, &mut s);
        assert_eq!(
            decide(WM_KEYUP, VK_LCONTROL, 0, &mut s),
            Decision::Pass,
            "敲过主键，纯修饰键那条本轮作废"
        );
    }

    #[test]
    fn set_hotkeys_replaces_the_set_and_resets_state() {
        set_hotkeys(&[(
            7,
            ParsedHotkey {
                modifiers: MOD_CONTROL,
                keys: vec![VK_Q],
            },
            true,
        )]);
        {
            let hotkeys = HOTKEYS.lock().unwrap();
            assert_eq!(hotkeys.len(), 1);
            assert_eq!(hotkeys[0].id, 7);
            assert!(hotkeys[0].swallow);
            assert_eq!(hotkeys[0].held, 0);
        }
        set_hotkeys(&[]);
        assert!(HOTKEYS.lock().unwrap().is_empty());
    }
}
