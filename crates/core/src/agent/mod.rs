//! 核心的代理窗口：热键、鼠标、IPC 与托盘的唯一收口。
//!
//! 本模块持有运行时状态 [`AgentState`] 与消息循环；IPC 命令的执行、托盘菜单、
//! 日志文案格式化各自分在下面的子模块里。

mod commands;
mod log_fmt;
mod menu;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use log_fmt::{hotkey_failure_message, log_hide, log_show};
use menu::{show_tray_menu, toggle_auto_hide, toggle_autostart};

pub use commands::forward_open_settings;
use commands::{find_config_exe, launch_settings};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, EVENT_OBJECT_DESTROY, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW,
    GWLP_USERDATA, GetMessageW, GetWindowLongPtrW, KillTimer, MSG, MSGFLT_ALLOW, PostMessageW,
    PostQuitMessage, RegisterClassW, SetTimer, SetWindowLongPtrW, TranslateMessage,
    WINDOW_EX_STYLE, WM_APP, WM_COMMAND, WM_DESTROY, WM_HOTKEY, WM_LBUTTONUP, WM_RBUTTONUP,
    WM_TIMER, WNDCLASSW, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};
use zonedeck_common::ipc::{Command, Response};
use zonedeck_common::{APP_NAME, ARG_ABOUT, ARG_RESTORE, Config, IgnoreMode, Setting, WindowInfo};

use crate::effects::WinEffects;
use crate::effects_worker::{AsyncEffects, EffectsWorker};
use crate::float_window::{FLOAT_MENU, FLOAT_TOGGLE, FloatWindow, WM_APP_FLOAT};
use crate::hide::{
    HideController, HidePlan, Target, dormant_pids, expand_descendants, expand_same_image,
    filter_freeze_whitelist, foreground_target, resolve_targets,
};
use crate::hotkey::{MOD_NOREPEAT, ParsedHotkey, is_disabled, parse_hotkey};
use crate::i18n::{self, Msg};
use crate::input_hooks::InputHooks;
use crate::keyboard_hook::{self, WM_KEY_TRIGGER};
use crate::mouse_hook::{self, TRIGGER_CORNER, WM_MOUSE_TRIGGER};
use crate::platform::win32::WindowsWindowManager;
use crate::tray::TrayIcon;
use crate::tray_badge::TrayIconSet;
use crate::win_event::{WM_APP_WINEVENT, WinEventHook};
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
/// 托盘图标挂载重试间隔。
const TRAY_RETRY_TIMER_ID: usize = 13;
const AUTO_HIDE_INTERVAL_MS: u32 = 5000;
const TRAY_RETRY_INTERVAL_MS: u32 = 2000;
const IPC_REPLY_TIMEOUT: Duration = Duration::from_secs(3);
/// 退出时等待副作用线程排干队列（解冻 / 取消静音）的上限。
const EFFECTS_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// 一次隐藏 / 恢复的触发来源，仅用于日志。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trigger {
    Hotkey,
    MouseButton,
    Corner,
    Idle,
    TrayClick,
    TrayMenu,
    FloatWindow,
    /// 配置程序经 IPC 下发。
    Ipc,
    /// 核心退出前的收尾恢复。
    Quit,
}

impl std::fmt::Display for Trigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Trigger::Hotkey => "热键",
            Trigger::MouseButton => "鼠标按键",
            Trigger::Corner => "移动到屏幕四角",
            Trigger::Idle => "空闲自动隐藏",
            Trigger::TrayClick => "单击托盘图标",
            Trigger::TrayMenu => "托盘菜单",
            Trigger::FloatWindow => "悬浮窗",
            Trigger::Ipc => "配置程序",
            Trigger::Quit => "核心退出前",
        };
        f.write_str(name)
    }
}

/// 调用方预先加载好的启动配置；加载说明待日志就绪后由 [`run`] 记录。
pub struct StartupConfig {
    pub config: Config,
    /// [`Config::load_reporting`] 的说明；日志就绪后才记录。
    pub note: Option<zonedeck_common::LoadNote>,
    /// 读取失败（不是解析失败）的原因；此时 `config` 是默认值。
    pub read_error: Option<String>,
    /// 配置文件本来就不存在（首次启动）。
    pub missing: bool,
}

impl StartupConfig {
    pub fn load(path: &Path) -> Self {
        // load_reporting 在文件缺失时也返回默认值，故须先按文件是否存在判断。
        let missing = !path.exists();
        match Config::load_reporting(path) {
            Ok((config, note)) => Self {
                config,
                note,
                read_error: None,
                missing,
            },
            Err(e) => Self {
                config: Config::default(),
                note: None,
                read_error: Some(e.to_string()),
                missing,
            },
        }
    }
}

pub struct AgentOptions {
    pub config_path: PathBuf,
    pub pipe_name: String,
    pub enable_tray: bool,
    pub auto_quit_ms: Option<u32>,
    /// 已加载好的配置；`None` 时由 [`run`] 自行加载。见 [`StartupConfig`]。
    pub preloaded: Option<StartupConfig>,
}

impl AgentOptions {
    pub fn standard(config_path: PathBuf) -> Self {
        Self {
            config_path,
            pipe_name: zonedeck_common::ipc::PIPE_NAME.to_string(),
            enable_tray: true,
            auto_quit_ms: None,
            preloaded: None,
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
    /// 窗口事件钩子（销毁 / 显示 / 改标题）。
    win_event_hook: Option<WinEventHook>,
    tray: Option<TrayIcon>,
    /// 托盘图标的角标变体缓存；未启用托盘时为 None。
    tray_icons: Option<TrayIconSet>,
    ipc_rx: Receiver<(Command, Sender<Response>)>,
    /// 承载两个低级输入钩子的专职线程；起不来时为 None。
    input_hooks: Option<InputHooks>,
    float_window: Option<FloatWindow>,
    /// 是否正在监听热键与鼠标（见 `Command::SetHotkeys`）。
    monitoring: bool,
    /// 热键当前是否已注册（避免重复 RegisterHotKey）。
    hotkeys_armed: bool,
}

/// 一条待装进键盘钩子的热键。
struct HookedHotkey {
    id: i32,
    /// 日志里的中文名。
    label: &'static str,
    /// 配置里的原始组合字符串，只用于日志。
    raw: String,
    hotkey: ParsedHotkey,
    /// 「不传递」开关。
    swallow: bool,
}

impl AgentState {
    fn register_hotkeys(&mut self, hwnd: HWND) {
        let mut hooked: Vec<HookedHotkey> = Vec::new();
        for (id, label, raw, want_hook, swallow) in [
            (
                HK_HIDE,
                "隐藏",
                &self.config.hotkey.hide_hotkey,
                self.config.hotkey.hide_hook,
                self.config.hotkey.hide_intercept,
            ),
            (
                HK_CLOSE,
                "关闭",
                &self.config.hotkey.close_hotkey,
                self.config.hotkey.close_hook,
                self.config.hotkey.close_intercept,
            ),
            (
                HK_HIDE_ONLY,
                "仅隐藏",
                &self.config.hotkey.hide_only_hotkey,
                self.config.hotkey.hide_only_hook,
                self.config.hotkey.hide_only_intercept,
            ),
            (
                HK_SHOW_ONLY,
                "仅显示",
                &self.config.hotkey.show_only_hotkey,
                self.config.hotkey.show_only_hook,
                self.config.hotkey.show_only_intercept,
            ),
            (
                HK_HIDE_FOREGROUND,
                "隐藏前台窗口",
                &self.config.hotkey.hide_foreground_hotkey,
                self.config.hotkey.hide_foreground_hook,
                self.config.hotkey.hide_foreground_intercept,
            ),
        ] {
            if is_disabled(raw) {
                logging::debug(&format!("{label}热键已置空，不注册"));
                continue;
            }
            match parse_hotkey(raw) {
                // 纯修饰键与多主键组合 RegisterHotKey 表达不了，只能走钩子。
                Ok(hk) if want_hook || hk.requires_hook() => hooked.push(HookedHotkey {
                    id,
                    label,
                    raw: raw.clone(),
                    hotkey: hk,
                    swallow,
                }),
                Ok(hk) => {
                    let Some(vk) = hk.single() else { continue };
                    unsafe {
                        match register(hwnd, id, hk.modifiers, vk) {
                            Ok(()) => logging::debug(&format!("{label}热键已注册: {raw}")),
                            Err(e) => log_warn!("{}", hotkey_failure_message(label, raw, &e)),
                        }
                    }
                }
                Err(e) => log_warn!("{label}热键解析失败，该热键不生效: {raw} — {e}"),
            }
        }
        self.arm_hook_hotkeys(hwnd, hooked);
    }

    /// 把走钩子的热键装载进键盘钩子；安装失败时能回退的回退 `RegisterHotKey`。
    fn arm_hook_hotkeys(&mut self, hwnd: HWND, hooked: Vec<HookedHotkey>) {
        if hooked.is_empty() {
            keyboard_hook::set_hotkeys(&[]);
            self.set_keyboard_hook(false);
            return;
        }
        let specs: Vec<(i32, ParsedHotkey, bool)> = hooked
            .iter()
            .map(|h| (h.id, h.hotkey.clone(), h.swallow))
            .collect();
        keyboard_hook::set_hotkeys(&specs);
        if self.set_keyboard_hook(true) {
            for h in &hooked {
                let pass = if h.swallow {
                    "不传递"
                } else {
                    "照常传递"
                };
                logging::debug(&format!(
                    "{}热键已由键盘钩子承载（{pass}）: {}",
                    h.label, h.raw
                ));
            }
            return;
        }
        keyboard_hook::set_hotkeys(&[]);
        log_warn!("键盘钩子安装失败，「不传递」不生效");
        for h in &hooked {
            let Some(vk) = h.hotkey.single() else {
                log_warn!(
                    "{}热键的组合只有键盘钩子承载得了，钩子装不上，该热键本次不生效: {}",
                    h.label,
                    h.raw
                );
                continue;
            };
            unsafe {
                match register(hwnd, h.id, h.hotkey.modifiers, vk) {
                    Ok(()) => {
                        logging::debug(&format!("{}热键已回退为普通注册: {}", h.label, h.raw))
                    }
                    Err(e) => log_warn!("{}", hotkey_failure_message(h.label, &h.raw, &e)),
                }
            }
        }
    }

    /// 请求钩子线程把键盘钩子对齐 `on`，返回对齐后是否已装上。
    fn set_keyboard_hook(&self, on: bool) -> bool {
        self.input_hooks
            .as_ref()
            .is_some_and(|hooks| hooks.set_keyboard(on))
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
        self.set_keyboard_hook(false);
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
        let want_mouse = self.monitoring && mouse_hook::wants_hook(&self.config.setting);
        if want_mouse {
            mouse_hook::set_flags(&self.config.setting);
        }
        let mouse_armed = self
            .input_hooks
            .as_ref()
            .is_some_and(|hooks| hooks.set_mouse(want_mouse));
        if want_mouse && !mouse_armed {
            log_warn!("鼠标钩子安装失败，鼠标按键绑定与四角触发不生效");
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
                if self.float_window.is_none() {
                    log_warn!("悬浮窗创建失败，本次运行不显示悬浮窗，其余功能不受影响");
                }
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

    /// 让托盘图标的状态角标与悬浮提示对齐当前配置与运行状态。
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

    /// 把指定快照落盘；失败不阻断本次隐藏，每次运行提醒用户一次。
    fn persist_snapshot(&mut self, snapshot: &recovery::Snapshot) {
        if let Err(e) = recovery::save(&self.recovery_path, snapshot) {
            log_error!(
                "写入崩溃恢复文件失败，核心若异常退出将无法自动找回窗口: {} — {e}",
                self.recovery_path.display()
            );
            if !self.persist_warned {
                self.persist_warned = true;
                self.balloon(
                    Msg::RecoveryPersistFailedTitle,
                    Msg::RecoveryPersistFailedBody,
                );
            }
        }
    }

    /// 按控制器当前状态落盘。
    fn persist_recovery(&mut self) {
        let snapshot = self.controller.snapshot();
        self.persist_snapshot(&snapshot);
    }

    /// 意图先行：先落盘计划后的快照，再隐藏窗口。
    /// `dormant` 为 [`dormant_pids`] 的结果，静音直接用它，
    /// 冻结与效率模式各用它按自己的作用范围展开后的结果。
    fn hide_with_plan(
        &mut self,
        targets: &[crate::hide::Target],
        freeze_set: &[u32],
        efficiency_set: &[u32],
        dormant: &[u32],
    ) -> HidePlan {
        let setting = self.config.setting.clone();
        let whitelist = self.config.whitelist().to_vec();
        let plan = self.hide_with_plan_using(
            &setting,
            targets,
            freeze_set,
            efficiency_set,
            dormant,
            &whitelist,
        );
        if self.config.notifications.on_hide && !plan.fresh.is_empty() {
            self.balloon(Msg::HiddenTitle, Msg::HiddenBody);
        }
        plan
    }

    /// [`Self::hide_with_plan`] 的按指定设置版本，供恢复工具的手动隐藏使用。
    fn hide_with_plan_using(
        &mut self,
        setting: &Setting,
        targets: &[crate::hide::Target],
        freeze_set: &[u32],
        efficiency_set: &[u32],
        mute_set: &[u32],
        whitelist: &[zonedeck_common::WhitelistRule],
    ) -> HidePlan {
        let plan = self.controller.plan_hide(
            setting,
            targets,
            freeze_set,
            efficiency_set,
            mute_set,
            whitelist,
        );
        let planned = self.controller.planned_snapshot(&plan);
        self.persist_snapshot(&planned);
        self.controller.commit_hide(plan.clone());
        self.sync_tray();
        plan
    }

    /// 按作用范围展开候选 PID，再过白名单，得到要施加能效控制的集合。
    /// 顺序不可颠倒：ZoneDeck 自己往往是展开后才出现在集合里的。
    ///
    /// `scope` 由调用方给出，冻结与效率模式各有各的范围设置。白名单共用
    /// [`IgnoreMode::Freeze`]：两者都是「别动这个进程的性能」。
    /// `snapshot` 由调用方取一次传进来。
    fn scoped_pids(
        &self,
        roots: Vec<u32>,
        scope: &str,
        snapshot: &crate::freeze::ProcessSnapshot,
    ) -> Vec<u32> {
        let names = &snapshot.names;
        let pids = match scope {
            zonedeck_common::POWER_SCOPE_TREE => expand_descendants(&roots, &snapshot.edges),
            zonedeck_common::POWER_SCOPE_IMAGE => expand_same_image(&roots, names),
            _ => roots,
        };

        let whitelist = self.config.whitelist();
        // 完整路径要逐 PID OpenProcess，没有按路径的条目就不查。
        let paths = if zonedeck_common::whitelist_needs_paths(whitelist, IgnoreMode::Freeze) {
            pids.iter()
                .map(|&pid| (pid, crate::platform::win32::process_path(pid)))
                .collect()
        } else {
            std::collections::HashMap::new()
        };

        let mut pids = filter_freeze_whitelist(pids, names, &paths, whitelist);
        // 无论映像名叫什么，都不能把自己冻死。
        let self_pid = std::process::id();
        pids.retain(|pid| *pid != self_pid);
        pids
    }

    /// 冻结与效率模式各自的目标集合；两者范围设置独立，共用一次进程快照。
    fn power_targets(&self, dormant: &[u32]) -> (Vec<u32>, Vec<u32>) {
        let setting = &self.config.setting;
        if !setting.freeze_after_hide && !setting.efficiency_after_hide {
            return (Vec::new(), Vec::new());
        }
        let snapshot = crate::freeze::process_snapshot();
        let freeze = if setting.freeze_after_hide {
            self.scoped_pids(dormant.to_vec(), &setting.power_scope, &snapshot)
        } else {
            Vec::new()
        };
        let efficiency = if setting.efficiency_after_hide {
            self.scoped_pids(dormant.to_vec(), &setting.efficiency_scope, &snapshot)
        } else {
            Vec::new()
        };
        (freeze, efficiency)
    }

    /// 按规则隐藏：枚举 → 解析规则 → 展开能效目标 → 执行并记日志。
    fn apply_hide(&mut self, trigger: Trigger) {
        let windows = self.controller.enumerate();
        let foreground = self.controller.foreground();
        let (targets, outcomes) = resolve_targets(&mut self.config, &windows, foreground);
        let plan = self.hide_targets(&targets, &windows);
        log_hide(trigger, &self.config, &outcomes, &plan);
    }

    /// 只隐藏当前前台窗口，可连续触发逐个隐藏。
    fn apply_hide_foreground(&mut self, trigger: Trigger) {
        let windows = self.controller.enumerate();
        let foreground = self.controller.foreground();
        let Some(target) = foreground_target(&windows, foreground) else {
            logging::debug(&format!(
                "{trigger}触发隐藏前台窗口：当前没有可隐藏的前台窗口"
            ));
            return;
        };

        let plan = self.hide_targets(&[target], &windows);
        if let Some(t) = plan.fresh.first() {
            logging::debug(&format!("{trigger}触发隐藏前台窗口: {}", t.describe()));
        }
    }

    /// 隐藏一组已选定的目标：展开冻结 / 效率模式的作用范围，再交给控制器执行。
    fn hide_targets(&mut self, targets: &[Target], windows: &[WindowInfo]) -> HidePlan {
        let dormant = dormant_pids(targets, windows);
        let (freeze_set, efficiency_set) = self.power_targets(&dormant);
        self.hide_with_plan(targets, &freeze_set, &efficiency_set, &dormant)
    }

    /// 处理窗口事件：销毁 / 被外部恢复显示的窗口移出隐藏记录，标题变化同步进
    /// 隐藏记录与规则（仅内存）。
    fn on_win_event(&mut self, event: u32, hwnd: i64) {
        match event {
            EVENT_OBJECT_DESTROY | EVENT_OBJECT_SHOW => {
                if self.controller.forget_window(hwnd) {
                    self.persist_recovery();
                    self.sync_tray();
                }
            }
            EVENT_OBJECT_NAMECHANGE => {
                let tracked_rule = self
                    .config
                    .window_rules
                    .iter()
                    .any(|r| r.regex.is_none() && r.hwnd == hwnd);
                if !tracked_rule && !self.controller.tracks_window(hwnd) {
                    return;
                }
                let title = self.controller.window_title(hwnd);
                self.controller.update_title(hwnd, &title);
                crate::hide::sync_rule_titles(&mut self.config.window_rules, hwnd, &title);
            }
            _ => {}
        }
    }

    fn apply_show(&mut self, trigger: Trigger) {
        let outcome = self.controller.show();
        log_show(trigger, outcome);
        self.persist_recovery();
        self.sync_tray();
        if self.config.notifications.on_show {
            self.balloon(Msg::ShownTitle, Msg::ShownBody);
        }
    }

    /// 发送托盘气泡（托盘不存在或已隐藏时静默忽略）；标题与正文须成对传入。
    fn balloon(&self, title: Msg, body: Msg) {
        if let Some(tray) = &self.tray {
            tray.balloon(i18n::t(title), i18n::t(body));
        }
    }

    fn apply_toggle(&mut self, trigger: Trigger) {
        if self.controller.is_hidden() {
            self.apply_show(trigger);
        } else {
            self.apply_hide(trigger);
        }
    }
}

/// 注册一个系统热键。`MOD_NOREPEAT` 让长按只触发一次。
unsafe fn register(hwnd: HWND, id: i32, modifiers: u32, vk: u16) -> windows::core::Result<()> {
    unsafe {
        RegisterHotKey(
            Some(hwnd),
            id,
            HOT_KEY_MODIFIERS(modifiers | MOD_NOREPEAT),
            vk as u32,
        )
    }
}

/// 代理窗口 `GWLP_USERDATA` 里存放的状态单元。菜单模态循环会重入 `wndproc`，
/// 用 `RefCell` 让重入时借用失败、事件被安全丢弃。
fn state_cell<'a>(hwnd: HWND) -> Option<&'a RefCell<AgentState>> {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<AgentState>;
        ptr.as_ref()
    }
}
/// 退出核心。`reason` 写进会话结束标记。
fn quit(state: &mut AgentState, hwnd: HWND, reason: &str) {
    let outcome = state.controller.show();
    if outcome.shown > 0 || outcome.stale > 0 {
        log_show(Trigger::Quit, outcome);
    }
    state.persist_recovery();
    state.unregister_hotkeys(hwnd);
    // 先摘钩子，退出过程中不再有输入事件进来。
    state.input_hooks = None;
    // 确保退出前解冻 / 取消静音已生效。
    if let Some(worker) = state.effects_worker.take() {
        worker.shutdown(EFFECTS_SHUTDOWN_TIMEOUT);
    }
    logging::session_exit(&format!("核心正常退出（{reason}）"));
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
    let Some(cell) = state_cell(hwnd) else {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    };
    // 借用失败即模态菜单重入，丢弃本次事件。
    let Ok(mut state) = cell.try_borrow_mut() else {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    };
    let state = &mut *state;
    if msg != 0 && msg == crate::tray::taskbar_created_msg() {
        if let Some(tray) = &mut state.tray {
            tray.on_taskbar_created();
        }
        return LRESULT(0);
    }

    match msg {
        WM_HOTKEY | WM_KEY_TRIGGER => {
            match wparam.0 as i32 {
                HK_HIDE => state.apply_toggle(Trigger::Hotkey),
                HK_CLOSE => quit(state, hwnd, "关闭热键"),
                HK_HIDE_ONLY => state.apply_hide(Trigger::Hotkey),
                HK_SHOW_ONLY => state.apply_show(Trigger::Hotkey),
                HK_HIDE_FOREGROUND => state.apply_hide_foreground(Trigger::Hotkey),
                _ => {}
            }
            LRESULT(0)
        }
        WM_APP_WINEVENT => {
            state.on_win_event(wparam.0 as u32, lparam.0 as i64);
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
                quit(state, hwnd, "配置程序发来退出命令");
            }
            LRESULT(0)
        }
        WM_APP_TRAY => {
            match lparam.0 as u32 {
                WM_LBUTTONUP => {
                    if state.config.setting.click_to_hide {
                        state.apply_toggle(Trigger::TrayClick);
                    }
                }
                WM_RBUTTONUP => {
                    show_tray_menu(
                        hwnd,
                        state.controller.is_hidden(),
                        state.config.setting.auto_hide_enabled,
                    );
                    // 排干菜单模态期间积压的 IPC 命令。
                    unsafe {
                        let _ = PostMessageW(Some(hwnd), WM_APP_IPC, WPARAM(0), LPARAM(0));
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_APP_FLOAT => {
            match wparam.0 {
                // 双击悬浮窗触发。
                FLOAT_TOGGLE => {
                    if state.config.setting.click_to_hide {
                        state.apply_toggle(Trigger::FloatWindow);
                    }
                }
                FLOAT_MENU => {
                    crate::float_window::show_float_menu(hwnd, MENU_SETTINGS, MENU_QUIT);
                    unsafe {
                        let _ = PostMessageW(Some(hwnd), WM_APP_IPC, WPARAM(0), LPARAM(0));
                    }
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
                MENU_TOGGLE => state.apply_toggle(Trigger::TrayMenu),
                MENU_AUTO_HIDE => toggle_auto_hide(state, hwnd),
                MENU_AUTOSTART => toggle_autostart(state),
                MENU_QUIT => quit(state, hwnd, "托盘菜单"),
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
            let trigger = if wparam.0 == TRIGGER_CORNER {
                Trigger::Corner
            } else {
                Trigger::MouseButton
            };
            if state.controller.is_hidden() {
                if allow_restore {
                    state.apply_show(trigger);
                }
            } else {
                state.apply_hide(trigger);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            match wparam.0 {
                AUTO_QUIT_TIMER_ID => {
                    unsafe {
                        let _ = KillTimer(Some(hwnd), AUTO_QUIT_TIMER_ID);
                    }
                    quit(state, hwnd, "冒烟测试计时到点");
                }
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
                // 托盘首挂失败后重试，挂载成功或超限后停表。
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
                        state.controller.tracks_any(),
                    ) {
                        state.apply_hide(Trigger::Idle);
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
        let class_name = w!("ZoneDeckAgentWindow");
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
            w!("ZoneDeck"),
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

/// 记录 [`Config::load_reporting`] 的说明。解析失败是错误（本次运行的规则全部不生效），
/// schema 偏高只是警告（配置照常生效，只是新版设置项会在下次保存时丢失）。
fn log_load_note(path: &Path, note: Option<&zonedeck_common::LoadNote>, when: &str) {
    match note {
        None => {}
        Some(zonedeck_common::LoadNote::Corrupt(reason)) => log_error!(
            "{when}时配置解析失败，已按默认配置生效，原有热键与规则不生效: {} — {reason}",
            path.display()
        ),
        Some(zonedeck_common::LoadNote::NewerSchema(reason)) => {
            log_warn!("{when}配置: {} — {reason}", path.display())
        }
    }
}

/// 把启动加载的说明写进日志。须在日志系统就绪之后调用。
fn log_startup_load(path: &Path, loaded: &StartupConfig) {
    if let Some(e) = &loaded.read_error {
        log_error!(
            "配置文件读取失败，已按默认配置启动，原有热键与规则本次不生效: {} — {e}",
            path.display()
        );
    }
    log_load_note(path, loaded.note.as_ref(), "启动");
}

/// 启动时是否该拉起配置程序：首次启动，或程序版本与上次运行的不一致。
/// `recorded` 为空一律当作版本已变。
fn should_open_settings(config_missing: bool, recorded: &str, current: &str) -> bool {
    config_missing || recorded != current
}

pub fn run(mut options: AgentOptions) {
    let loaded = options
        .preloaded
        .take()
        .unwrap_or_else(|| StartupConfig::load(&options.config_path));
    log_startup_load(&options.config_path, &loaded);
    let config_missing = loaded.missing;
    let mut config = loaded.config;
    i18n::set_from_pref(&config.setting.language);

    // 品牌改名迁移：把旧名称的自启注册迁到新名称。冒烟测试不碰系统级注册。
    if options.auto_quit_ms.is_none() {
        use crate::autostart::LegacyMigration;
        match crate::autostart::migrate_legacy(config.setting.autostart_admin) {
            LegacyMigration::NotNeeded => {}
            LegacyMigration::Done => {
                log_warn!("发现旧品牌（Boss Key）的自启注册，已迁移到新名称");
            }
            LegacyMigration::Failed(e) => {
                log_warn!("旧品牌自启迁移失败，保留旧注册待下次启动重试: {e}");
            }
        }
    }

    let open_settings = should_open_settings(
        config_missing,
        &config.app_version,
        zonedeck_common::APP_VERSION,
    );

    // 冒烟测试不拉起配置程序。
    if options.auto_quit_ms.is_none() && open_settings {
        let reason = if config_missing {
            "首次启动，拉起配置程序".to_string()
        } else {
            let was = if config.app_version.is_empty() {
                "未记录"
            } else {
                &config.app_version
            };
            format!(
                "更新后首次启动（{was} → {}），拉起配置程序",
                zonedeck_common::APP_VERSION
            )
        };
        logging::info(&reason);
        // 记录当前版本并落盘，避免下次启动重复弹出。
        config.app_version = zonedeck_common::APP_VERSION.to_string();
        if let Err(e) = config.save(&options.config_path) {
            log_warn!(
                "写入程序版本到配置失败，下次启动会重复拉起配置程序: {} — {e}",
                options.config_path.display()
            );
        }
        if let Some(path) = find_config_exe() {
            if let Err(e) = std::process::Command::new(&path).spawn() {
                log_warn!(
                    "启动时拉起配置程序失败（可能被安全软件拦截）: {} — {e}",
                    path.display()
                );
            }
        } else {
            log_warn!(
                "核心所在目录下找不到配置程序（config.exe / zonedeck-config.exe），启动时无法拉起设置界面"
            );
        }
    }

    // 代理窗口是热键与 IPC 的唯一收口，创建失败即核心无法工作。
    let hwnd = match create_agent_window() {
        Ok(hwnd) => hwnd,
        Err(e) => {
            log_error!(
                "创建代理窗口失败，核心无法启动: {}",
                crate::util::win_err(&e)
            );
            return;
        }
    };

    let tray = if options.enable_tray {
        unsafe {
            // 管理员进程默认收不到中等完整性 explorer 的 TaskbarCreated 广播，须显式放行。
            let _ = ChangeWindowMessageFilterEx(
                hwnd,
                crate::tray::taskbar_created_msg(),
                MSGFLT_ALLOW,
                None,
            );
        }
        let tray = TrayIcon::new(hwnd, WM_APP_TRAY, "ZoneDeck");
        if !tray.is_visible() {
            // 首挂失败时定时重试，TaskbarCreated 广播为主要恢复路径。
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

    let effects_worker = EffectsWorker::spawn(WinEffects::new(exe_dir));

    // 低级输入钩子挂在专职线程上，代理线程的重活不得拖慢全局输入。
    let input_hooks = InputHooks::spawn(hwnd);
    if input_hooks.is_none() {
        log_error!(
            "输入钩子线程未能就绪（失败原因见前一条记录），本次运行鼠标按键触发、四角触发与「不传递」热键均不可用"
        );
    }

    let state = Box::new(RefCell::new(AgentState {
        config,
        config_path: options.config_path.clone(),
        recovery_path,
        controller: HideController::new(WindowsWindowManager, effects_worker.effects()),
        effects_worker: Some(effects_worker),
        persist_warned: false,
        win_event_hook: None,
        tray,
        tray_icons,
        ipc_rx,
        input_hooks,
        float_window: None,
        monitoring: true,
        hotkeys_armed: false,
    }));

    // 恢复文件存在即上次异常退出仍有窗口被隐藏。
    {
        let mut state = state.borrow_mut();
        if let Some(snapshot) = recovery::load(&state.recovery_path) {
            if snapshot.is_restorable(recovery::current_boot_time_ms()) {
                let (hidden, frozen, muted) = (
                    snapshot.hidden.len(),
                    snapshot.frozen.len(),
                    snapshot.muted.len(),
                );
                let outcome = state.controller.restore_from(snapshot);
                logging::warn(&format!(
                    "检测到上次异常退出：{hidden} 个被隐藏的窗口中恢复 {} 个、跳过 {} 条失效记录（另解冻 {frozen} 个、取消静音 {muted} 个进程）",
                    outcome.shown, outcome.stale
                ));
            } else {
                // 跨重启或旧版格式的快照中句柄与 PID 已失效，丢弃不恢复。
                logging::debug("恢复文件来自上一次开机或旧版本，其中的窗口句柄已失效，跳过恢复");
            }
            recovery::clear(&state.recovery_path);
        }
    }

    unsafe {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            &*state as *const RefCell<AgentState> as isize,
        );
    }

    {
        let mut state = state.borrow_mut();
        state.win_event_hook = WinEventHook::install(hwnd);
        if state.win_event_hook.is_none() {
            log_warn!("窗口事件钩子安装失败，隐藏记录仅在触发隐藏 / 恢复时维护");
        }
        state.sync_monitoring(hwnd);
    }

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

    {
        let state = state.borrow();
        if state.config.notifications.on_start
            && let Some(tray) = &state.tray
        {
            tray.balloon(i18n::t(Msg::StartTitle), i18n::t(Msg::StartBody));
        }
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

    // 非 quit() 路径退出时兜底排干副作用队列。
    if let Some(worker) = state.borrow_mut().effects_worker.take() {
        worker.shutdown(EFFECTS_SHUTDOWN_TIMEOUT);
    }

    drop(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_open_on_first_run_and_after_every_version_change() {
        assert!(
            should_open_settings(true, "", "3.1.0"),
            "首次启动（无配置文件）须拉起配置程序"
        );
        assert!(
            should_open_settings(false, "3.0.0", "3.1.0"),
            "程序版本变了须拉起配置程序"
        );
        assert!(
            should_open_settings(false, "", "3.1.0"),
            "更早的版本没记过程序版本，视为版本已变"
        );
        assert!(
            !should_open_settings(false, "3.1.0", "3.1.0"),
            "版本没变就别每次启动都弹窗"
        );
        assert!(
            should_open_settings(false, "3.1.0", "3.1.0-rc.2"),
            "回退到预发布版也是版本变动"
        );
    }

    #[test]
    fn hotkey_occupied_is_named_explicitly() {
        let e = windows::core::Error::from_hresult(
            windows::Win32::Foundation::ERROR_HOTKEY_ALREADY_REGISTERED.to_hresult(),
        );
        assert_eq!(
            hotkey_failure_message("隐藏", "Ctrl+Q", &e),
            "隐藏热键注册失败，已被其他程序占用，该热键不生效: Ctrl+Q"
        );
    }

    /// 用重复注册制造真实的占用冲突，验证 RegisterHotKey 确实报出 1409。
    #[test]
    fn duplicate_registration_really_yields_the_occupied_message() {
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("Static"),
                w!("ZoneDeckHotkeyTestWindow"),
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

            // F24 + 三修饰键，避免与真实热键冲突。
            let modifiers =
                crate::hotkey::MOD_CONTROL | crate::hotkey::MOD_ALT | crate::hotkey::MOD_SHIFT;
            register(hwnd, HK_HIDE, modifiers, 0x87).expect("首次注册应成功");
            let e = register(hwnd, HK_CLOSE, modifiers, 0x87).expect_err("重复注册同一组合应失败");

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
        // 非 1409 的失败应带出真实错误码。
        let e = windows::core::Error::from_hresult(windows::core::HRESULT(0x8007_0005u32 as i32));
        let text = hotkey_failure_message("关闭", "Win+Esc", &e);
        assert!(!text.contains("已被其他程序占用"), "不应猜测原因: {text}");
        assert!(text.contains("Win+Esc"), "应带出热键: {text}");
        assert!(text.contains("0x80070005"), "应带出系统错误码: {text}");
    }
}
