use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use bosskey_common::Config;
use bosskey_common::ipc::{Command, Response};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetCursorPos, GetMessageW, GetWindowLongPtrW,
    KillTimer, MF_CHECKED, MF_SEPARATOR, MF_STRING, MSG, PostMessageW, PostQuitMessage,
    RegisterClassW, SetForegroundWindow, SetTimer, SetWindowLongPtrW, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE, WM_APP, WM_COMMAND,
    WM_DESTROY, WM_HOTKEY, WM_LBUTTONUP, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

use crate::hide::HideController;
use crate::hotkey::{MOD_NOREPEAT, ParsedHotkey, parse_hotkey};
use crate::ipc_server;
use crate::platform::win32::WindowsWindowManager;
use crate::tray::TrayIcon;

const HK_HIDE: i32 = 1;
const HK_CLOSE: i32 = 2;

const WM_APP_IPC: u32 = WM_APP + 1;
const WM_APP_TRAY: u32 = WM_APP + 2;

const MENU_SETTINGS: usize = 1001;
const MENU_TOGGLE: usize = 1002;
const MENU_QUIT: usize = 1003;
const MENU_AUTOSTART: usize = 1004;

const AUTO_QUIT_TIMER_ID: usize = 10;
const IPC_REPLY_TIMEOUT: Duration = Duration::from_secs(3);

pub struct AgentOptions {
    pub config_path: PathBuf,
    pub pipe_name: String,
    pub enable_tray: bool,
    pub auto_quit_ms: Option<u32>,
}

impl AgentOptions {
    pub fn standard(config_path: PathBuf) -> Self {
        Self {
            config_path,
            pipe_name: bosskey_common::ipc::PIPE_NAME.to_string(),
            enable_tray: true,
            auto_quit_ms: None,
        }
    }
}

struct AgentState {
    config: Config,
    config_path: PathBuf,
    controller: HideController<WindowsWindowManager>,
    tray: Option<TrayIcon>,
    ipc_rx: Receiver<(Command, Sender<Response>)>,
}

impl AgentState {
    fn register_hotkeys(&self, hwnd: HWND) {
        for (id, label, raw) in [
            (HK_HIDE, "隐藏", &self.config.hotkey.hide_hotkey),
            (HK_CLOSE, "关闭", &self.config.hotkey.close_hotkey),
        ] {
            match parse_hotkey(raw) {
                Ok(hk) => unsafe {
                    if !register(hwnd, id, &hk) {
                        eprintln!("{label}热键注册失败（可能已被占用）: {raw}");
                    }
                },
                Err(e) => eprintln!("{label}热键解析失败: {e}"),
            }
        }
    }

    fn unregister_hotkeys(&self, hwnd: HWND) {
        unsafe {
            let _ = UnregisterHotKey(Some(hwnd), HK_HIDE);
            let _ = UnregisterHotKey(Some(hwnd), HK_CLOSE);
        }
    }

    fn execute(&mut self, hwnd: HWND, cmd: Command) -> (Response, bool) {
        match cmd {
            Command::ReloadConfig => match Config::load(&self.config_path) {
                Ok(config) => {
                    self.unregister_hotkeys(hwnd);
                    self.config = config;
                    self.register_hotkeys(hwnd);
                    (Response::Ok, false)
                }
                Err(e) => (
                    Response::Error {
                        message: format!("重载配置失败: {e}"),
                    },
                    false,
                ),
            },
            Command::GetState => (
                Response::State {
                    hidden: self.controller.is_hidden(),
                },
                false,
            ),
            Command::Hide => {
                let config = self.config.clone();
                self.controller.hide(&config);
                (Response::Ok, false)
            }
            Command::Show => {
                self.controller.show();
                (Response::Ok, false)
            }
            Command::Toggle => {
                let config = self.config.clone();
                self.controller.toggle(&config);
                (Response::Ok, false)
            }
            Command::SetAutostart { enabled } => (set_autostart(enabled), false),
            Command::Quit => (Response::Ok, true),
        }
    }
}

unsafe fn register(hwnd: HWND, id: i32, hk: &ParsedHotkey) -> bool {
    unsafe {
        RegisterHotKey(
            Some(hwnd),
            id,
            HOT_KEY_MODIFIERS(hk.modifiers | MOD_NOREPEAT),
            hk.vk as u32,
        )
        .is_ok()
    }
}

fn set_autostart(enabled: bool) -> Response {
    let auto = match crate::autostart::Autostart::standard() {
        Ok(a) => a,
        Err(e) => {
            return Response::Error {
                message: e.to_string(),
            };
        }
    };
    if enabled {
        match auto.enable() {
            Ok(_) => Response::Ok,
            Err(e) => Response::Error {
                message: e.to_string(),
            },
        }
    } else {
        auto.disable();
        Response::Ok
    }
}

fn toggle_autostart(state: &AgentState) {
    let Ok(auto) = crate::autostart::Autostart::standard() else {
        return;
    };
    let (title, message) = if auto.status().is_some() {
        auto.disable();
        ("开机自启已关闭", "Boss Key 将不再随系统启动")
    } else {
        match auto.enable() {
            Ok(crate::autostart::Method::TaskScheduler) => {
                ("开机自启已开启", "已注册计划任务（最高权限）")
            }
            Ok(crate::autostart::Method::Registry) => ("开机自启已开启", "已写入注册表启动项"),
            Err(_) => ("开机自启设置失败", "计划任务与注册表方式均失败"),
        }
    };
    if let Some(tray) = &state.tray {
        tray.balloon(title, message);
    }
}

fn state_mut<'a>(hwnd: HWND) -> Option<&'a mut AgentState> {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AgentState;
        ptr.as_mut()
    }
}

fn show_tray_menu(hwnd: HWND, hidden: bool) -> bool {
    let autostart_on = crate::autostart::Autostart::standard()
        .map(|a| a.status().is_some())
        .unwrap_or(false);
    unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return false;
        };
        let toggle_label = if hidden {
            w!("显示窗口")
        } else {
            w!("隐藏窗口")
        };
        let autostart_flags = if autostart_on {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        let _ = AppendMenuW(menu, MF_STRING, MENU_SETTINGS, w!("设置"));
        let _ = AppendMenuW(menu, MF_STRING, MENU_TOGGLE, toggle_label);
        let _ = AppendMenuW(menu, autostart_flags, MENU_AUTOSTART, w!("开机自启"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, MENU_QUIT, w!("退出"));

        let mut pt = windows::Win32::Foundation::POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let shown = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            None,
            hwnd,
            None,
        )
        .as_bool();
        let _ = DestroyMenu(menu);
        shown
    }
}

fn launch_settings(state: &AgentState) {
    let config_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("bosskey-config.exe")));

    match config_exe {
        Some(path) if path.exists() => {
            let _ = std::process::Command::new(path).spawn();
        }
        _ => {
            if let Some(tray) = &state.tray {
                tray.balloon("Boss Key", "未找到配置程序 bosskey-config.exe");
            }
        }
    }
}

fn quit(state: &mut AgentState, hwnd: HWND) {
    state.controller.show();
    state.unregister_hotkeys(hwnd);
    if let Some(tray) = &mut state.tray {
        tray.balloon("Boss Key已停止服务", "Boss Key已成功退出");
        tray.hide();
    }
    unsafe {
        PostQuitMessage(0);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let Some(state) = state_mut(hwnd) else {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    };

    match msg {
        WM_HOTKEY => {
            match wparam.0 as i32 {
                HK_HIDE => {
                    let config = state.config.clone();
                    state.controller.toggle(&config);
                }
                HK_CLOSE => quit(state, hwnd),
                _ => {}
            }
            LRESULT(0)
        }
        WM_APP_IPC => {
            let mut should_quit = false;
            while let Ok((cmd, reply_tx)) = state.ipc_rx.try_recv() {
                let (response, quit_flag) = state.execute(hwnd, cmd);
                let _ = reply_tx.send(response);
                should_quit |= quit_flag;
            }
            if should_quit {
                quit(state, hwnd);
            }
            LRESULT(0)
        }
        WM_APP_TRAY => {
            match lparam.0 as u32 {
                WM_LBUTTONUP => {
                    if state.config.setting.click_to_hide {
                        let config = state.config.clone();
                        state.controller.toggle(&config);
                    }
                }
                WM_RBUTTONUP => {
                    show_tray_menu(hwnd, state.controller.is_hidden());
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            match wparam.0 & 0xFFFF {
                MENU_SETTINGS => launch_settings(state),
                MENU_TOGGLE => {
                    let config = state.config.clone();
                    state.controller.toggle(&config);
                }
                MENU_AUTOSTART => toggle_autostart(state),
                MENU_QUIT => quit(state, hwnd),
                _ => {}
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == AUTO_QUIT_TIMER_ID {
                unsafe {
                    let _ = KillTimer(Some(hwnd), AUTO_QUIT_TIMER_ID);
                }
                quit(state, hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn create_agent_window() -> Option<HWND> {
    unsafe {
        let hinstance = GetModuleHandleW(PCWSTR::null()).ok()?;
        let class_name = w!("BossKeyAgentWindow");
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
            w!("Boss Key"),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .ok()
    }
}

pub fn run(options: AgentOptions) {
    let config = Config::load(&options.config_path).unwrap_or_default();

    let Some(hwnd) = create_agent_window() else {
        eprintln!("创建代理窗口失败");
        return;
    };

    let tray = if options.enable_tray {
        Some(TrayIcon::new(hwnd, WM_APP_TRAY, "Boss Key"))
    } else {
        None
    };

    let (ipc_tx, ipc_rx) = channel::<(Command, Sender<Response>)>();

    let mut state = Box::new(AgentState {
        config,
        config_path: options.config_path.clone(),
        controller: HideController::new(WindowsWindowManager),
        tray,
        ipc_rx,
    });

    state.register_hotkeys(hwnd);

    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut *state as *mut AgentState as isize);
    }

    let hwnd_value = hwnd.0 as isize;
    ipc_server::spawn(options.pipe_name.clone(), move |cmd| {
        let (reply_tx, reply_rx) = channel::<Response>();
        if ipc_tx.send((cmd, reply_tx)).is_err() {
            return Response::Error {
                message: "核心已退出".to_string(),
            };
        }
        unsafe {
            let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);
            if PostMessageW(Some(hwnd), WM_APP_IPC, WPARAM(0), LPARAM(0)).is_err() {
                return Response::Error {
                    message: "无法通知核心".to_string(),
                };
            }
        }
        reply_rx
            .recv_timeout(IPC_REPLY_TIMEOUT)
            .unwrap_or(Response::Error {
                message: "核心响应超时".to_string(),
            })
    });

    if let Some(tray) = &state.tray {
        tray.balloon(
            "Boss Key正在运行！",
            "Boss Key正在为您服务，您可通过托盘图标看到我",
        );
    }

    if let Some(ms) = options.auto_quit_ms {
        unsafe {
            SetTimer(Some(hwnd), AUTO_QUIT_TIMER_ID, ms, None);
        }
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

        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        let _ = DestroyWindow(hwnd);
    }

    drop(state);
}
