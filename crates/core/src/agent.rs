use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use bosskey_common::ipc::{Command, Response};
use bosskey_common::{ARG_ABOUT, ARG_RESTORE, Config};
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

use crate::effects::WinEffects;
use crate::float_window::{FLOAT_MENU, FLOAT_TOGGLE, FloatWindow, WM_APP_FLOAT};
use crate::hide::{
    HideController, RuleOutcome, expand_descendants, freezable_pids, resolve_targets,
};
use crate::hotkey::{MOD_NOREPEAT, ParsedHotkey, parse_hotkey};
use crate::mouse_hook::{self, MouseHook, TRIGGER_CORNER, WM_MOUSE_TRIGGER};
use crate::platform::win32::WindowsWindowManager;
use crate::tray::TrayIcon;
use crate::{idle, ipc_server, logging, recovery};

const HK_HIDE: i32 = 1;
const HK_CLOSE: i32 = 2;

const WM_APP_IPC: u32 = WM_APP + 1;
const WM_APP_TRAY: u32 = WM_APP + 2;

const MENU_SETTINGS: usize = 1001;
const MENU_TOGGLE: usize = 1002;
const MENU_QUIT: usize = 1003;
const MENU_AUTOSTART: usize = 1004;
const MENU_RESTORE: usize = 1005;
const MENU_ABOUT: usize = 1006;

const AUTO_QUIT_TIMER_ID: usize = 10;
const AUTO_HIDE_TIMER_ID: usize = 11;
/// 监控停用看门狗：配置程序停用监控后须持续心跳，超时未收到就恢复监控。
const SUSPEND_GUARD_TIMER_ID: usize = 12;
const AUTO_HIDE_INTERVAL_MS: u32 = 5000;
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
    recovery_path: PathBuf,
    controller: HideController<WindowsWindowManager, WinEffects>,
    tray: Option<TrayIcon>,
    ipc_rx: Receiver<(Command, Sender<Response>)>,
    mouse_hook: Option<MouseHook>,
    float_window: Option<FloatWindow>,
    /// 是否正在监听热键与鼠标（见 `Command::SetHotkeys`）。
    monitoring: bool,
    /// 热键当前是否已注册（避免重复 RegisterHotKey）。
    hotkeys_armed: bool,
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
                        logging::warn(&format!("{label}热键注册失败（可能已被占用）: {raw}"));
                    }
                },
                Err(e) => logging::warn(&format!("{label}热键解析失败: {e}")),
            }
        }
    }

    fn unregister_hotkeys(&self, hwnd: HWND) {
        unsafe {
            let _ = UnregisterHotKey(Some(hwnd), HK_HIDE);
            let _ = UnregisterHotKey(Some(hwnd), HK_CLOSE);
        }
    }

    /// 让热键与鼠标钩子对齐 `self.monitoring`。幂等：已是目标状态时什么都不做。
    fn sync_monitoring(&mut self, hwnd: HWND) {
        if self.monitoring && !self.hotkeys_armed {
            self.register_hotkeys(hwnd);
            self.hotkeys_armed = true;
        } else if !self.monitoring && self.hotkeys_armed {
            self.unregister_hotkeys(hwnd);
            self.hotkeys_armed = false;
        }
        self.refresh_runtime(hwnd);
    }

    fn refresh_runtime(&mut self, hwnd: HWND) {
        // 监控停用期间不装鼠标钩子。
        if self.monitoring && mouse_hook::wants_hook(&self.config.setting) {
            mouse_hook::set_flags(&self.config.setting);
            if self.mouse_hook.is_none() {
                self.mouse_hook = MouseHook::install(hwnd, &self.config.setting);
            }
        } else {
            self.mouse_hook = None;
        }

        unsafe {
            let _ = KillTimer(Some(hwnd), AUTO_HIDE_TIMER_ID);
            if self.config.setting.auto_hide_enabled {
                SetTimer(Some(hwnd), AUTO_HIDE_TIMER_ID, AUTO_HIDE_INTERVAL_MS, None);
            }
        }

        if self.config.setting.show_float_window {
            if self.float_window.is_none() {
                self.float_window = FloatWindow::create(hwnd);
            }
        } else {
            self.float_window = None;
        }
    }

    fn sync_tray(&mut self) {
        if !self.config.setting.hide_icon_after_hide {
            return;
        }
        let hidden = self.controller.is_hidden();
        if let Some(tray) = &mut self.tray {
            if hidden {
                tray.hide();
            } else {
                tray.show();
            }
        }
    }

    /// 隐藏状态落盘：崩溃后下次启动据此找回窗口。快照为空时清除文件。
    fn persist_recovery(&self) {
        if let Err(e) = recovery::save(&self.recovery_path, &self.controller.snapshot()) {
            logging::warn(&format!("写入崩溃恢复文件失败: {e}"));
        }
    }

    fn apply_hide(&mut self) {
        let windows = self.controller.enumerate();
        let foreground = self.controller.foreground();
        let (targets, outcomes) = resolve_targets(&mut self.config, &windows, foreground);
        let freezable = freezable_pids(&targets, &windows);
        // 「冻结完整进程」：把可冻结集展开到整棵子进程树。
        let freeze_set = if self.config.setting.freeze_whole_tree {
            expand_descendants(&freezable, &crate::freeze::process_tree())
        } else {
            freezable
        };
        log_resolution(&self.config, &outcomes, &targets);
        self.controller
            .apply_hide(&self.config.setting, &targets, &freeze_set);
        self.persist_recovery();
        self.sync_tray();
        if self.config.notifications.on_hide {
            self.balloon("Boss Key", "已隐藏窗口");
        }
    }

    fn apply_show(&mut self) {
        self.controller.show();
        logging::info("已显示先前隐藏的窗口");
        self.persist_recovery();
        self.sync_tray();
        if self.config.notifications.on_show {
            self.balloon("Boss Key", "已恢复显示窗口");
        }
    }

    /// 发送托盘气泡（托盘不存在或已隐藏时静默忽略）。
    fn balloon(&self, title: &str, message: &str) {
        if let Some(tray) = &self.tray {
            tray.balloon(title, message);
        }
    }

    fn apply_toggle(&mut self) {
        if self.controller.is_hidden() {
            self.apply_show();
        } else {
            self.apply_hide();
        }
    }

    fn execute(&mut self, hwnd: HWND, cmd: Command) -> (Response, bool) {
        match cmd {
            Command::ReloadConfig => match Config::load(&self.config_path) {
                Ok(config) => {
                    // 先摘掉旧热键，能否重装由 sync_monitoring 决定。
                    if self.hotkeys_armed {
                        self.unregister_hotkeys(hwnd);
                        self.hotkeys_armed = false;
                    }
                    self.config = config;
                    self.sync_monitoring(hwnd);
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
            Command::GetElevation => (
                Response::Elevated {
                    elevated: crate::elevation::is_elevated(),
                },
                false,
            ),
            Command::GetStatus => (
                Response::Status {
                    hidden: self.controller.is_hidden(),
                    elevated: crate::elevation::is_elevated(),
                    monitoring: self.monitoring,
                },
                false,
            ),
            Command::Hide => {
                self.apply_hide();
                (Response::Ok, false)
            }
            Command::Show => {
                self.apply_show();
                (Response::Ok, false)
            }
            Command::Toggle => {
                self.apply_toggle();
                (Response::Ok, false)
            }
            Command::SetAutostart { enabled } => (set_autostart(enabled), false),
            // 停用有状态，期间的 ReloadConfig 不会复活它；看门狗定时器兜底超时恢复。
            Command::SetHotkeys { enabled } => {
                let changed = self.monitoring != enabled;
                self.monitoring = enabled;
                self.sync_monitoring(hwnd);
                unsafe {
                    let _ = KillTimer(Some(hwnd), SUSPEND_GUARD_TIMER_ID);
                    if !enabled {
                        SetTimer(
                            Some(hwnd),
                            SUSPEND_GUARD_TIMER_ID,
                            bosskey_common::ipc::SUSPEND_TIMEOUT_MS,
                            None,
                        );
                    }
                }
                if changed {
                    logging::debug(if enabled {
                        "已恢复热键与鼠标监控"
                    } else {
                        "配置中：已临时停用热键与鼠标监控"
                    });
                }
                (Response::Ok, false)
            }
            Command::Quit => (Response::Ok, true),
        }
    }
}

/// 记录本次隐藏的分级日志。
fn log_resolution(config: &Config, outcomes: &[RuleOutcome], targets: &[crate::hide::Target]) {
    logging::info(&format!(
        "触发隐藏：命中 {} 个窗口（窗口规则 {} 条 / 进程规则 {} 条）",
        targets.len(),
        config.window_rules.len(),
        config.process_rules.len()
    ));
    for (rule, outcome) in config.window_rules.iter().zip(outcomes) {
        match outcome {
            RuleOutcome::Reacquired => {
                logging::info(&format!("窗口「{}」句柄已变化，已追溯回填", rule.title))
            }
            RuleOutcome::Missing => {
                logging::warn(&format!("窗口「{}」已关闭或重启，未能追溯", rule.title))
            }
            _ => {}
        }
    }
    for t in targets {
        logging::debug(&format!("隐藏目标 hwnd={} pid={}", t.hwnd, t.pid));
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
    if state.config.notifications.on_autostart
        && let Some(tray) = &state.tray
    {
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
        let _ = AppendMenuW(menu, MF_STRING, MENU_RESTORE, w!("窗口恢复工具"));
        let _ = AppendMenuW(menu, autostart_flags, MENU_AUTOSTART, w!("开机自启"));
        let _ = AppendMenuW(menu, MF_STRING, MENU_ABOUT, w!("关于"));
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

/// 两个路径是否指向同一个可执行文件（Windows 路径大小写不敏感）。
fn same_exe(path: &str, exe: &Path) -> bool {
    exe.to_str()
        .is_some_and(|e| !path.is_empty() && path.eq_ignore_ascii_case(e))
}

/// 把配置程序自己的窗口从「被隐藏」状态里放出来，再拉起设置。
fn release_config_windows(state: &mut AgentState, exe: &Path) {
    let windows = state.controller.enumerate();
    let mut pids: Vec<u32> = windows
        .iter()
        .filter(|w| same_exe(&w.path, exe))
        .map(|w| w.pid)
        .filter(|pid| *pid != 0)
        .collect();
    pids.sort_unstable();
    pids.dedup();

    let released = state.controller.release_pids(&pids);
    if released > 0 {
        logging::info(&format!(
            "配置窗口此前被 Boss Key 隐藏，已先释放 {released} 个窗口再拉起设置"
        ));
        state.persist_recovery();
        state.sync_tray();
    }
}

/// 拉起配置程序。`action` 会作为命令行参数传入（如 `restore` 直达窗口恢复工具）。
fn launch_settings(state: &mut AgentState, action: Option<&str>) {
    // 生产名 "Boss Key.exe"，开发名 "bosskey-config.exe"。
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from));
    let found = dir.and_then(|d| {
        ["Boss Key.exe", "bosskey-config.exe"]
            .into_iter()
            .map(|name| d.join(name))
            .find(|p| p.exists())
    });

    let Some(path) = found else {
        if let Some(tray) = &state.tray {
            tray.balloon("Boss Key", "未找到配置程序");
        }
        return;
    };

    release_config_windows(state, &path);

    let mut cmd = std::process::Command::new(&path);
    if let Some(arg) = action {
        cmd.arg(arg);
    }
    if let Err(e) = cmd.spawn() {
        logging::warn(&format!("启动配置程序失败: {e}"));
    }
}

fn quit(state: &mut AgentState, hwnd: HWND) {
    state.controller.show();
    state.persist_recovery();
    state.unregister_hotkeys(hwnd);
    logging::info("核心正常退出");
    let notify_quit = state.config.notifications.on_quit;
    if let Some(tray) = &mut state.tray {
        if notify_quit {
            tray.balloon("Boss Key已停止服务", "Boss Key已成功退出");
        }
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
                HK_HIDE => state.apply_toggle(),
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
                        state.apply_toggle();
                    }
                }
                WM_RBUTTONUP => {
                    show_tray_menu(hwnd, state.controller.is_hidden());
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_APP_FLOAT => {
            match wparam.0 {
                // 双击悬浮窗触发老板键。
                FLOAT_TOGGLE => {
                    if state.config.setting.click_to_hide {
                        state.apply_toggle();
                    }
                }
                // 右键菜单以代理窗口为宿主，经 WM_COMMAND 复用处理。
                FLOAT_MENU => {
                    crate::float_window::show_float_menu(hwnd, MENU_SETTINGS, MENU_QUIT);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            match wparam.0 & 0xFFFF {
                MENU_SETTINGS => launch_settings(state, None),
                MENU_RESTORE => launch_settings(state, Some(ARG_RESTORE)),
                MENU_ABOUT => launch_settings(state, Some(ARG_ABOUT)),
                MENU_TOGGLE => state.apply_toggle(),
                MENU_AUTOSTART => toggle_autostart(state),
                MENU_QUIT => quit(state, hwnd),
                _ => {}
            }
            LRESULT(0)
        }
        WM_MOUSE_TRIGGER => {
            // 隐藏一律生效，恢复取决于各自的开关。
            let allow_restore = if wparam.0 == TRIGGER_CORNER {
                state.config.setting.allow_move_restore
            } else {
                state.config.setting.mouse.allow_click_restore
            };
            if state.controller.is_hidden() {
                if allow_restore {
                    state.apply_show();
                }
            } else {
                state.apply_hide();
            }
            LRESULT(0)
        }
        WM_TIMER => {
            match wparam.0 {
                AUTO_QUIT_TIMER_ID => {
                    unsafe {
                        let _ = KillTimer(Some(hwnd), AUTO_QUIT_TIMER_ID);
                    }
                    quit(state, hwnd);
                }
                // 停用心跳超时，恢复监控。
                SUSPEND_GUARD_TIMER_ID => {
                    unsafe {
                        let _ = KillTimer(Some(hwnd), SUSPEND_GUARD_TIMER_ID);
                    }
                    if !state.monitoring {
                        logging::warn("配置程序长时间未续期监控停用，已自动恢复热键与鼠标监控");
                        state.monitoring = true;
                        state.sync_monitoring(hwnd);
                    }
                }
                AUTO_HIDE_TIMER_ID => {
                    if state.config.setting.auto_hide_enabled {
                        let idle_ms = idle::idle_millis().unwrap_or(0);
                        if idle::should_auto_hide(
                            idle_ms,
                            state.config.setting.auto_hide_time,
                            state.controller.is_hidden(),
                        ) {
                            state.apply_hide();
                        }
                    }
                }
                _ => {}
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

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));

    let recovery_path = options
        .config_path
        .with_file_name(recovery::RECOVERY_FILE_NAME);

    let mut state = Box::new(AgentState {
        config,
        config_path: options.config_path.clone(),
        recovery_path,
        controller: HideController::new(WindowsWindowManager, WinEffects::new(exe_dir)),
        tray,
        ipc_rx,
        mouse_hook: None,
        float_window: None,
        monitoring: true,
        hotkeys_armed: false,
    });

    // 恢复文件存在即上次异常退出仍有窗口被隐藏，先找回。
    if let Some(snapshot) = recovery::load(&state.recovery_path) {
        logging::warn(&format!(
            "检测到上次异常退出，正在恢复 {} 个被隐藏的窗口",
            snapshot.hidden.len()
        ));
        state.controller.restore_from(snapshot);
        recovery::clear(&state.recovery_path);
    }

    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut *state as *mut AgentState as isize);
    }

    state.sync_monitoring(hwnd);

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

    if state.config.notifications.on_start
        && let Some(tray) = &state.tray
    {
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
