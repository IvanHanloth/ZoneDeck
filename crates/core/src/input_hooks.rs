//! 输入钩子专职线程：`WH_MOUSE_LL` 与 `WH_KEYBOARD_LL` 都挂在这条线程上。
//!
//! 低级钩子的回调由安装线程的消息泵派发，系统的输入线程要等钩子链返回才继续投递
//! 事件。故不与代理窗口共用线程：枚举窗口、写恢复文件这类重活会卡住全局输入，
//! 单次超过 `LowLevelHooksTimeout`（默认 300ms）时系统还会丢弃该事件。
//! 两个钩子的回调都只做内存判定与 `PostMessageW`，互不阻塞，共用一条线程即可。

use std::sync::atomic::{AtomicIsize, Ordering::Relaxed};
use std::sync::mpsc::channel;
use std::thread::JoinHandle;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    HWND_MESSAGE, MSG, PostMessageW, PostQuitMessage, RegisterClassW, SendMessageW,
    TranslateMessage, WINDOW_EX_STYLE, WM_APP, WM_CLOSE, WM_DESTROY, WNDCLASSW, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

use crate::keyboard_hook::KeyboardHook;
use crate::logging;
use crate::mouse_hook::MouseHook;

/// 装 / 卸某个钩子；`wparam` 为钩子种类，`lparam` 非 0 表示装。
/// 返回值非 0 表示该钩子当前已装上。
const WM_HOOK_SET: u32 = WM_APP + 7;

const HOOK_MOUSE: usize = 0;
const HOOK_KEYBOARD: usize = 1;

/// 代理窗口句柄，钩子命中后把触发投递给它。
static AGENT_HWND: AtomicIsize = AtomicIsize::new(0);

/// 钩子句柄只在钩子线程上创建与销毁，故存在该线程的 TLS 里。
#[derive(Default)]
struct Installed {
    mouse: Option<MouseHook>,
    keyboard: Option<KeyboardHook>,
}

thread_local! {
    static INSTALLED: std::cell::RefCell<Installed> = std::cell::RefCell::new(Installed::default());
}

/// 把某个钩子对齐到目标状态，返回对齐后是否已装上。幂等。
fn apply(kind: usize, want: bool) -> bool {
    let agent = HWND(AGENT_HWND.load(Relaxed) as *mut std::ffi::c_void);
    INSTALLED.with(|cell| {
        let mut installed = cell.borrow_mut();
        match kind {
            HOOK_MOUSE => {
                if !want {
                    installed.mouse = None;
                } else if installed.mouse.is_none() {
                    installed.mouse = MouseHook::install(agent);
                }
                installed.mouse.is_some()
            }
            HOOK_KEYBOARD => {
                if !want {
                    installed.keyboard = None;
                } else if installed.keyboard.is_none() {
                    installed.keyboard = KeyboardHook::install(agent);
                }
                installed.keyboard.is_some()
            }
            _ => false,
        }
    })
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_HOOK_SET => LRESULT(apply(wparam.0, lparam.0 != 0) as isize),
        WM_DESTROY => {
            INSTALLED.with(|cell| *cell.borrow_mut() = Installed::default());
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 钩子线程自己的仅消息窗口，作为装卸请求的收口。
fn create_hook_window() -> windows::core::Result<HWND> {
    unsafe {
        let hinstance = GetModuleHandleW(PCWSTR::null())?;
        let class_name = w!("BossKeyInputHookWindow");
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
            w!("Boss Key Input Hooks"),
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

/// 钩子线程的句柄；析构时通知线程退出并等它收尾（卸钩子）。
pub struct InputHooks {
    hwnd: isize,
    thread: Option<JoinHandle<()>>,
}

impl InputHooks {
    /// 起线程并等它把仅消息窗口建好。窗口建不出来时返回 None，调用方须自行降级。
    pub fn spawn(agent_hwnd: HWND) -> Option<InputHooks> {
        AGENT_HWND.store(agent_hwnd.0 as isize, Relaxed);
        let (tx, rx) = channel::<isize>();
        let thread = std::thread::Builder::new()
            .name("bosskey-input-hooks".into())
            .spawn(move || {
                unsafe {
                    // 不上 TIME_CRITICAL，避免把调度权从前台程序手里整体抢走。
                    let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL);
                }
                let hwnd = match create_hook_window() {
                    Ok(hwnd) => hwnd,
                    Err(e) => {
                        crate::log_error!(
                            "创建输入钩子窗口失败，鼠标绑定与「不传递」热键将不可用: {}",
                            crate::util::win_err(&e)
                        );
                        let _ = tx.send(0);
                        return;
                    }
                };
                if tx.send(hwnd.0 as isize).is_err() {
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
            })
            .ok()?;

        match rx.recv() {
            Ok(hwnd) if hwnd != 0 => Some(InputHooks {
                hwnd,
                thread: Some(thread),
            }),
            _ => {
                let _ = thread.join();
                None
            }
        }
    }

    /// 让鼠标钩子对齐 `on`，返回对齐后是否已装上。
    pub fn set_mouse(&self, on: bool) -> bool {
        self.set(HOOK_MOUSE, on)
    }

    /// 让键盘钩子对齐 `on`，返回对齐后是否已装上。
    pub fn set_keyboard(&self, on: bool) -> bool {
        self.set(HOOK_KEYBOARD, on)
    }

    /// 钩子线程从不阻塞、也从不回发消息给代理线程，故 `SendMessageW` 不会死锁，
    /// 且能直接拿到安装结果。
    fn set(&self, kind: usize, on: bool) -> bool {
        unsafe {
            let hwnd = HWND(self.hwnd as *mut std::ffi::c_void);
            SendMessageW(
                hwnd,
                WM_HOOK_SET,
                Some(WPARAM(kind)),
                Some(LPARAM(on as isize)),
            )
            .0 != 0
        }
    }
}

impl Drop for InputHooks {
    fn drop(&mut self) {
        unsafe {
            let hwnd = HWND(self.hwnd as *mut std::ffi::c_void);
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            logging::warn("输入钩子线程异常退出");
        }
        AGENT_HWND.store(0, Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    /// 代理窗口句柄传 0：钩子照常装得上，命中时的 `PostMessageW` 会被 0 句柄挡掉。
    fn hooks() -> InputHooks {
        InputHooks::spawn(HWND(std::ptr::null_mut())).expect("输入钩子线程应能启动")
    }

    /// 直接验证承载钩子的消息窗口属于另一条线程。
    #[test]
    fn hooks_live_on_their_own_thread() {
        let hooks = hooks();
        let owner =
            unsafe { GetWindowThreadProcessId(HWND(hooks.hwnd as *mut std::ffi::c_void), None) };
        assert_ne!(owner, 0, "钩子窗口应存活");
        assert_ne!(
            owner,
            unsafe { GetCurrentThreadId() },
            "钩子必须挂在专职线程上，否则调用方的重活会拖慢全局输入"
        );
    }

    /// 本测试线程没有消息循环，而低级钩子只能装在跑消息泵的线程上，
    /// 故能装上本身即说明请求落到了钩子线程。
    #[test]
    fn mouse_and_keyboard_hooks_arm_and_disarm_independently() {
        let hooks = hooks();
        assert!(hooks.set_mouse(true));
        assert!(hooks.set_mouse(true), "重复请求应幂等，不重复安装");
        assert!(!hooks.set_keyboard(false), "另一个钩子不受影响");

        assert!(hooks.set_keyboard(true));
        assert!(!hooks.set_mouse(false));
        assert!(hooks.set_keyboard(true), "卸掉鼠标钩子不应连累键盘钩子");

        assert!(!hooks.set_keyboard(false));
    }

    /// 析构须让线程退出，否则热重载会让线程越积越多。
    #[test]
    fn dropping_stops_the_thread_and_destroys_its_window() {
        let hwnd = {
            let hooks = hooks();
            hooks.set_mouse(true);
            hooks.hwnd
        };
        let owner = unsafe { GetWindowThreadProcessId(HWND(hwnd as *mut std::ffi::c_void), None) };
        assert_eq!(owner, 0, "线程退出后钩子窗口应已销毁");
    }
}
