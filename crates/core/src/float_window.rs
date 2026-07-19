//! 桌面悬浮窗：无边框、置顶、不占任务栏的小窗口，显示程序图标。
//! 左键拖动、双击触发老板键、右键弹菜单；背景经颜色键透明。

use core::ffi::c_void;

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CS_DBLCLKS, CreatePopupMenu, CreateWindowExW, DI_NORMAL, DefWindowProcW,
    DestroyMenu, DestroyWindow, DrawIconEx, GWLP_USERDATA, GetCursorPos, GetWindowLongPtrW, HICON,
    HWND_TOPMOST, IDC_ARROW, LWA_COLORKEY, LoadCursorW, MF_SEPARATOR, MF_STRING, PostMessageW,
    RegisterClassW, SPI_GETWORKAREA, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOSIZE,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetForegroundWindow, SetLayeredWindowAttributes,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, TPM_LEFTALIGN,
    TPM_RIGHTBUTTON, TrackPopupMenu, WM_APP, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_NCDESTROY, WM_PAINT, WM_RBUTTONUP, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

/// 悬浮窗把用户意图转发给代理窗口（复用其隐藏/菜单逻辑）。
pub(crate) const WM_APP_FLOAT: u32 = WM_APP + 4;
/// 双击：切换隐藏/显示。
pub(crate) const FLOAT_TOGGLE: usize = 0;
/// 右键：弹出上下文菜单。
pub(crate) const FLOAT_MENU: usize = 1;

const FLOAT_SIZE: i32 = 64;
const FLOAT_MARGIN: i32 = 24;
/// 透明色键（洋红，图标几乎不会用到），背景填充此色后即变透明。
const COLOR_KEY: COLORREF = COLORREF(0x00FF_00FF);

/// 工作区（去掉任务栏后的可用屏幕区域），像素坐标。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// 悬浮窗初始位置：贴工作区右下角，留 `margin` 像素边距。
pub fn initial_position(work: WorkArea, size: i32, margin: i32) -> (i32, i32) {
    let x = work.right - size - margin;
    let y = work.bottom - size - margin;
    clamp_position((x, y), size, work)
}

/// 把窗口左上角夹在工作区内，保证整窗可见；工作区比窗口还小时对齐左上角。
pub fn clamp_position(pos: (i32, i32), size: i32, work: WorkArea) -> (i32, i32) {
    let (x, y) = pos;
    let max_x = (work.right - size).max(work.left);
    let max_y = (work.bottom - size).max(work.top);
    (x.clamp(work.left, max_x), y.clamp(work.top, max_y))
}

struct FloatState {
    /// 代理窗口句柄，双击/右键的意图消息发往这里。
    agent_hwnd: HWND,
    icon: HICON,
    dragging: bool,
    /// 抓取点相对窗口左上角的偏移（客户区坐标）。
    grab: POINT,
}

pub(crate) struct FloatWindow {
    hwnd: HWND,
}

impl FloatWindow {
    /// 创建并显示悬浮窗；须在拥有消息循环的线程上调用，失败返回 `None`。
    pub(crate) fn create(agent_hwnd: HWND) -> Option<FloatWindow> {
        unsafe {
            let hinstance = GetModuleHandleW(PCWSTR::null()).ok()?;
            let class_name = w!("BossKeyFloatWindow");
            let wc = WNDCLASSW {
                style: CS_DBLCLKS,
                lpfnWndProc: Some(float_wndproc),
                hInstance: hinstance.into(),
                lpszClassName: class_name,
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                ..Default::default()
            };
            RegisterClassW(&wc);

            let (x, y) = initial_position(work_area(), FLOAT_SIZE, FLOAT_MARGIN);
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                class_name,
                w!("Boss Key"),
                WS_POPUP,
                x,
                y,
                FLOAT_SIZE,
                FLOAT_SIZE,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
            .ok()?;

            let state = Box::new(FloatState {
                agent_hwnd,
                icon: crate::tray::load_app_icon(),
                dragging: false,
                grab: POINT::default(),
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            let _ = SetLayeredWindowAttributes(hwnd, COLOR_KEY, 0, LWA_COLORKEY);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            Some(FloatWindow { hwnd })
        }
    }
}

impl Drop for FloatWindow {
    fn drop(&mut self) {
        // WM_NCDESTROY 会同步释放 FloatState，这里只销毁窗口。
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn float_state<'a>(hwnd: HWND) -> Option<&'a mut FloatState> {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut FloatState;
        ptr.as_mut()
    }
}

/// 从 `LPARAM` 取带符号的鼠标坐标（GET_X/Y_LPARAM）。
fn loword_i32(l: LPARAM) -> i32 {
    (l.0 & 0xFFFF) as u16 as i16 as i32
}
fn hiword_i32(l: LPARAM) -> i32 {
    ((l.0 >> 16) & 0xFFFF) as u16 as i16 as i32
}

fn work_area() -> WorkArea {
    unsafe {
        let mut r = RECT::default();
        let ok = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut r as *mut RECT as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok();
        if ok {
            WorkArea {
                left: r.left,
                top: r.top,
                right: r.right,
                bottom: r.bottom,
            }
        } else {
            WorkArea {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            }
        }
    }
}

fn paint_icon(hwnd: HWND) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let brush = CreateSolidBrush(COLOR_KEY);
        FillRect(hdc, &ps.rcPaint, brush);
        let _ = DeleteObject(brush.into());
        if let Some(st) = float_state(hwnd) {
            let _ = DrawIconEx(
                hdc, 0, 0, st.icon, FLOAT_SIZE, FLOAT_SIZE, 0, None, DI_NORMAL,
            );
        }
        let _ = EndPaint(hwnd, &ps);
    }
}

unsafe extern "system" fn float_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint_icon(hwnd);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            if let Some(st) = float_state(hwnd) {
                st.dragging = true;
                st.grab = POINT {
                    x: loword_i32(lparam),
                    y: hiword_i32(lparam),
                };
                unsafe {
                    let _ = SetCapture(hwnd);
                }
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if let Some(st) = float_state(hwnd)
                && st.dragging
            {
                unsafe {
                    let mut pt = POINT::default();
                    let _ = GetCursorPos(&mut pt);
                    let (cx, cy) = clamp_position(
                        (pt.x - st.grab.x, pt.y - st.grab.y),
                        FLOAT_SIZE,
                        work_area(),
                    );
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_TOPMOST),
                        cx,
                        cy,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if let Some(st) = float_state(hwnd)
                && st.dragging
            {
                st.dragging = false;
                unsafe {
                    let _ = ReleaseCapture();
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK => {
            if let Some(st) = float_state(hwnd) {
                unsafe {
                    let _ = PostMessageW(
                        Some(st.agent_hwnd),
                        WM_APP_FLOAT,
                        WPARAM(FLOAT_TOGGLE),
                        LPARAM(0),
                    );
                }
            }
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            if let Some(st) = float_state(hwnd) {
                unsafe {
                    let _ = PostMessageW(
                        Some(st.agent_hwnd),
                        WM_APP_FLOAT,
                        WPARAM(FLOAT_MENU),
                        LPARAM(0),
                    );
                }
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut FloatState;
            if !ptr.is_null() {
                unsafe {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    drop(Box::from_raw(ptr));
                }
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 悬浮窗右键菜单：设置 / 退出。以代理窗口为宿主，选择项经 `WM_COMMAND` 处理。
pub(crate) fn show_float_menu(agent_hwnd: HWND, id_settings: usize, id_quit: usize) -> bool {
    unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return false;
        };
        crate::util::append_menu_item(menu, MF_STRING, id_settings, crate::i18n::Msg::MenuSettings);
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        crate::util::append_menu_item(menu, MF_STRING, id_quit, crate::i18n::Msg::MenuQuit);

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(agent_hwnd);
        let shown = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            None,
            agent_hwnd,
            None,
        )
        .as_bool();
        let _ = DestroyMenu(menu);
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: WorkArea = WorkArea {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };

    #[test]
    fn initial_position_sits_bottom_right_with_margin() {
        assert_eq!(
            initial_position(FHD, 64, 24),
            (1920 - 64 - 24, 1080 - 64 - 24)
        );
    }

    #[test]
    fn initial_position_clamps_on_tiny_workarea() {
        let tiny = WorkArea {
            left: 0,
            top: 0,
            right: 40,
            bottom: 40,
        };
        assert_eq!(initial_position(tiny, 64, 24), (0, 0));
    }

    #[test]
    fn clamp_pulls_window_back_from_top_left() {
        assert_eq!(clamp_position((-100, -50), 64, FHD), (0, 0));
    }

    #[test]
    fn clamp_pulls_window_back_from_bottom_right() {
        assert_eq!(
            clamp_position((5000, 5000), 64, FHD),
            (1920 - 64, 1080 - 64)
        );
    }

    #[test]
    fn clamp_keeps_inside_position_unchanged() {
        assert_eq!(clamp_position((300, 400), 64, FHD), (300, 400));
    }

    #[test]
    fn clamp_handles_workarea_smaller_than_window() {
        let tiny = WorkArea {
            left: 0,
            top: 0,
            right: 40,
            bottom: 40,
        };
        assert_eq!(clamp_position((10, 10), 64, tiny), (0, 0));
    }

    #[test]
    fn clamp_respects_negative_origin_multi_monitor() {
        let left_monitor = WorkArea {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1080,
        };
        assert_eq!(clamp_position((-5000, 500), 64, left_monitor), (-1920, 500));
        assert_eq!(clamp_position((5000, 500), 64, left_monitor), (-64, 500));
    }
}
