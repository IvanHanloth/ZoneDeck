//! 低级鼠标钩子：承载鼠标按键绑定与四角触发。
//!
//! 回调只做判定与 `PostMessageW`，重活都在代理窗口的消息处理里；
//! 由 [`crate::input_hooks`] 的专职线程安装。

use std::sync::Mutex;
use std::sync::atomic::{AtomicIsize, AtomicU32, AtomicU64, Ordering::Relaxed};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, MSLLHOOKSTRUCT, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    WH_MOUSE_LL, WM_APP, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEMOVE, WM_RBUTTONDOWN,
    WM_XBUTTONDOWN,
};
use windows::core::PCWSTR;
use zonedeck_common::{MouseButton as MouseButtonCfg, Setting};

use crate::hotkey::parse_modifiers;
use crate::modifiers::current_modifiers;
use crate::monitor::ScreenRect;

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

/// 光标是否落在所在显示器的某个被启用的角上。
///
/// `screen` 必须是**光标所在那块显示器**的矩形，不能是主显示器的宽高：钩子给的
/// 坐标是虚拟屏幕坐标，主屏左边的副屏上 x 恒为负，拿主屏宽高判会把副屏整条
/// 上下边缘都当成「左上 / 左下角」。
fn corner_hit(x: i32, y: i32, screen: ScreenRect, flags: u32) -> bool {
    // 落在这块显示器之外不算它的角：显示器非矩形排列时，光标可能停在两块之间的
    // 空隙里，而 `MONITOR_DEFAULTTONEAREST` 仍会给出最近的一块。
    if x < screen.left || x >= screen.right || y < screen.top || y >= screen.bottom {
        return false;
    }
    let t = CORNER_THRESHOLD;
    let left = x <= screen.left + t;
    let right = x >= screen.right - t;
    let top = y <= screen.top + t;
    let bottom = y >= screen.bottom - t;
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
        // 按光标所在的那块显示器判角；查不到就当没命中，别拿主屏尺寸硬套。
        let Some(screen) = crate::monitor::at_point(sample.x, sample.y).map(|m| m.bounds) else {
            return;
        };
        if !corner_hit(sample.x, sample.y, screen, flags) {
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

    /// 主显示器：虚拟屏幕坐标原点在它的左上角。
    const PRIMARY: ScreenRect = ScreenRect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };

    /// 摆在主显示器**左边**的副屏，x 坐标整段为负——正是旧实现失手的地方。
    const LEFT_OF_PRIMARY: ScreenRect = ScreenRect {
        left: -1920,
        top: 0,
        right: 0,
        bottom: 1080,
    };

    /// 摆在主显示器右边的副屏。
    const RIGHT_OF_PRIMARY: ScreenRect = ScreenRect {
        left: 1920,
        top: 0,
        right: 3840,
        bottom: 1080,
    };

    #[test]
    fn corner_hit_detects_each_enabled_corner() {
        assert!(corner_hit(0, 0, PRIMARY, F_TL));
        assert!(corner_hit(1919, 0, PRIMARY, F_TR));
        assert!(corner_hit(0, 1079, PRIMARY, F_BL));
        assert!(corner_hit(1919, 1079, PRIMARY, F_BR));
    }

    #[test]
    fn corner_hit_ignores_disabled_corners_and_center() {
        assert!(!corner_hit(0, 0, PRIMARY, F_TR), "左上角未启用时不触发");
        assert!(
            !corner_hit(960, 540, PRIMARY, CORNER_MASK),
            "屏幕中心不触发"
        );
    }

    // ---- 多显示器 ----------------------------------------------------------
    // 钩子给的是虚拟屏幕坐标：主屏左边的副屏 x 恒为负，右边的副屏 x 恒大于主屏宽度。
    // 判定必须按「光标所在的那块显示器」来，拿主屏宽高硬套会把副屏整条边缘都当成角。

    #[test]
    fn a_monitor_left_of_primary_has_its_corners_at_negative_coordinates() {
        assert!(corner_hit(-1920, 0, LEFT_OF_PRIMARY, F_TL), "副屏左上角");
        assert!(corner_hit(-1, 0, LEFT_OF_PRIMARY, F_TR), "副屏右上角");
        assert!(corner_hit(-1920, 1079, LEFT_OF_PRIMARY, F_BL), "副屏左下角");
        assert!(corner_hit(-1, 1079, LEFT_OF_PRIMARY, F_BR), "副屏右下角");
    }

    #[test]
    fn the_top_edge_of_a_left_monitor_is_not_one_long_corner() {
        // 旧实现里 `x <= 10` 对整块左副屏恒真，沿上边缘走一趟就会不停误触发。
        for x in [-1500, -1000, -600, -200, -50] {
            assert!(
                !corner_hit(x, 0, LEFT_OF_PRIMARY, CORNER_MASK),
                "上边缘中段 x={x} 不该算角落"
            );
            assert!(
                !corner_hit(x, 1079, LEFT_OF_PRIMARY, CORNER_MASK),
                "下边缘中段 x={x} 不该算角落"
            );
        }
    }

    #[test]
    fn the_edge_of_a_right_monitor_is_not_one_long_corner() {
        // 对称的另一半：旧实现里右副屏 `x >= 主屏宽-10` 恒真。
        for x in [2000, 2600, 3200, 3800] {
            assert!(
                !corner_hit(x, 0, RIGHT_OF_PRIMARY, CORNER_MASK),
                "上边缘中段 x={x} 不该算角落"
            );
        }
        assert!(corner_hit(3839, 0, RIGHT_OF_PRIMARY, F_TR), "真正的右上角");
    }

    #[test]
    fn the_seam_between_two_monitors_only_counts_for_the_side_it_belongs_to() {
        // 主屏左边界 x=0 与左副屏右边界 x=-1 紧贴；各自只在自己的矩形里算角。
        assert!(corner_hit(0, 0, PRIMARY, F_TL), "x=0 是主屏的左上角");
        assert!(
            !corner_hit(0, 0, LEFT_OF_PRIMARY, CORNER_MASK),
            "x=0 已经出了左副屏的范围，不是它的角"
        );
    }

    #[test]
    fn monitors_of_different_sizes_each_use_their_own_bounds() {
        // 副屏分辨率与主屏不同是常态，阈值须相对各自的边界。
        let small = ScreenRect {
            left: 1920,
            top: 0,
            right: 3200,
            bottom: 720,
        };
        assert!(corner_hit(3199, 719, small, F_BR), "小屏自己的右下角");
        assert!(
            !corner_hit(1919, 1079, small, CORNER_MASK),
            "主屏的右下角坐标不该命中小屏"
        );
    }

    #[test]
    fn a_monitor_above_primary_has_negative_y() {
        let above = ScreenRect {
            left: 0,
            top: -1080,
            right: 1920,
            bottom: 0,
        };
        assert!(corner_hit(0, -1080, above, F_TL), "上方副屏的左上角");
        assert!(
            !corner_hit(960, -1080, above, CORNER_MASK),
            "上边缘中段不该算角落"
        );
    }

    #[test]
    fn the_threshold_is_a_small_band_not_half_the_screen() {
        // 边界内 CORNER_THRESHOLD 像素算命中，再往里就不算。
        assert!(corner_hit(CORNER_THRESHOLD, 0, PRIMARY, F_TL));
        assert!(!corner_hit(CORNER_THRESHOLD + 1, 0, PRIMARY, F_TL));
    }

    #[test]
    fn single_monitor_hit_zones_are_unchanged_by_the_multi_monitor_fix() {
        // 主显示器的 left/top 都是 0，改用显示器矩形后判定必须与旧的
        // 「x <= t / x >= 宽度 - t」逐像素等价——这次改的是多显示器，
        // 不该顺带挪动单显示器下的角落热区。
        let t = CORNER_THRESHOLD;
        let (w, h) = (PRIMARY.right, PRIMARY.bottom);
        for x in [0, 1, t - 1, t, t + 1, w - t - 1, w - t, w - 1] {
            for y in [0, t, t + 1, h - t - 1, h - t, h - 1] {
                let old = {
                    let left = x <= t;
                    let right = x >= w - t;
                    let top = y <= t;
                    let bottom = y >= h - t;
                    // 四项展开是旧实现的原样，留着好逐项对照，不做等价化简。
                    #[allow(clippy::nonminimal_bool)]
                    {
                        (top && left) || (top && right) || (bottom && left) || (bottom && right)
                    }
                };
                assert_eq!(
                    corner_hit(x, y, PRIMARY, CORNER_MASK),
                    old,
                    "({x}, {y}) 的判定与旧实现不一致"
                );
            }
        }
    }

    #[test]
    fn a_point_outside_the_monitor_is_never_a_corner_of_it() {
        // 显示器非矩形排列时光标可能停在空隙里，而查询是「取最近的一块」，
        // 点并不在其中；此时不能把它当成那块的角。
        let gap_point = (-5, 2000);
        assert!(
            !corner_hit(gap_point.0, gap_point.1, PRIMARY, CORNER_MASK),
            "主屏之外的点不是主屏的角"
        );
        assert!(
            !corner_hit(gap_point.0, gap_point.1, LEFT_OF_PRIMARY, CORNER_MASK),
            "左副屏之外的点不是它的角"
        );
    }
}
