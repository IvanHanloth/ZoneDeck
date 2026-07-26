use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use bosskey_common::ipc::{Command, Response};
use bosskey_common::{APP_NAME, ARG_ABOUT, ARG_RESTORE, Config};
use windows::Win32::Foundation::{ERROR_HOTKEY_ALREADY_REGISTERED, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CW_USEDEFAULT, ChangeWindowMessageFilterEx, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetCursorPos,
    GetMessageW, GetWindowLongPtrW, KillTimer, MF_CHECKED, MF_SEPARATOR, MF_STRING, MSG,
    MSGFLT_ALLOW, PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow, SetTimer,
    SetWindowLongPtrW, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TrackPopupMenu, TranslateMessage,
    WINDOW_EX_STYLE, WM_APP, WM_COMMAND, WM_DESTROY, WM_HOTKEY, WM_LBUTTONUP, WM_RBUTTONUP,
    WM_TIMER, WNDCLASSW, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

use crate::effects::WinEffects;
use crate::effects_worker::{AsyncEffects, EffectsWorker};
use crate::float_window::{FLOAT_MENU, FLOAT_TOGGLE, FloatWindow, WM_APP_FLOAT};
use crate::hide::{
    HideController, HidePlan, RuleOutcome, ShowOutcome, expand_descendants, foreground_target,
    freezable_pids, resolve_targets,
};
use crate::hotkey::{MOD_NOREPEAT, ParsedHotkey, is_disabled, parse_hotkey};
use crate::i18n::{self, Msg};
use crate::keyboard_hook::{self, KeyboardHook, WM_KEY_TRIGGER};
use crate::mouse_hook::{self, MouseHook, TRIGGER_CORNER, WM_MOUSE_TRIGGER};
use crate::platform::win32::WindowsWindowManager;
use crate::tray::TrayIcon;
use crate::tray_badge::TrayIconSet;
use crate::util::append_menu_item;
use crate::{idle, ipc_server, log_error, log_warn, logging, recovery};

const HK_HIDE: i32 = 1;
const HK_CLOSE: i32 = 2;
const HK_HIDE_ONLY: i32 = 3;
const HK_SHOW_ONLY: i32 = 4;
const HK_HIDE_FOREGROUND: i32 = 5;

const WM_APP_IPC: u32 = WM_APP + 1;
const WM_APP_TRAY: u32 = WM_APP + 2;

const MENU_SETTINGS: usize = 1001;
const MENU_TOGGLE: usize = 1002;
const MENU_QUIT: usize = 1003;
const MENU_AUTOSTART: usize = 1004;
const MENU_RESTORE: usize = 1005;
const MENU_ABOUT: usize = 1006;
const MENU_AUTO_HIDE: usize = 1007;

const AUTO_QUIT_TIMER_ID: usize = 10;
const AUTO_HIDE_TIMER_ID: usize = 11;
/// 监控停用看门狗：配置程序停用监控后须持续心跳，超时未收到就恢复监控。
const SUSPEND_GUARD_TIMER_ID: usize = 12;
/// 托盘图标挂载重试：开机计划任务常早于任务栏启动，首挂会失败，须兜底重试。
const TRAY_RETRY_TIMER_ID: usize = 13;
const AUTO_HIDE_INTERVAL_MS: u32 = 5000;
const TRAY_RETRY_INTERVAL_MS: u32 = 2000;
const IPC_REPLY_TIMEOUT: Duration = Duration::from_secs(3);
/// 退出时等待副作用线程排干队列（解冻 / 取消静音）的上限。
const EFFECTS_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

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
    controller: HideController<WindowsWindowManager, AsyncEffects>,
    /// 副作用专职线程；退出时须 shutdown 排干队列。
    effects_worker: Option<EffectsWorker>,
    /// 恢复文件写入失败是否已提醒过（每次运行只弹一次气泡）。
    persist_warned: bool,
    tray: Option<TrayIcon>,
    /// 托盘图标的角标变体缓存；未启用托盘时为 None。
    tray_icons: Option<TrayIconSet>,
    ipc_rx: Receiver<(Command, Sender<Response>)>,
    mouse_hook: Option<MouseHook>,
    float_window: Option<FloatWindow>,
    /// 是否正在监听热键与鼠标（见 `Command::SetHotkeys`）。
    monitoring: bool,
    /// 热键当前是否已注册（避免重复 RegisterHotKey）。
    hotkeys_armed: bool,
    /// 承载「不传递」热键的键盘钩子；没有热键开启「不传递」时不安装。
    keyboard_hook: Option<KeyboardHook>,
}

impl AgentState {
    fn register_hotkeys(&mut self, hwnd: HWND) {
        let mut intercepts: Vec<(i32, &'static str, String, ParsedHotkey)> = Vec::new();
        for (id, label, raw, intercept) in [
            (
                HK_HIDE,
                "隐藏",
                &self.config.hotkey.hide_hotkey,
                self.config.hotkey.hide_intercept,
            ),
            (
                HK_CLOSE,
                "关闭",
                &self.config.hotkey.close_hotkey,
                self.config.hotkey.close_intercept,
            ),
            (
                HK_HIDE_ONLY,
                "仅隐藏",
                &self.config.hotkey.hide_only_hotkey,
                self.config.hotkey.hide_only_intercept,
            ),
            (
                HK_SHOW_ONLY,
                "仅显示",
                &self.config.hotkey.show_only_hotkey,
                self.config.hotkey.show_only_intercept,
            ),
            (
                HK_HIDE_FOREGROUND,
                "隐藏前台窗口",
                &self.config.hotkey.hide_foreground_hotkey,
                self.config.hotkey.hide_foreground_intercept,
            ),
        ] {
            if is_disabled(raw) {
                logging::debug(&format!("{label}热键已置空，不注册"));
                continue;
            }
            match parse_hotkey(raw) {
                Ok(hk) if intercept => intercepts.push((id, label, raw.clone(), hk)),
                Ok(hk) => unsafe {
                    match register(hwnd, id, &hk) {
                        Ok(()) => logging::debug(&format!("{label}热键已注册: {raw}")),
                        Err(e) => log_warn!("{}", hotkey_failure_message(label, raw, &e)),
                    }
                },
                Err(e) => log_warn!("{label}热键解析失败，该热键不生效: {raw} — {e}"),
            }
        }
        self.arm_intercepts(hwnd, intercepts);
    }

    /// 把开启「不传递」的热键装载进键盘钩子。
    ///
    /// 钩子安装失败时回退 `RegisterHotKey`：热键仍可用，只是无法阻止按键传递。
    fn arm_intercepts(
        &mut self,
        hwnd: HWND,
        intercepts: Vec<(i32, &'static str, String, ParsedHotkey)>,
    ) {
        if intercepts.is_empty() {
            keyboard_hook::set_hotkeys(&[]);
            self.keyboard_hook = None;
            return;
        }
        let parsed: Vec<(i32, ParsedHotkey)> =
            intercepts.iter().map(|(id, _, _, hk)| (*id, *hk)).collect();
        keyboard_hook::set_hotkeys(&parsed);
        if self.keyboard_hook.is_none() {
            self.keyboard_hook = KeyboardHook::install(hwnd);
        }
        if self.keyboard_hook.is_some() {
            for (_, label, raw, _) in &intercepts {
                logging::debug(&format!("{label}热键已由键盘钩子拦截（不传递）: {raw}"));
            }
            return;
        }
        keyboard_hook::set_hotkeys(&[]);
        log_warn!("键盘钩子安装失败，「不传递」不生效，相关热键回退为普通注册");
        for (id, label, raw, hk) in &intercepts {
            unsafe {
                match register(hwnd, *id, hk) {
                    Ok(()) => logging::debug(&format!("{label}热键已注册: {raw}")),
                    Err(e) => log_warn!("{}", hotkey_failure_message(label, raw, &e)),
                }
            }
        }
    }

    fn unregister_hotkeys(&mut self, hwnd: HWND) {
        unsafe {
            for id in [
                HK_HIDE,
                HK_CLOSE,
                HK_HIDE_ONLY,
                HK_SHOW_ONLY,
                HK_HIDE_FOREGROUND,
            ] {
                let _ = UnregisterHotKey(Some(hwnd), id);
            }
        }
        keyboard_hook::set_hotkeys(&[]);
        self.keyboard_hook = None;
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

        self.update_tray_icon();
    }

    fn sync_tray(&mut self) {
        let hidden = self.controller.is_hidden();
        if self.config.setting.hide_icon_after_hide
            && let Some(tray) = &mut self.tray
        {
            if hidden {
                tray.hide();
            } else {
                tray.show();
            }
        }
        self.update_tray_icon();
    }

    /// 让托盘图标的状态角标与悬浮提示对齐当前配置与运行状态。均未变化时开销为零。
    fn update_tray_icon(&mut self) {
        let (Some(tray), Some(icons)) = (&mut self.tray, &mut self.tray_icons) else {
            return;
        };
        let status = crate::tray_badge::TrayStatus {
            hidden: self.controller.is_hidden(),
            auto_hide: self.config.setting.auto_hide_enabled,
            hide_current: self.config.setting.hide_current,
            freeze: self.config.setting.freeze_after_hide,
            elevated: crate::elevation::is_elevated(),
            monitor_paused: !self.monitoring,
        };
        let badge = crate::tray_badge::active_badge(&self.config.setting.tray_badges, &status);
        tray.set_icon(icons.icon(badge));
        tray.set_tip(if self.config.setting.tray_show_tooltip {
            APP_NAME
        } else {
            ""
        });
    }

    /// 把指定快照落盘。失败不阻断本次隐藏，只影响崩溃找回；每次运行提醒用户一次。
    fn persist_snapshot(&mut self, snapshot: &recovery::Snapshot) {
        if let Err(e) = recovery::save(&self.recovery_path, snapshot) {
            log_error!(
                "写入崩溃恢复文件失败，核心若异常退出将无法自动找回窗口: {} — {e}",
                self.recovery_path.display()
            );
            if !self.persist_warned {
                self.persist_warned = true;
                self.balloon(APP_NAME, i18n::t(Msg::RecoveryPersistFailedBody));
            }
        }
    }

    /// 按控制器当前状态落盘。
    fn persist_recovery(&mut self) {
        let snapshot = self.controller.snapshot();
        self.persist_snapshot(&snapshot);
    }

    /// 意图先行：先落盘计划后的快照，再隐藏窗口；副作用由专职线程异步执行。
    fn hide_with_plan(&mut self, targets: &[crate::hide::Target], freeze_set: &[u32]) -> HidePlan {
        let plan = self
            .controller
            .plan_hide(&self.config.setting, targets, freeze_set);
        let planned = self.controller.planned_snapshot(&plan);
        self.persist_snapshot(&planned);
        self.controller.commit_hide(plan.clone());
        self.sync_tray();
        if self.config.notifications.on_hide && !plan.fresh.is_empty() {
            self.balloon(APP_NAME, i18n::t(Msg::HiddenBody));
        }
        plan
    }

    /// 「冻结完整进程」开启时把可冻结集展开到整棵子进程树。
    fn freeze_set(&self, freezable: Vec<u32>) -> Vec<u32> {
        if self.config.setting.freeze_whole_tree {
            expand_descendants(&freezable, &crate::freeze::process_tree())
        } else {
            freezable
        }
    }

    fn apply_hide(&mut self) {
        let windows = self.controller.enumerate();
        let foreground = self.controller.foreground();
        let (targets, outcomes) = resolve_targets(&mut self.config, &windows, foreground);
        let freeze_set = self.freeze_set(freezable_pids(&targets, &windows));
        let plan = self.hide_with_plan(&targets, &freeze_set);
        log_hide(&self.config, &outcomes, &plan);
    }

    fn apply_hide_foreground(&mut self) {
        let windows = self.controller.enumerate();
        let foreground = self.controller.foreground();
        let Some(target) = foreground_target(&windows, foreground) else {
            logging::info("隐藏前台窗口：当前没有可隐藏的前台窗口");
            return;
        };

        let targets = [target];
        let freeze_set = self.freeze_set(freezable_pids(&targets, &windows));
        let plan = self.hide_with_plan(&targets, &freeze_set);
        if let Some(t) = plan.fresh.first() {
            logging::info(&format!("隐藏前台窗口: {}", t.describe()));
        }
    }

    fn apply_show(&mut self) {
        let outcome = self.controller.show();
        log_show(outcome);
        self.persist_recovery();
        self.sync_tray();
        if self.config.notifications.on_show {
            self.balloon(APP_NAME, i18n::t(Msg::ShownBody));
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
                    i18n::set_from_pref(&self.config.setting.language);
                    self.sync_monitoring(hwnd);
                    (Response::Ok, false)
                }
                Err(e) => (
                    Response::Error {
                        message: i18n::tf(Msg::ErrReloadConfig, &[("err", &e.to_string())]),
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
                    auto_hide_enabled: self.config.setting.auto_hide_enabled,
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
            Command::SetAutostart { enabled, admin } => (set_autostart(enabled, admin), false),
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

/// 摘要式记录本次隐藏；规则失效 / 追溯事件逐条保留。
fn log_hide(config: &Config, outcomes: &[RuleOutcome], plan: &HidePlan) {
    for (rule, outcome) in config.window_rules.iter().zip(outcomes) {
        match outcome {
            RuleOutcome::Reacquired => logging::info(&format!(
                "窗口规则「{}」的句柄已失效（目标程序重启过），已重新匹配并更新规则",
                rule.title
            )),
            // 「未能追溯」只说明当前没有匹配的窗口，看不出是关闭了还是标题变了，故不臆断原因。
            RuleOutcome::Missing => logging::warn(&format!(
                "窗口规则「{}」未匹配到任何窗口（可能已关闭或标题已变），本次不隐藏它",
                rule.title
            )),
            _ => {}
        }
    }
    if plan.fresh.is_empty() {
        logging::info("触发隐藏：没有新的目标窗口");
        return;
    }
    logging::info(&format!(
        "隐藏 {} 个窗口: {}",
        plan.fresh.len(),
        summarize(plan.fresh.iter().map(|t| t.describe()))
    ));
    if !plan.freeze.is_empty() {
        logging::info(&format!(
            "冻结 {} 个进程（增强={}）: {}",
            plan.freeze.len(),
            plan.enhanced,
            summarize(plan.freeze.iter().map(|r| r.pid.to_string()))
        ));
    }
}

/// 记录恢复结果；有失效记录被跳过时如实写出。
fn log_show(outcome: ShowOutcome) {
    if outcome.stale > 0 {
        logging::info(&format!(
            "恢复显示 {} 个窗口，另有 {} 条记录已失效（窗口已关闭或句柄被复用），已跳过",
            outcome.shown, outcome.stale
        ));
    } else {
        logging::info(&format!("恢复显示 {} 个窗口", outcome.shown));
    }
}

/// 拼接清单，最多列出前 8 项，其余以「等 N 项」收尾。
fn summarize(items: impl Iterator<Item = String>) -> String {
    const MAX: usize = 8;
    let all: Vec<String> = items.collect();
    if all.len() <= MAX {
        all.join("、")
    } else {
        format!("{}、等 {} 项", all[..MAX].join("、"), all.len())
    }
}

unsafe fn register(hwnd: HWND, id: i32, hk: &ParsedHotkey) -> windows::core::Result<()> {
    unsafe {
        RegisterHotKey(
            Some(hwnd),
            id,
            HOT_KEY_MODIFIERS(hk.modifiers | MOD_NOREPEAT),
            hk.vk as u32,
        )
    }
}

/// 热键注册失败的日志文案。
///
/// 「已被占用」（1409）是最常见的原因，但并非唯一，故按系统错误码区分：
/// 命中 1409 才断言被占用，其余情况如实报告错误码，不作猜测。
fn hotkey_failure_message(label: &str, raw: &str, e: &windows::core::Error) -> String {
    if e.code() == ERROR_HOTKEY_ALREADY_REGISTERED.to_hresult() {
        format!("{label}热键注册失败，已被其他程序占用，该热键不生效: {raw}")
    } else {
        format!(
            "{label}热键注册失败，该热键不生效: {raw} — {}",
            crate::util::win_err(e)
        )
    }
}

fn set_autostart(enabled: bool, admin: bool) -> Response {
    let auto = match crate::autostart::Autostart::standard() {
        Ok(a) => a,
        Err(e) => {
            return Response::Error {
                message: e.to_string(),
            };
        }
    };
    if enabled {
        match auto.enable(admin) {
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

/// 托盘菜单切换自动隐藏：翻转配置并落盘，与在设置界面切换等效。
/// 设置界面经 2 秒一次的状态轮询回读该值，保持两侧一致。
fn toggle_auto_hide(state: &mut AgentState, hwnd: HWND) {
    let enabled = !state.config.setting.auto_hide_enabled;
    state.config.setting.auto_hide_enabled = enabled;
    logging::info(if enabled {
        "托盘菜单：已启用自动隐藏"
    } else {
        "托盘菜单：已暂停自动隐藏"
    });
    if let Err(e) = state.config.save(&state.config_path) {
        log_warn!("自动隐藏开关写入配置失败，核心重启后将丢失本次切换: {e}");
    }
    // 重新武装/停掉空闲检测定时器，并刷新托盘角标。
    state.refresh_runtime(hwnd);
}

fn toggle_autostart(state: &AgentState) {
    let Ok(auto) = crate::autostart::Autostart::standard() else {
        return;
    };
    let admin = state.config.setting.autostart_admin;
    let (title, message) = if auto.status().is_some() {
        auto.disable();
        (Msg::AutostartOffTitle, Msg::AutostartOffBody)
    } else {
        match auto.enable(admin) {
            Ok(crate::autostart::Method::TaskScheduler) if admin => {
                (Msg::AutostartOnTitle, Msg::AutostartOnTaskAdmin)
            }
            Ok(crate::autostart::Method::TaskScheduler) => {
                (Msg::AutostartOnTitle, Msg::AutostartOnTaskUser)
            }
            Ok(crate::autostart::Method::Registry) => {
                (Msg::AutostartOnTitle, Msg::AutostartOnRegistry)
            }
            Err(_) => (Msg::AutostartFailTitle, Msg::AutostartFailBody),
        }
    };
    if state.config.notifications.on_autostart
        && let Some(tray) = &state.tray
    {
        tray.balloon(i18n::t(title), i18n::t(message));
    }
}

fn state_mut<'a>(hwnd: HWND) -> Option<&'a mut AgentState> {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AgentState;
        ptr.as_mut()
    }
}

fn show_tray_menu(hwnd: HWND, hidden: bool, auto_hide_on: bool) -> bool {
    let autostart_on = crate::autostart::Autostart::standard()
        .map(|a| a.status().is_some())
        .unwrap_or(false);
    unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return false;
        };
        let toggle_label = if hidden {
            Msg::MenuShowWindows
        } else {
            Msg::MenuHideWindows
        };
        let checked = |on: bool| {
            if on {
                MF_STRING | MF_CHECKED
            } else {
                MF_STRING
            }
        };
        append_menu_item(menu, MF_STRING, MENU_SETTINGS, Msg::MenuSettings);
        append_menu_item(menu, MF_STRING, MENU_TOGGLE, toggle_label);
        append_menu_item(menu, MF_STRING, MENU_RESTORE, Msg::MenuRestoreTool);
        append_menu_item(
            menu,
            checked(auto_hide_on),
            MENU_AUTO_HIDE,
            Msg::MenuAutoHide,
        );
        append_menu_item(
            menu,
            checked(autostart_on),
            MENU_AUTOSTART,
            Msg::MenuAutostart,
        );
        append_menu_item(menu, MF_STRING, MENU_ABOUT, Msg::MenuAbout);
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        append_menu_item(menu, MF_STRING, MENU_QUIT, Msg::MenuQuit);

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

/// 定位同目录下的配置程序：生产名 config.exe，开发名 bosskey-config.exe。
fn find_config_exe() -> Option<PathBuf> {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))?;
    ["config.exe", "bosskey-config.exe"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|p| p.exists())
}

/// 拉起配置程序。`action` 会作为命令行参数传入（如 `restore` 直达窗口恢复工具）。
fn launch_settings(state: &mut AgentState, action: Option<&str>) {
    let Some(path) = find_config_exe() else {
        if let Some(tray) = &state.tray {
            tray.balloon(APP_NAME, i18n::t(Msg::ConfigExeMissing));
        }
        return;
    };

    release_config_windows(state, &path);

    let mut cmd = std::process::Command::new(&path);
    if let Some(arg) = action {
        cmd.arg(arg);
    }
    if let Err(e) = cmd.spawn() {
        log_warn!("启动配置程序失败: {e}");
    }
}

fn quit(state: &mut AgentState, hwnd: HWND) {
    let outcome = state.controller.show();
    if outcome.shown > 0 || outcome.stale > 0 {
        log_show(outcome);
    }
    state.persist_recovery();
    state.unregister_hotkeys(hwnd);
    // 排干副作用队列，确保退出前解冻 / 取消静音已生效。
    if let Some(worker) = state.effects_worker.take() {
        worker.shutdown(EFFECTS_SHUTDOWN_TIMEOUT);
    }
    logging::info("核心正常退出");
    let notify_quit = state.config.notifications.on_quit;
    if let Some(tray) = &mut state.tray {
        if notify_quit {
            tray.balloon(i18n::t(Msg::QuitTitle), i18n::t(Msg::QuitBody));
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
    if msg != 0 && msg == crate::tray::taskbar_created_msg() {
        if let Some(tray) = &mut state.tray {
            tray.on_taskbar_created();
        }
        return LRESULT(0);
    }

    match msg {
        // WM_KEY_TRIGGER 是「不传递」热键经键盘钩子转发的等价触发。
        WM_HOTKEY | WM_KEY_TRIGGER => {
            match wparam.0 as i32 {
                HK_HIDE => state.apply_toggle(),
                HK_CLOSE => quit(state, hwnd),
                HK_HIDE_ONLY => state.apply_hide(),
                HK_SHOW_ONLY => state.apply_show(),
                HK_HIDE_FOREGROUND => state.apply_hide_foreground(),
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
                    show_tray_menu(
                        hwnd,
                        state.controller.is_hidden(),
                        state.config.setting.auto_hide_enabled,
                    );
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
                MENU_AUTO_HIDE => toggle_auto_hide(state, hwnd),
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
                // 托盘首挂失败后的兜底重试；挂载成功或超限后停表。
                TRAY_RETRY_TIMER_ID => {
                    let pending = state
                        .tray
                        .as_mut()
                        .map(TrayIcon::retry_pending)
                        .unwrap_or(false);
                    if !pending {
                        unsafe {
                            let _ = KillTimer(Some(hwnd), TRAY_RETRY_TIMER_ID);
                        }
                    }
                }
                AUTO_HIDE_TIMER_ID if state.config.setting.auto_hide_enabled => {
                    let idle_ms = idle::idle_millis().unwrap_or(0);
                    if idle::should_auto_hide(
                        idle_ms,
                        state.config.setting.auto_hide_time,
                        state.controller.is_hidden(),
                    ) {
                        state.apply_hide();
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

fn create_agent_window() -> windows::core::Result<HWND> {
    unsafe {
        let hinstance = GetModuleHandleW(PCWSTR::null())?;
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
    }
}

/// 加载配置；无论损坏还是读取失败都回退默认配置以保证核心可用，但必须留下记录。
///
/// 静默回退会让用户的热键与规则「凭空失效」，且日志里毫无痕迹，故此处按 error 记录，
/// 并写出配置路径与具体原因。
fn load_config_logging_fallback(path: &Path) -> Config {
    const FALLBACK: &str = "已按默认配置启动，原有热键与规则本次不生效";
    match Config::load_reporting(path) {
        Ok((config, None)) => config,
        Ok((config, Some(parse_error))) => {
            log_error!(
                "配置文件解析失败，{FALLBACK}: {} — {parse_error}",
                path.display()
            );
            config
        }
        Err(e) => {
            log_error!("配置文件读取失败，{FALLBACK}: {} — {e}", path.display());
            Config::default()
        }
    }
}

pub fn run(options: AgentOptions) {
    // 是否首次启动（尚无配置文件）/ 更新后首次启动（配置里记录的版本与当前不一致）。
    // load() 在文件缺失时也返回默认值，故须先按文件是否存在判断「首次」。
    let config_missing = !options.config_path.exists();
    let mut config = load_config_logging_fallback(&options.config_path);
    i18n::set_from_pref(&config.setting.language);
    let version_changed = !config_missing && config.version != bosskey_common::APP_CONFIG_VERSION;

    // 仅正常运行时（非冒烟测试）在这两种情况下默认拉起配置程序。
    if options.auto_quit_ms.is_none() && (config_missing || version_changed) {
        logging::info(if config_missing {
            "首次启动，拉起配置程序"
        } else {
            "更新后首次启动，拉起配置程序"
        });
        // 记录当前版本并落盘，避免下次启动重复弹出（首次启动时顺带创建配置文件）。
        config.version = bosskey_common::APP_CONFIG_VERSION.to_string();
        if let Err(e) = config.save(&options.config_path) {
            log_warn!("写入配置版本失败: {e}");
        }
        if let Some(path) = find_config_exe() {
            if let Err(e) = std::process::Command::new(&path).spawn() {
                log_warn!("启动配置程序失败: {e}");
            }
        } else {
            log_warn!("未找到配置程序，无法在启动时拉起");
        }
    }

    // 代理窗口是热键与 IPC 的唯一收口，创建失败即核心无法工作，须留下记录再退出。
    let hwnd = match create_agent_window() {
        Ok(hwnd) => hwnd,
        Err(e) => {
            log_error!("创建代理窗口失败，核心无法启动: {}", crate::util::win_err(&e));
            return;
        }
    };

    let tray = if options.enable_tray {
        unsafe {
            // 管理员进程默认收不到中等完整性 explorer 的 TaskbarCreated 广播（UIPI），
            // 须显式放行，否则计划任务开机启动时托盘图标永远补挂不上。
            let _ = ChangeWindowMessageFilterEx(
                hwnd,
                crate::tray::taskbar_created_msg(),
                MSGFLT_ALLOW,
                None,
            );
        }
        let tray = TrayIcon::new(hwnd, WM_APP_TRAY, "Boss Key");
        if !tray.is_visible() {
            // 首挂失败（任务栏尚未就绪）时定时重试兜底，TaskbarCreated 广播为主要恢复路径。
            unsafe {
                SetTimer(
                    Some(hwnd),
                    TRAY_RETRY_TIMER_ID,
                    TRAY_RETRY_INTERVAL_MS,
                    None,
                );
            }
        }
        Some(tray)
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

    let tray_icons = tray.as_ref().map(|_| TrayIconSet::new());

    // 副作用由专职线程执行，消息循环不被慢操作阻塞。
    let effects_worker = EffectsWorker::spawn(WinEffects::new(exe_dir));

    let mut state = Box::new(AgentState {
        config,
        config_path: options.config_path.clone(),
        recovery_path,
        controller: HideController::new(WindowsWindowManager, effects_worker.effects()),
        effects_worker: Some(effects_worker),
        persist_warned: false,
        tray,
        tray_icons,
        ipc_rx,
        mouse_hook: None,
        float_window: None,
        monitoring: true,
        hotkeys_armed: false,
        keyboard_hook: None,
    });

    // 恢复文件存在即上次异常退出仍有窗口被隐藏，先找回。
    if let Some(snapshot) = recovery::load(&state.recovery_path) {
        if snapshot.is_restorable(recovery::current_boot_time_ms()) {
            logging::warn(&format!(
                "检测到上次异常退出，开始找回 {} 个被隐藏的窗口（另需解冻 {} 个、取消静音 {} 个进程）",
                snapshot.hidden.len(),
                snapshot.frozen.len(),
                snapshot.muted.len()
            ));
            let outcome = state.controller.restore_from(snapshot);
            logging::info(&format!(
                "崩溃恢复完成：恢复 {} 个窗口，跳过 {} 条失效记录",
                outcome.shown, outcome.stale
            ));
        } else {
            // 跨重启（或旧版格式）快照中的句柄与 PID 已失效，只丢弃并留档。
            log_warn!(
                "恢复文件来自上一次开机或旧版本，其中的窗口句柄已失效，跳过恢复: {}",
                state.recovery_path.display()
            );
        }
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
                message: i18n::t(Msg::ErrCoreExited).to_string(),
            };
        }
        unsafe {
            let hwnd = HWND(hwnd_value as *mut std::ffi::c_void);
            if PostMessageW(Some(hwnd), WM_APP_IPC, WPARAM(0), LPARAM(0)).is_err() {
                return Response::Error {
                    message: i18n::t(Msg::ErrNotifyCore).to_string(),
                };
            }
        }
        reply_rx
            .recv_timeout(IPC_REPLY_TIMEOUT)
            .unwrap_or(Response::Error {
                message: i18n::t(Msg::ErrCoreTimeout).to_string(),
            })
    });

    if state.config.notifications.on_start
        && let Some(tray) = &state.tray
    {
        tray.balloon(i18n::t(Msg::StartTitle), i18n::t(Msg::StartBody));
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

    // 非 quit() 路径（如 WM_DESTROY）退出时兜底排干副作用队列。
    if let Some(worker) = state.effects_worker.take() {
        worker.shutdown(EFFECTS_SHUTDOWN_TIMEOUT);
    }

    drop(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_occupied_is_named_explicitly() {
        let e = windows::core::Error::from_hresult(ERROR_HOTKEY_ALREADY_REGISTERED.to_hresult());
        assert_eq!(
            hotkey_failure_message("隐藏", "Ctrl+Q", &e),
            "隐藏热键注册失败，已被其他程序占用，该热键不生效: Ctrl+Q"
        );
    }

    /// 验证「被占用」的判定与真实 API 一致：`hotkey_failure_message` 依赖
    /// RegisterHotKey 失败时确实报出 1409，此处用重复注册制造真实的占用冲突。
    #[test]
    fn duplicate_registration_really_yields_the_occupied_message() {
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("Static"),
                w!("BossKeyHotkeyTestWindow"),
                WS_OVERLAPPED,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                0,
                0,
                None,
                None,
                None,
                None,
            )
            .expect("创建测试窗口失败");

            // F24 + 三修饰键，避免与开发机上的真实热键冲突。
            let hk = ParsedHotkey {
                modifiers: crate::hotkey::MOD_CONTROL
                    | crate::hotkey::MOD_ALT
                    | crate::hotkey::MOD_SHIFT,
                vk: 0x87,
            };
            register(hwnd, HK_HIDE, &hk).expect("首次注册应成功");
            let e = register(hwnd, HK_CLOSE, &hk).expect_err("重复注册同一组合应失败");

            assert!(
                hotkey_failure_message("隐藏", "Ctrl+Alt+Shift+F24", &e)
                    .contains("已被其他程序占用"),
                "真实的占用冲突应被识别为占用，而非退化成裸错误码: {}",
                crate::util::win_err(&e)
            );

            let _ = UnregisterHotKey(Some(hwnd), HK_HIDE);
            let _ = DestroyWindow(hwnd);
        }
    }

    #[test]
    fn other_hotkey_failures_report_the_error_code_instead_of_guessing() {
        // 非 1409 的失败不应谎称「已被占用」，而应带出真实错误码。
        let e = windows::core::Error::from_hresult(windows::core::HRESULT(0x8007_0005u32 as i32));
        let text = hotkey_failure_message("关闭", "Win+Esc", &e);
        assert!(!text.contains("已被其他程序占用"), "不应猜测原因: {text}");
        assert!(text.contains("Win+Esc"), "应带出热键: {text}");
        assert!(text.contains("0x80070005"), "应带出系统错误码: {text}");
    }
}
