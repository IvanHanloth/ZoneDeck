//! 录制期独占键盘：装 `WH_KEYBOARD_LL` 把所有按键吞掉，只转成事件送给调用方。
//!
//! 独占键盘是让录制不外溢的唯一办法：WebView 的 `preventDefault` 挡不住
//! `RegisterHotKey` 的全局热键、别人装的低级钩子与 Win+R / Alt+Tab 这类 shell 热键。
//!
//! Win+L 与 Ctrl+Alt+Del 由 winlogon 在钩子层之下处理，拦不住；
//! 直接读取 Raw Input 的程序也不经过本钩子。

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::thread::JoinHandle;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    GetCurrentProcessId, GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, HHOOK,
    HWND_MESSAGE, KBDLLHOOKSTRUCT, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WINDOW_EX_STYLE,
    WM_CLOSE, WM_DESTROY, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSW,
    WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

use crate::hotkey::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

/// 一次按键的按下或抬起。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub vk: u16,
    pub down: bool,
    /// 该事件发生后按住的修饰键位掩码。
    pub modifiers: u32,
    /// 该事件发生后按住的非修饰键，按按下先后排列。
    pub keys: Vec<u16>,
}

// 每个物理修饰键一位，左右分开记，松开一侧不会误清整个修饰位。
// 注入的输入可能报不分左右的通用码，另占一位。
const B_LCTRL: u16 = 1 << 0;
const B_RCTRL: u16 = 1 << 1;
const B_GCTRL: u16 = 1 << 2;
const B_LALT: u16 = 1 << 3;
const B_RALT: u16 = 1 << 4;
const B_GALT: u16 = 1 << 5;
const B_LSHIFT: u16 = 1 << 6;
const B_RSHIFT: u16 = 1 << 7;
const B_GSHIFT: u16 = 1 << 8;
const B_LWIN: u16 = 1 << 9;
const B_RWIN: u16 = 1 << 10;

const CTRL_BITS: u16 = B_LCTRL | B_RCTRL | B_GCTRL;
const ALT_BITS: u16 = B_LALT | B_RALT | B_GALT;
const SHIFT_BITS: u16 = B_LSHIFT | B_RSHIFT | B_GSHIFT;
const WIN_BITS: u16 = B_LWIN | B_RWIN;

/// 该虚拟键码对应的修饰位；不是修饰键则为 None。
fn modifier_bit(vk: u16) -> Option<u16> {
    let bit = match vk {
        0x11 => B_GCTRL,  // VK_CONTROL
        0xA2 => B_LCTRL,  // VK_LCONTROL
        0xA3 => B_RCTRL,  // VK_RCONTROL
        0x12 => B_GALT,   // VK_MENU
        0xA4 => B_LALT,   // VK_LMENU
        0xA5 => B_RALT,   // VK_RMENU
        0x10 => B_GSHIFT, // VK_SHIFT
        0xA0 => B_LSHIFT, // VK_LSHIFT
        0xA1 => B_RSHIFT, // VK_RSHIFT
        0x5B => B_LWIN,   // VK_LWIN
        0x5C => B_RWIN,   // VK_RWIN
        _ => return None,
    };
    Some(bit)
}

/// 该虚拟键码是不是修饰键；界面据此区分「只按着修饰键」与「按了不支持的键」。
pub fn is_modifier(vk: u16) -> bool {
    modifier_bit(vk).is_some()
}

/// 布局相关的主键在当前键盘布局下显示的字符，键为热键字符串里的位置名。
/// OEM 符号键与小键盘的字面含义随布局变化，配置存位置名、只有显示走这张表；
/// 取不到名字或与位置名一致的不进表，界面回落显示位置名。
pub fn layout_key_labels() -> std::collections::HashMap<String, String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyNameTextW, MAPVK_VK_TO_VSC, MapVirtualKeyW,
    };

    let oem = (0xBAu16..=0xC0).chain(0xDB..=0xDF).chain([0xE2]);
    let mut map = std::collections::HashMap::new();
    for vk in oem.chain(0x60..=0x6F) {
        let Some(name) = crate::hotkey::vk_to_key(vk) else {
            continue;
        };
        let scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) };
        if scan == 0 {
            continue;
        }
        let mut buf = [0u16; 64];
        // 扫描码放在 lparam 的 16..24 位，与 WM_KEYDOWN 的编码一致。
        let len = unsafe { GetKeyNameTextW((scan << 16) as i32, &mut buf) };
        if len <= 0 {
            continue;
        }
        let label = String::from_utf16_lossy(&buf[..len as usize]);
        if !label.is_empty() && label != name {
            map.insert(name, label);
        }
    }
    map
}

/// 按住的修饰键。钩子吞掉按键后 `GetAsyncKeyState` 的更新时机有歧义，
/// 而我们处在钩子链头部、每次按下抬起都看得到，自己记是确定的。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ModifierTracker {
    held: u16,
}

impl ModifierTracker {
    /// 记下一次按键，返回此刻的修饰键位掩码。
    pub fn apply(&mut self, vk: u16, down: bool) -> u32 {
        if let Some(bit) = modifier_bit(vk) {
            if down {
                self.held |= bit;
            } else {
                self.held &= !bit;
            }
        }
        self.modifiers()
    }

    pub fn modifiers(&self) -> u32 {
        let mut m = 0;
        if self.held & CTRL_BITS != 0 {
            m |= MOD_CONTROL;
        }
        if self.held & ALT_BITS != 0 {
            m |= MOD_ALT;
        }
        if self.held & SHIFT_BITS != 0 {
            m |= MOD_SHIFT;
        }
        if self.held & WIN_BITS != 0 {
            m |= MOD_WIN;
        }
        m
    }

    /// 以键盘的真实状态起步，覆盖「开始录制时已经按住修饰键」的情况。
    /// 按左右分别播种，之后任一侧抬起都能正确清位。
    fn seed_from_keyboard(&mut self) {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        for (vk, bit) in [
            (0xA2u16, B_LCTRL),
            (0xA3, B_RCTRL),
            (0xA4, B_LALT),
            (0xA5, B_RALT),
            (0xA0, B_LSHIFT),
            (0xA1, B_RSHIFT),
            (0x5B, B_LWIN),
            (0x5C, B_RWIN),
        ] {
            if unsafe { GetAsyncKeyState(i32::from(vk)) as u16 & 0x8000 } != 0 {
                self.held |= bit;
            }
        }
    }
}

type KeyHandler = Box<dyn Fn(KeyEvent) + Send>;

/// 当前按住的非修饰键，按按下先后排列。录制多主键组合时用来给出整组主键。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct HeldKeys {
    keys: Vec<u16>,
}

impl HeldKeys {
    /// 记下一次按键，返回此刻按住的非修饰键。超过热键能容纳的主键数后不再收。
    pub fn apply(&mut self, vk: u16, down: bool) -> &[u16] {
        if is_modifier(vk) {
            return &self.keys;
        }
        if down {
            if !self.keys.contains(&vk) && self.keys.len() < crate::hotkey::MAX_KEYS {
                self.keys.push(vk);
            }
        } else {
            self.keys.retain(|k| *k != vk);
        }
        &self.keys
    }
}

struct Capture {
    tracker: ModifierTracker,
    held: HeldKeys,
    on_key: KeyHandler,
}

static CAPTURE: Mutex<Option<Capture>> = Mutex::new(None);
/// 本进程 PID，缓存下来供钩子里的前台判定用。
static OWN_PID: AtomicU32 = AtomicU32::new(0);
/// 钩子被调用的次数。为 0 说明钩子没装上、已被系统摘掉，或被别的程序的钩子截在前面。
static SEEN: AtomicU32 = AtomicU32::new(0);
/// 因本进程不在前台而放行的次数。只有它在涨说明钩子是活的，卡在前台判定上。
static PASSED_BACKGROUND: AtomicU32 = AtomicU32::new(0);

/// 上一轮录制的诊断计数：`(钩子收到的按键数, 因非前台放行的次数)`。
/// 录制没反应时据此区分「钩子没被调用」与「钩子活着但判定本进程不在前台」。
pub fn capture_stats() -> (u32, u32) {
    (SEEN.load(Relaxed), PASSED_BACKGROUND.load(Relaxed))
}

/// 前台窗口所属进程的 PID；取不到时为 0。
pub fn foreground_pid() -> u32 {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return 0;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid
    }
}

/// 前台窗口是否属于本进程。录制界面一失焦就停止吞键，避免键盘被锁死。
fn foreground_is_ours() -> bool {
    let own = OWN_PID.load(Relaxed);
    if own == 0 {
        return false;
    }
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid == own
    }
}

unsafe extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode == 0 {
        SEEN.fetch_add(1, Relaxed);
        if !foreground_is_ours() {
            PASSED_BACKGROUND.fetch_add(1, Relaxed);
        } else {
            let down = match wparam.0 as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => Some(true),
                WM_KEYUP | WM_SYSKEYUP => Some(false),
                _ => None,
            };
            if let Some(down) = down {
                let vk = unsafe { (*(lparam.0 as *const KBDLLHOOKSTRUCT)).vkCode as u16 };
                // 回调只往通道里塞一条就返回：低级钩子超过 LowLevelHooksTimeout
                // （默认 300ms）事件会被系统丢弃。
                if let Ok(mut guard) = CAPTURE.lock()
                    && let Some(capture) = guard.as_mut()
                {
                    let modifiers = capture.tracker.apply(vk, down);
                    let keys = capture.held.apply(vk, down).to_vec();
                    (capture.on_key)(KeyEvent {
                        vk,
                        down,
                        modifiers,
                        keys,
                    });
                    return LRESULT(1);
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, ncode, wparam, lparam) }
}

/// 钩子线程自己的仅消息窗口，用来接收退出请求。
fn create_capture_window() -> windows::core::Result<HWND> {
    unsafe {
        let hinstance = GetModuleHandleW(PCWSTR::null())?;
        let class_name = w!("ZoneDeckKeyCaptureWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("ZoneDeck Key Capture"),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinstance.into()),
            None,
        )
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 装上就自动卸的钩子句柄。
struct Hook(HHOOK);

impl Drop for Hook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWindowsHookEx(self.0);
        }
    }
}

/// 录制期对键盘的独占。析构时卸钩子并停线程。
pub struct KeyCapture {
    hwnd: isize,
    thread: Option<JoinHandle<()>>,
}

impl KeyCapture {
    /// 起专职线程装钩子；`on_key` 在钩子线程上调用，必须够快（只投递，别干活）。
    ///
    /// 低级钩子的回调由安装线程的消息泵派发，故不能挂在调用方线程上。
    /// 同一时刻只应有一个 `KeyCapture` 存活。
    pub fn start(on_key: impl Fn(KeyEvent) + Send + 'static) -> Option<KeyCapture> {
        OWN_PID.store(unsafe { GetCurrentProcessId() }, Relaxed);
        SEEN.store(0, Relaxed);
        PASSED_BACKGROUND.store(0, Relaxed);
        let mut tracker = ModifierTracker::default();
        tracker.seed_from_keyboard();
        match CAPTURE.lock() {
            Ok(mut guard) => {
                *guard = Some(Capture {
                    tracker,
                    held: HeldKeys::default(),
                    on_key: Box::new(on_key),
                });
            }
            Err(_) => {
                crate::log_error!("键盘录制状态已损坏，无法开始录制");
                return None;
            }
        }

        let (tx, rx) = std::sync::mpsc::channel::<isize>();
        let thread = std::thread::Builder::new()
            .name("zonedeck-key-capture".into())
            .spawn(move || {
                unsafe {
                    // 不上 TIME_CRITICAL，避免抢走前台程序的调度权。
                    let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL);
                }
                let hwnd = match create_capture_window() {
                    Ok(hwnd) => hwnd,
                    Err(e) => {
                        crate::log_error!(
                            "创建键盘录制窗口失败，录制将无法独占键盘: {}",
                            crate::util::win_err(&e)
                        );
                        let _ = tx.send(0);
                        return;
                    }
                };
                let hook = unsafe {
                    GetModuleHandleW(PCWSTR::null()).ok().and_then(|hinstance| {
                        SetWindowsHookExW(
                            WH_KEYBOARD_LL,
                            Some(hook_proc),
                            Some(hinstance.into()),
                            0,
                        )
                        .ok()
                    })
                };
                let Some(hook) = hook.map(Hook) else {
                    crate::logging::error("安装键盘录制钩子失败，录制将无法独占键盘");
                    unsafe {
                        let _ = DestroyWindow(hwnd);
                    }
                    let _ = tx.send(0);
                    return;
                };
                if tx.send(hwnd.0 as isize).is_err() {
                    drop(hook);
                    unsafe {
                        let _ = DestroyWindow(hwnd);
                    }
                    return;
                }
                unsafe {
                    let mut msg: MSG = std::mem::zeroed();
                    loop {
                        let ret = GetMessageW(&mut msg, None, 0, 0);
                        if ret.0 <= 0 {
                            break;
                        }
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
                drop(hook);
            });

        let thread = match thread {
            Ok(thread) => thread,
            Err(e) => {
                crate::log_error!("创建键盘录制线程失败，录制将无法独占键盘: {e}");
                clear_capture();
                return None;
            }
        };

        match rx.recv() {
            Ok(hwnd) if hwnd != 0 => Some(KeyCapture {
                hwnd,
                thread: Some(thread),
            }),
            _ => {
                let _ = thread.join();
                clear_capture();
                None
            }
        }
    }
}

fn clear_capture() {
    match CAPTURE.lock() {
        Ok(mut guard) => *guard = None,
        Err(mut poisoned) => **poisoned.get_mut() = None,
    }
}

impl Drop for KeyCapture {
    fn drop(&mut self) {
        unsafe {
            let hwnd = HWND(self.hwnd as *mut std::ffi::c_void);
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            crate::logging::warn("键盘录制线程异常退出");
        }
        // 钩子已卸，回调不会再被调用，这里只是把闭包放掉。
        clear_capture();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Threading::GetCurrentThreadId;

    const VK_LSHIFT: u16 = 0xA0;
    const VK_RSHIFT: u16 = 0xA1;
    const VK_LCONTROL: u16 = 0xA2;
    const VK_LMENU: u16 = 0xA4;
    const VK_LWIN: u16 = 0x5B;
    const VK_Q: u16 = 0x51;

    #[test]
    fn plain_keys_do_not_change_the_modifier_state() {
        let mut t = ModifierTracker::default();
        assert_eq!(t.apply(VK_Q, true), 0);
        assert_eq!(t.apply(VK_Q, false), 0);
    }

    #[test]
    fn modifiers_accumulate_and_clear() {
        let mut t = ModifierTracker::default();
        assert_eq!(t.apply(VK_LCONTROL, true), MOD_CONTROL);
        assert_eq!(t.apply(VK_LSHIFT, true), MOD_CONTROL | MOD_SHIFT);
        assert_eq!(t.apply(VK_LMENU, true), MOD_CONTROL | MOD_SHIFT | MOD_ALT);
        assert_eq!(
            t.apply(VK_LWIN, true),
            MOD_CONTROL | MOD_SHIFT | MOD_ALT | MOD_WIN
        );
        assert_eq!(t.apply(VK_LMENU, false), MOD_CONTROL | MOD_SHIFT | MOD_WIN);
        assert_eq!(t.apply(VK_LWIN, false), MOD_CONTROL | MOD_SHIFT);
        assert_eq!(t.apply(VK_LSHIFT, false), MOD_CONTROL);
        assert_eq!(t.apply(VK_LCONTROL, false), 0);
    }

    #[test]
    fn releasing_one_side_keeps_the_other_side_held() {
        let mut t = ModifierTracker::default();
        t.apply(VK_LSHIFT, true);
        assert_eq!(t.apply(VK_RSHIFT, true), MOD_SHIFT);
        assert_eq!(
            t.apply(VK_LSHIFT, false),
            MOD_SHIFT,
            "右 Shift 还按着，Shift 位不该清掉"
        );
        assert_eq!(t.apply(VK_RSHIFT, false), 0);
    }

    #[test]
    fn generic_modifier_codes_from_injected_input_are_tracked_separately() {
        let mut t = ModifierTracker::default();
        // 注入的输入报不分左右的 VK_CONTROL。
        assert_eq!(t.apply(0x11, true), MOD_CONTROL);
        assert_eq!(
            t.apply(VK_LCONTROL, false),
            MOD_CONTROL,
            "左 Ctrl 的抬起不该清掉通用码留下的状态"
        );
        assert_eq!(t.apply(0x11, false), 0);
    }

    #[test]
    fn stray_release_without_a_matching_press_is_harmless() {
        let mut t = ModifierTracker::default();
        assert_eq!(t.apply(VK_LCONTROL, false), 0);
        assert_eq!(t.apply(VK_LCONTROL, true), MOD_CONTROL);
    }

    #[test]
    fn held_keys_track_press_order_and_ignore_modifiers() {
        let mut h = HeldKeys::default();
        assert_eq!(h.apply(VK_LCONTROL, true), &[] as &[u16], "修饰键不算主键");
        assert_eq!(h.apply(0x57, true), &[0x57]);
        assert_eq!(h.apply(VK_Q, true), &[0x57, VK_Q], "按按下先后排列");
        assert_eq!(h.apply(VK_Q, true), &[0x57, VK_Q], "长按重复不重复记录");
        assert_eq!(h.apply(0x57, false), &[VK_Q]);
        assert_eq!(h.apply(VK_Q, false), &[] as &[u16]);
    }

    #[test]
    fn held_keys_stop_at_the_hotkey_main_key_limit() {
        let mut h = HeldKeys::default();
        for vk in 0x41..=0x41 + crate::hotkey::MAX_KEYS as u16 {
            h.apply(vk, true);
        }
        assert_eq!(
            h.apply(0x5A, true).len(),
            crate::hotkey::MAX_KEYS,
            "超出热键容量的主键不再收，免得录出解析不了的组合"
        );
    }

    /// 低级钩子只能装在跑消息泵的线程上，装得上即说明钩子落到了专职线程。
    #[test]
    fn capture_runs_on_its_own_thread_and_stops_on_drop() {
        let hwnd = {
            let capture = KeyCapture::start(|_| {}).expect("键盘录制应能启动");
            let owner = unsafe {
                GetWindowThreadProcessId(HWND(capture.hwnd as *mut std::ffi::c_void), None)
            };
            assert_ne!(owner, 0, "录制窗口应存活");
            assert_ne!(
                owner,
                unsafe { GetCurrentThreadId() },
                "钩子必须挂在专职线程上"
            );
            capture.hwnd
        };
        let owner = unsafe { GetWindowThreadProcessId(HWND(hwnd as *mut std::ffi::c_void), None) };
        assert_eq!(owner, 0, "析构后录制窗口应已销毁");
        assert!(CAPTURE.lock().unwrap().is_none(), "析构后不该再留着回调");
    }

    /// 吞键的硬性兜底：本进程不在前台时一律放行。
    /// 测试进程没有窗口，前台窗口必然属于别人。
    #[test]
    fn keys_pass_through_when_we_are_not_the_foreground_process() {
        let _capture = KeyCapture::start(|_| panic!("非前台时不该收到按键")).expect("录制应能启动");
        assert!(!foreground_is_ours(), "测试进程没有窗口，不该被当成前台");
        // 直接过一遍钩子的判定，确认走的是放行分支。
        let data = KBDLLHOOKSTRUCT {
            vkCode: u32::from(VK_Q),
            ..Default::default()
        };
        let ret = unsafe {
            hook_proc(
                0,
                WPARAM(WM_KEYDOWN as usize),
                LPARAM(&data as *const _ as isize),
            )
        };
        assert_eq!(ret.0, 0, "非前台时钩子必须放行，不能返回吞掉");
    }
}
