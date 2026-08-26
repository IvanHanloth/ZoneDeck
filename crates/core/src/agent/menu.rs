//! 托盘菜单的构建与菜单项对应的动作。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, MF_CHECKED, MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
};
use windows::core::PCWSTR;

use crate::i18n::Msg;
use crate::util::append_menu_item;
use crate::{log_warn, logging};

use super::commands::autostart_method_name;
use super::{
    AgentState, MENU_ABOUT, MENU_AUTO_HIDE, MENU_AUTOSTART, MENU_QUIT, MENU_RESTORE, MENU_SETTINGS,
    MENU_TOGGLE,
};

/// 托盘菜单切换自动隐藏：翻转配置并落盘，与在设置界面切换等效。
pub(super) fn toggle_auto_hide(state: &mut AgentState, hwnd: HWND) {
    let enabled = !state.config.setting.auto_hide_enabled;
    state.config.setting.auto_hide_enabled = enabled;
    logging::debug(if enabled {
        "托盘菜单：已启用自动隐藏"
    } else {
        "托盘菜单：已暂停自动隐藏"
    });
    if let Err(e) = state.config.save(&state.config_path) {
        log_warn!("自动隐藏开关写入配置失败，核心重启后将丢失本次切换: {e}");
    }
    state.refresh_runtime(hwnd);
}

pub(super) fn toggle_autostart(state: &AgentState) {
    let auto = match crate::autostart::Autostart::standard() {
        Ok(auto) => auto,
        Err(e) => {
            log_warn!("托盘菜单切换开机自启失败，无法确定核心程序路径，自启状态未改变: {e}");
            return;
        }
    };
    let admin = state.config.setting.autostart_admin;
    let (title, message) = if auto.status().is_some() {
        auto.disable();
        logging::debug("托盘菜单：已关闭开机自启");
        (Msg::AutostartOffTitle, Msg::AutostartOffBody)
    } else {
        match auto.enable(admin) {
            Ok(method) => {
                logging::debug(&format!(
                    "托盘菜单：已开启开机自启，方式为{}（管理员权限={admin}）",
                    autostart_method_name(method)
                ));
                match method {
                    crate::autostart::Method::TaskScheduler if admin => {
                        (Msg::AutostartOnTitle, Msg::AutostartOnTaskAdmin)
                    }
                    crate::autostart::Method::TaskScheduler => {
                        (Msg::AutostartOnTitle, Msg::AutostartOnTaskUser)
                    }
                    crate::autostart::Method::Registry => {
                        (Msg::AutostartOnTitle, Msg::AutostartOnRegistry)
                    }
                }
            }
            Err(e) => {
                log_warn!(
                    "托盘菜单开启开机自启失败，开机后核心不会自动运行（管理员权限={admin}，任务名 {}）: {e}",
                    crate::autostart::TASK_NAME
                );
                (Msg::AutostartFailTitle, Msg::AutostartFailBody)
            }
        }
    };
    if state.config.notifications.on_autostart {
        state.notify(title, message);
    }
}

pub(super) fn show_tray_menu(hwnd: HWND, hidden: bool, auto_hide_on: bool) -> bool {
    let autostart_on = crate::autostart::Autostart::standard()
        .map(|a| a.status().is_some())
        .unwrap_or(false);
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
    crate::util::show_popup_menu(hwnd, TPM_LEFTALIGN | TPM_BOTTOMALIGN, |menu| unsafe {
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
    })
}
