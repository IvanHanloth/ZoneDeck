//! 托盘图标的点击行为分发，以及单击与双击的消歧。

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows::Win32::UI::WindowsAndMessaging::{
    KillTimer, PostMessageW, SetTimer, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP,
};
use zonedeck_common::{
    TRAY_ACTION_MENU, TRAY_ACTION_NONE, TRAY_ACTION_SETTINGS, TRAY_ACTION_TOGGLE,
};

use super::commands::launch_settings;
use super::menu::show_tray_menu;
use super::{AgentState, TRAY_CLICK_TIMER_ID, Trigger, WM_APP_IPC};

/// 单击是否必须等过双击判定窗口才能执行。
///
/// shell 发来的双击序列是「抬起 → 双击 → 抬起」，第一次抬起先到，直接执行单击
/// 动作就会在双击动作之前多跑一遍。双击不做事时没有这个冲突，单击零延迟。
pub(super) fn needs_delay(double_action: &str) -> bool {
    double_action != TRAY_ACTION_NONE
}

/// 执行一次点击动作；取值已由配置归一兜底，认不出的静默忽略。
fn run_tray_action(state: &mut AgentState, hwnd: HWND, action: &str) {
    match action {
        TRAY_ACTION_TOGGLE => state.apply_toggle(Trigger::TrayClick),
        TRAY_ACTION_MENU => {
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
        TRAY_ACTION_SETTINGS => launch_settings(state, None),
        _ => {}
    }
}

/// 处理一条托盘回调消息，`mouse_msg` 取自回调的 `lparam`。
pub(super) fn on_tray_message(state: &mut AgentState, hwnd: HWND, mouse_msg: u32) {
    match mouse_msg {
        WM_LBUTTONUP => on_left_up(state, hwnd),
        WM_LBUTTONDBLCLK => on_left_double(state, hwnd),
        WM_RBUTTONUP => {
            let action = state.config.setting.tray_clicks.right.clone();
            run_tray_action(state, hwnd, &action);
        }
        _ => {}
    }
}

fn on_left_up(state: &mut AgentState, hwnd: HWND) {
    // 双击后面还跟着一次抬起，吞掉它，否则会再排一次单击。
    if state.tray_swallow_up {
        state.tray_swallow_up = false;
        return;
    }
    if !needs_delay(&state.config.setting.tray_clicks.double) {
        let action = state.config.setting.tray_clicks.left.clone();
        run_tray_action(state, hwnd, &action);
        return;
    }
    state.tray_click_pending = true;
    unsafe {
        SetTimer(Some(hwnd), TRAY_CLICK_TIMER_ID, GetDoubleClickTime(), None);
    }
}

fn on_left_double(state: &mut AgentState, hwnd: HWND) {
    cancel_pending_click(state, hwnd);
    state.tray_swallow_up = true;
    let action = state.config.setting.tray_clicks.double.clone();
    run_tray_action(state, hwnd, &action);
}

/// 双击判定窗口到点：这次确实是单击，执行挂起的动作。
pub(super) fn on_click_timer(state: &mut AgentState, hwnd: HWND) {
    if !state.tray_click_pending {
        cancel_pending_click(state, hwnd);
        return;
    }
    // 先停表：动作可能弹出模态菜单，期间定时器不该再响。
    cancel_pending_click(state, hwnd);
    let action = state.config.setting.tray_clicks.left.clone();
    run_tray_action(state, hwnd, &action);
}

fn cancel_pending_click(state: &mut AgentState, hwnd: HWND) {
    state.tray_click_pending = false;
    unsafe {
        let _ = KillTimer(Some(hwnd), TRAY_CLICK_TIMER_ID);
    }
}

/// 丢弃未决的点击状态，供托盘图标被撤下时清场。
pub(super) fn reset(state: &mut AgentState, hwnd: HWND) {
    cancel_pending_click(state, hwnd);
    state.tray_swallow_up = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_click_only_waits_when_the_double_click_is_bound() {
        assert!(
            !needs_delay(TRAY_ACTION_NONE),
            "双击不做事时单击不该等，否则平白慢半拍"
        );
        for action in [TRAY_ACTION_TOGGLE, TRAY_ACTION_MENU, TRAY_ACTION_SETTINGS] {
            assert!(needs_delay(action), "{action} 绑在双击上时单击须等判定");
        }
    }
}
