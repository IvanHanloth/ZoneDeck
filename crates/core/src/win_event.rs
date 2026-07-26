//! 窗口事件追踪：用 `SetWinEventHook` 订阅顶层窗口的销毁 / 显示 / 改标题事件，
//! 转发给代理窗口，由 agent 实时维护隐藏记录与规则信息。
//! 回调只做过滤与 `PostMessageW`，重活都在代理窗口的消息处理里。

use std::sync::atomic::{AtomicIsize, Ordering::Relaxed};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    CHILDID_SELF, EVENT_OBJECT_DESTROY, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW, OBJID_WINDOW,
    PostMessageW, WINEVENT_OUTOFCONTEXT, WM_APP,
};

/// 窗口事件转发给代理窗口的消息；`wparam` 为事件号，`lparam` 为窗口句柄。
pub const WM_APP_WINEVENT: u32 = WM_APP + 6;

static HWND_RAW: AtomicIsize = AtomicIsize::new(0);

/// 事件是否需要转发：只关心顶层窗口自身的销毁 / 显示 / 改标题。
fn relevant(event: u32, id_object: i32, id_child: i32) -> bool {
    id_object == OBJID_WINDOW.0
        && id_child == CHILDID_SELF as i32
        && matches!(
            event,
            EVENT_OBJECT_DESTROY | EVENT_OBJECT_SHOW | EVENT_OBJECT_NAMECHANGE
        )
}

unsafe extern "system" fn hook_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _id_thread: u32,
    _time: u32,
) {
    if !relevant(event, id_object, id_child) || hwnd.is_invalid() {
        return;
    }
    let raw = HWND_RAW.load(Relaxed);
    if raw == 0 {
        return;
    }
    unsafe {
        let agent = HWND(raw as *mut std::ffi::c_void);
        let _ = PostMessageW(
            Some(agent),
            WM_APP_WINEVENT,
            WPARAM(event as usize),
            LPARAM(hwnd.0 as isize),
        );
    }
}

/// 窗口事件钩子；析构时卸载。须在有消息循环的线程（代理窗口线程）上安装。
pub struct WinEventHook {
    handle: HWINEVENTHOOK,
}

impl WinEventHook {
    /// 一次挂钩覆盖 DESTROY(0x8001)–NAMECHANGE(0x800C) 区间，回调里再过滤。
    pub fn install(agent_hwnd: HWND) -> Option<WinEventHook> {
        HWND_RAW.store(agent_hwnd.0 as isize, Relaxed);
        let handle = unsafe {
            SetWinEventHook(
                EVENT_OBJECT_DESTROY,
                EVENT_OBJECT_NAMECHANGE,
                None,
                Some(hook_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        };
        if handle.is_invalid() {
            None
        } else {
            Some(WinEventHook { handle })
        }
    }
}

impl Drop for WinEventHook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWinEvent(self.handle);
        }
        HWND_RAW.store(0, Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::WindowsAndMessaging::{EVENT_OBJECT_FOCUS, EVENT_OBJECT_HIDE, OBJID_CURSOR};

    #[test]
    fn only_toplevel_destroy_show_namechange_are_relevant() {
        let obj = OBJID_WINDOW.0;
        let child = CHILDID_SELF as i32;
        assert!(relevant(EVENT_OBJECT_DESTROY, obj, child));
        assert!(relevant(EVENT_OBJECT_SHOW, obj, child));
        assert!(relevant(EVENT_OBJECT_NAMECHANGE, obj, child));
        assert!(!relevant(EVENT_OBJECT_HIDE, obj, child), "隐藏事件不用追踪");
        assert!(!relevant(EVENT_OBJECT_FOCUS, obj, child), "区间内的无关事件应过滤");
        assert!(!relevant(EVENT_OBJECT_DESTROY, OBJID_CURSOR.0, child), "非窗口对象应过滤");
        assert!(!relevant(EVENT_OBJECT_DESTROY, obj, 3), "子对象事件应过滤");
    }
}
