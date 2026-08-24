//! 低级鼠标钩子：承载鼠标按键绑定与四角触发。
//!
//! 回调只做判定与 `PostMessageW`，重活都在代理窗口的消息处理里；
//! 由 [`crate::input_hooks`] 的专职线程安装。

use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, AtomicIsize, AtomicU32, AtomicU64, Ordering::Relaxed};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetSystemMetrics, HHOOK, MSLLHOOKSTRUCT, PostMessageW, SM_CXSCREEN,
    SM_CYSCREEN, SetWindowsHookExW, UnhookWindowsHookEx, WH_MOUSE_LL, WM_APP, WM_LBUTTONDOWN,
    WM_MBUTTONDOWN, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_XBUTTONDOWN,
};
use windows::core::PCWSTR;
use zonedeck_common::{MouseButton as MouseButtonCfg, Setting};

use crate::hotkey::parse_modifiers;
use crate::modifiers::current_modifiers;

pub const WM_MOUSE_TRIGGER: u32 = WM_APP + 3;
pub const TRIGGER_BUTTON: usize = 0;
pub const TRIGGER_CORNER: usize = 1;

const F_TL: u32 = 8;
const F_TR: u32 = 16;
const F_BL: u32 = 32;
const F_BR: u32 = 64;
const CORNER_MASK: u32 = F_TL | F_TR | F_BL | F_BR;
/// 只认「快速移动」角落。
const F_FAST: u32 = 128;

const CORNER_THRESHOLD: i32 = 10;
const CORNER_COOLDOWN_MS: u64 = 1000;
/// 移入角落的速度门槛（像素/毫秒），约 1500 px/s。
const FAST_SPEED_PX_PER_MS: f32 = 1.5;
/// 两次移动采样间隔超过此值则不算同一次。
const SPEED_SAMPLE_MAX_MS: u64 = 120;

/// 五颗键在 [`Buttons`] 数组里的下标，和 `MouseSetting` 的字段一一对应。
const BTN_LEFT: usize = 0;
const BTN_MIDDLE: usize = 1;
const BTN_RIGHT: usize = 2;
const BTN_SIDE1: usize = 3;
const BTN_SIDE2: usize = 4;
const BTN_COUNT: usize = 5;

static HWND_RAW: AtomicIsize = AtomicIsize::new(0);
static CORNER_FLAGS: AtomicU32 = AtomicU32::new(0);
static SCREEN_W: AtomicI32 = AtomicI32::new(0);
static SCREEN_H: AtomicI32 = AtomicI32::new(0);
static LAST_CORNER: AtomicU64 = AtomicU64::new(0);
/// 上一个鼠标位置与时刻，用来估算甩入角落的速度；时刻为 0 表示尚无采样。
static LAST_MOVE_POS: AtomicU64 = AtomicU64::new(0);
static LAST_MOVE_MS: AtomicU64 = AtomicU64::new(0);
static BUTTONS: Mutex<Buttons> = Mutex::new(Buttons::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ButtonState {
    enabled: bool,
    /// 需要连击几次（1..=3）。
    clicks: u8,
    /// 需要按住的修饰键位掩码；0 表示不要求修饰键。
    modifiers: u32,
    /// 当前已经连到第几下。
    count: u8,
    /// 上一次计入的点击时刻（`GetTickCount64`）。
    last_ms: u64,
}

impl ButtonState {
    const fn new() -> Self {
        Self {
            enabled: false,
            clicks: 1,
            modifiers: 0,
            count: 0,
            last_ms: 0,
        }
    }

    fn configure(&mut self, cfg: &MouseButtonCfg) {
        self.enabled = cfg.enabled;
        self.clicks = cfg.clicks.max(1);
        self.modifiers = parse_modifiers(&cfg.modifiers);
        self.count = 0;
        self.last_ms = 0;
    }
}

#[derive(Debug, Clone, Copy)]
struct Buttons {
    states: [ButtonState; BTN_COUNT],
    /// 连击判定窗口（毫秒）。
    window_ms: u64,
}

impl Buttons {
    const fn new() -> Self {
        Self {
            states: [ButtonState::new(); BTN_COUNT],
            window_ms: 400,
        }
    }
}

fn buttons_from(s: &Setting) -> Buttons {
    let mut b = Buttons::new();
    let cfgs = [
        &s.mouse.left,
        &s.mouse.middle,
        &s.mouse.right,
        &s.mouse.side1,
        &s.mouse.side2,
    ];
    for (state, cfg) in b.states.iter_mut().zip(cfgs) {
        state.configure(cfg);
    }
    b.window_ms = u64::from(s.mouse.multi_click_ms);
    b
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MoveSample {
    x: i32,
    y: i32,
    ms: u64,
}

/// 把坐标打包进一个 `u64`：高 32 位 x、低 32 位 y。
fn pack_pos(x: i32, y: i32) -> u64 {
    ((x as u32 as u64) << 32) | y as u32 as u64
}

fn unpack_pos(v: u64) -> (i32, i32) {
    ((v >> 32) as u32 as i32, v as u32 as i32)
}

/// 取出上一次采样并换上这一次；无上一次采样时返回 None。
fn swap_last_move(sample: MoveSample) -> Option<MoveSample> {
    let ms = LAST_MOVE_MS.swap(sample.ms, Relaxed);
    let pos = LAST_MOVE_POS.swap(pack_pos(sample.x, sample.y), Relaxed);
    if ms == 0 {
        return None;
    }
    let (x, y) = unpack_pos(pos);
    Some(MoveSample { x, y, ms })
}

/// 这一步移动是否算「甩」：位移 / 用时超过速度门槛，且采样间隔有效。
fn is_fast_move(from: MoveSample, to: MoveSample) -> bool {
    let dt = to.ms.saturating_sub(from.ms);
    if dt == 0 || dt > SPEED_SAMPLE_MAX_MS {
        return false;
    }
    let dx = (to.x - from.x) as f32;
    let dy = (to.y - from.y) as f32;
    (dx * dx + dy * dy).sqrt() / dt as f32 >= FAST_SPEED_PX_PER_MS
}

fn corner_flags_from(s: &Setting) -> u32 {
    let mut f = 0;
    if s.corner_fast_only {
        f |= F_FAST;
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
    s.mouse.any_enabled() || corner_flags_from(s) & CORNER_MASK != 0
}

pub fn set_flags(s: &Setting) {
    CORNER_FLAGS.store(corner_flags_from(s), Relaxed);
    if let Ok(mut b) = BUTTONS.lock() {
        *b = buttons_from(s);
    }
}

/// 记一次点击：窗口内累加、超时重置。数满 `clicks` 下即触发并清零。
fn register_click(state: &mut ButtonState, now: u64, window_ms: u64) -> bool {
    if state.count > 0 && now.wrapping_sub(state.last_ms) <= window_ms {
        state.count += 1;
    } else {
        state.count = 1;
    }
    state.last_ms = now;
    if state.count >= state.clicks {
        state.count = 0;
        return true;
    }
    false
}

/// 鼠标消息 → 按键下标；不关心的消息返回 None。
fn button_index(msg: u32, mouse_data: u32) -> Option<usize> {
    match msg {
        WM_LBUTTONDOWN => Some(BTN_LEFT),
        WM_MBUTTONDOWN => Some(BTN_MIDDLE),
        WM_RBUTTONDOWN => Some(BTN_RIGHT),
        WM_XBUTTONDOWN => match (mouse_data >> 16) as u16 {
            1 => Some(BTN_SIDE1),
            2 => Some(BTN_SIDE2),
            _ => None,
        },
        _ => None,
    }
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
    if msg == WM_MOUSEMOVE {
        let flags = CORNER_FLAGS.load(Relaxed);
        if flags & CORNER_MASK == 0 {
            return;
        }
        let now = unsafe { GetTickCount64() };
        let sample = MoveSample {
            x: data.pt.x,
            y: data.pt.y,
            ms: now,
        };
        // 须在早退之前记下采样，供下次算速度。
        let previous = swap_last_move(sample);

        if now.wrapping_sub(LAST_CORNER.load(Relaxed)) < CORNER_COOLDOWN_MS {
            return;
        }
        let (w, h) = (SCREEN_W.load(Relaxed), SCREEN_H.load(Relaxed));
        if !corner_hit(sample.x, sample.y, w, h, flags) {
            return;
        }
        if flags & F_FAST != 0 && !previous.is_some_and(|p| is_fast_move(p, sample)) {
            return;
        }
        LAST_CORNER.store(now, Relaxed);
        post(TRIGGER_CORNER);
        return;
    }

    let Some(idx) = button_index(msg, data.mouseData) else {
        return;
    };
    let Ok(mut buttons) = BUTTONS.lock() else {
        return;
    };
    let window_ms = buttons.window_ms;
    let state = &mut buttons.states[idx];
    if !state.enabled {
        return;
    }
    // 要求修饰键时必须完全吻合。
    if state.modifiers != 0 && current_modifiers() != state.modifiers {
        state.count = 0;
        return;
    }
    let now = unsafe { GetTickCount64() };
    let fire = register_click(state, now, window_ms);
    drop(buttons);
    if fire {
        post(TRIGGER_BUTTON);
    }
}

unsafe extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if ncode == 0 {
            let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            handle_event(wparam.0 as u32, data);
        }
        // 只观察，不吞事件。
        CallNextHookEx(None, ncode, wparam, lparam)
    }
}

pub struct MouseHook {
    handle: HHOOK,
}

impl MouseHook {
    /// 安装钩子；须在 [`crate::input_hooks`] 的专职线程上调用。
    /// 触发条件由 [`set_flags`] 单独设置。
    pub fn install(agent_hwnd: HWND) -> Option<MouseHook> {
        HWND_RAW.store(agent_hwnd.0 as isize, Relaxed);
        // 上一轮的采样已过时。
        LAST_MOVE_MS.store(0, Relaxed);
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
    use crate::hotkey::{MOD_CONTROL, MOD_SHIFT};

    /// 关掉默认开启的中键。
    fn setting_with(mutate: impl FnOnce(&mut Setting)) -> Setting {
        let mut s = Setting::default();
        s.mouse.middle.enabled = false;
        mutate(&mut s);
        s
    }

    /// 已启用、需要 `clicks` 连击、无修饰键的按键状态。
    fn armed(clicks: u8) -> ButtonState {
        ButtonState {
            enabled: true,
            clicks,
            ..ButtonState::new()
        }
    }

    #[test]
    fn wants_hook_only_when_a_trigger_is_enabled() {
        assert!(!wants_hook(&setting_with(|_| {})));
        assert!(
            wants_hook(&Setting::default()),
            "默认开着中键，装钩子是应该的"
        );
        assert!(wants_hook(&setting_with(|s| s.mouse.left.enabled = true)));
        assert!(wants_hook(&setting_with(|s| s.bottom_right_hide = true)));
        assert!(
            !wants_hook(&setting_with(|s| s.allow_move_restore = true)),
            "仅开启移动恢复不应安装钩子"
        );
    }

    #[test]
    fn buttons_map_each_setting_field() {
        let s = setting_with(|s| {
            s.mouse.right.enabled = true;
            s.mouse.right.clicks = 3;
            s.mouse.right.modifiers = "Ctrl+Shift".into();
            s.mouse.multi_click_ms = 250;
        });
        let b = buttons_from(&s);
        assert!(b.states[BTN_RIGHT].enabled);
        assert_eq!(b.states[BTN_RIGHT].clicks, 3);
        assert_eq!(b.states[BTN_RIGHT].modifiers, MOD_CONTROL | MOD_SHIFT);
        assert!(!b.states[BTN_LEFT].enabled);
        assert_eq!(b.window_ms, 250);
    }

    #[test]
    fn button_index_maps_messages_and_xbuttons() {
        assert_eq!(button_index(WM_LBUTTONDOWN, 0), Some(BTN_LEFT));
        assert_eq!(button_index(WM_MBUTTONDOWN, 0), Some(BTN_MIDDLE));
        assert_eq!(button_index(WM_RBUTTONDOWN, 0), Some(BTN_RIGHT));
        assert_eq!(button_index(WM_XBUTTONDOWN, 1 << 16), Some(BTN_SIDE1));
        assert_eq!(button_index(WM_XBUTTONDOWN, 2 << 16), Some(BTN_SIDE2));
        assert_eq!(button_index(WM_XBUTTONDOWN, 3 << 16), None);
        assert_eq!(button_index(WM_MOUSEMOVE, 0), None);
    }

    #[test]
    fn single_click_fires_every_time() {
        let mut st = armed(1);
        assert!(register_click(&mut st, 100, 400));
        assert!(register_click(&mut st, 5_000, 400), "单击不受窗口影响");
    }

    #[test]
    fn triple_click_needs_three_clicks_inside_the_window() {
        let mut st = armed(3);
        assert!(!register_click(&mut st, 1000, 400));
        assert!(!register_click(&mut st, 1300, 400));
        assert!(register_click(&mut st, 1600, 400), "窗口内第三下触发");
        assert!(!register_click(&mut st, 1700, 400));
    }

    #[test]
    fn slow_clicks_never_reach_the_third() {
        let mut st = armed(3);
        for t in [1000, 1500, 2000, 2500] {
            assert!(!register_click(&mut st, t, 400), "间隔超窗口不该触发");
        }
    }

    #[test]
    fn a_late_click_restarts_the_streak() {
        let mut st = armed(3);
        assert!(!register_click(&mut st, 1000, 400));
        assert!(!register_click(&mut st, 2000, 400));
        assert!(!register_click(&mut st, 2200, 400));
        assert!(register_click(&mut st, 2400, 400), "第 3 下才触发");
    }

    fn sample(x: i32, y: i32, ms: u64) -> MoveSample {
        MoveSample { x, y, ms }
    }

    #[test]
    fn fast_flick_is_recognised_slow_drift_is_not() {
        assert!(is_fast_move(
            sample(1700, 900, 1000),
            sample(1900, 1070, 1020)
        ));
        assert!(!is_fast_move(
            sample(1870, 1050, 1000),
            sample(1900, 1070, 1200)
        ));
    }

    #[test]
    fn positions_survive_the_atomic_round_trip() {
        // 主屏左侧 / 上方的显示器坐标为负。
        for (x, y) in [(0, 0), (1919, 1079), (-1920, -300), (i32::MIN, i32::MAX)] {
            assert_eq!(unpack_pos(pack_pos(x, y)), (x, y));
        }
    }

    #[test]
    fn swapping_yields_the_previous_sample_and_nothing_on_the_first() {
        LAST_MOVE_MS.store(0, Relaxed);
        assert_eq!(
            swap_last_move(sample(-5, 1070, 1000)),
            None,
            "首次采样没有上一次"
        );
        assert_eq!(
            swap_last_move(sample(1900, 3, 1200)),
            Some(sample(-5, 1070, 1000))
        );
        LAST_MOVE_MS.store(0, Relaxed);
    }

    #[test]
    fn stale_or_zero_interval_samples_are_not_trusted() {
        assert!(!is_fast_move(sample(0, 0, 1000), sample(1900, 1070, 1500)));
        assert!(!is_fast_move(sample(0, 0, 1000), sample(1900, 1070, 1000)));
    }

    #[test]
    fn corner_flags_carry_the_fast_only_bit() {
        let s = setting_with(|s| {
            s.top_left_hide = true;
            s.corner_fast_only = false;
        });
        assert_eq!(corner_flags_from(&s) & F_FAST, 0);
        assert!(
            corner_flags_from(&Setting::default()) & F_FAST != 0,
            "默认只认快速移动"
        );
        assert!(
            !wants_hook(&setting_with(|s| s.corner_fast_only = true)),
            "只开「仅快速移动」而没选角落、也没开按键，不该装钩子"
        );
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
