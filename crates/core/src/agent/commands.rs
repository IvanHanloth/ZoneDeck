//! IPC 命令的执行，以及与配置程序、开机自启相关的进程操作。

use std::path::{Path, PathBuf};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};
use zonedeck_common::Config;
use zonedeck_common::ipc::{Command, PipeClient, Response};

use crate::i18n::{self, Msg};
use crate::{log_error, log_warn, logging};

use super::{AgentState, SUSPEND_GUARD_TIMER_ID, Trigger, log_load_note};

impl AgentState {
    pub(super) fn execute(&mut self, hwnd: HWND, cmd: Command) -> (Response, bool) {
        match cmd {
            Command::ReloadConfig => match Config::load_reporting(&self.config_path) {
                Ok((config, note)) => {
                    log_load_note(&self.config_path, note.as_ref(), "重载");
                    if self.hotkeys_armed {
                        self.unregister_hotkeys(hwnd);
                        self.hotkeys_armed = false;
                    }
                    self.config = config;
                    i18n::set_from_pref(&self.config.setting.language);
                    logging::set_level(logging::Level::from_config(&self.config.setting.log_level));
                    self.sync_monitoring(hwnd);
                    logging::debug("配置已重新加载，热键、鼠标监控与日志等级均已对齐");
                    (Response::Ok, false)
                }
                Err(e) => {
                    log_error!(
                        "重新加载配置失败，本次改动未生效，核心仍在用上一次加载的配置: {} — {e}",
                        self.config_path.display()
                    );
                    (
                        Response::Error {
                            message: i18n::tf(Msg::ErrReloadConfig, &[("err", &e.to_string())]),
                        },
                        false,
                    )
                }
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
                self.apply_hide(Trigger::Ipc);
                (Response::Ok, false)
            }
            Command::Show => {
                self.apply_show(Trigger::Ipc);
                (Response::Ok, false)
            }
            Command::Toggle => {
                self.apply_toggle(Trigger::Ipc);
                (Response::Ok, false)
            }
            Command::SetAutostart { enabled, admin } => (set_autostart(enabled, admin), false),
            // 停用有状态，期间的 ReloadConfig 不会复活它；由看门狗定时器超时恢复。
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
                            zonedeck_common::ipc::SUSPEND_TIMEOUT_MS,
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
            Command::ReleaseWindows { hwnds } => {
                let released = self.controller.release_windows(&hwnds);
                if released > 0 {
                    logging::debug(&format!("窗口恢复工具释放 {released} 个窗口"));
                    self.persist_recovery();
                    self.sync_tray();
                }
                (Response::Ok, false)
            }
            Command::AdoptWindows { hwnds } => {
                let targets: Vec<crate::hide::Target> = hwnds
                    .iter()
                    .map(|&h| crate::hide::Target::bare(h, 0))
                    .collect();
                // 恢复工具的手动隐藏不施加副作用，仅隐藏并纳入记录。
                let mut setting = self.config.setting.clone();
                setting.mute_after_hide = false;
                setting.freeze_after_hide = false;
                setting.send_before_hide = false;
                setting.minimize_before_hide = false;
                let plan = self.hide_with_plan_using(&setting, &targets, &[], &[], &[], &[]);
                if !plan.fresh.is_empty() {
                    logging::debug(&format!("窗口恢复工具隐藏 {} 个窗口", plan.fresh.len()));
                }
                (Response::Ok, false)
            }
            Command::OpenSettings => {
                launch_settings(self, None);
                (Response::Ok, false)
            }
            Command::ResetPowerStats => {
                self.power_stats.reset();
                logging::debug("能效统计已清零");
                (Response::Ok, false)
            }
            Command::Quit => (Response::Ok, true),
        }
    }
}

/// 开机自启注册方式的日志写法。
pub(super) fn autostart_method_name(method: crate::autostart::Method) -> &'static str {
    match method {
        crate::autostart::Method::TaskScheduler => "计划任务",
        crate::autostart::Method::Registry => "注册表启动项",
    }
}

pub(super) fn set_autostart(enabled: bool, admin: bool) -> Response {
    let auto = match crate::autostart::Autostart::standard() {
        Ok(a) => a,
        Err(e) => {
            log_warn!("设置开机自启失败，无法确定核心程序路径，自启状态未改变: {e}");
            return Response::Error {
                message: e.to_string(),
            };
        }
    };
    if enabled {
        match auto.enable(admin) {
            Ok(method) => {
                logging::debug(&format!(
                    "已开启开机自启，方式为{}（管理员权限={admin}）",
                    autostart_method_name(method)
                ));
                Response::Ok
            }
            Err(e) => {
                log_warn!(
                    "开启开机自启失败，开机后核心不会自动运行（管理员权限={admin}，任务名 {}）: {e}",
                    crate::autostart::TASK_NAME
                );
                Response::Error {
                    message: e.to_string(),
                }
            }
        }
    } else {
        auto.disable();
        logging::debug("已关闭开机自启");
        Response::Ok
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
        logging::debug(&format!(
            "配置窗口此前被 ZoneDeck 隐藏，已先释放 {released} 个窗口再拉起设置"
        ));
        state.persist_recovery();
        state.sync_tray();
    }
}

/// 定位同目录下的配置程序：生产名 config.exe，开发名 zonedeck-config.exe。
pub(super) fn find_config_exe() -> Option<PathBuf> {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))?;
    ["config.exe", "zonedeck-config.exe"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|p| p.exists())
}

/// 把「打开配置界面」转交给已在运行的核心，转交不成就自己拉起配置程序。
/// 返回两条路是否有一条走通。
pub fn forward_open_settings() -> bool {
    if matches!(
        PipeClient::connect_default()
            .fast()
            .send(&Command::OpenSettings),
        Ok(Response::Ok)
    ) {
        return true;
    }
    // 管道不通或对方是不认识该命令的旧版核心；配置程序自带单实例。
    find_config_exe()
        .map(|path| std::process::Command::new(&path).spawn().is_ok())
        .unwrap_or(false)
}

/// 拉起配置程序；`action` 作为命令行参数传入。
pub(super) fn launch_settings(state: &mut AgentState, action: Option<&str>) {
    let Some(path) = find_config_exe() else {
        log_warn!(
            "核心所在目录下找不到配置程序（config.exe / zonedeck-config.exe），无法打开设置界面"
        );
        state.notify(Msg::ConfigExeMissingTitle, Msg::ConfigExeMissingBody);
        return;
    };

    release_config_windows(state, &path);

    let mut cmd = std::process::Command::new(&path);
    if let Some(arg) = action {
        cmd.arg(arg);
    }
    if let Err(e) = cmd.spawn() {
        log_warn!(
            "启动配置程序失败，设置界面未打开（可能被安全软件拦截）: {} — {e}",
            path.display()
        );
    }
}
