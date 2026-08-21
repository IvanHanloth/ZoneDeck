//! 进程效率模式：让进程继续运行，但只吃能效核心、低频、低优先级。
//!
//! 与冻结的分工：冻结把进程整个挂起（彻底不跑），效率模式让它照常跑完手头的活，
//! 只是跑得慢、耗电少。适合不能被停掉的后台程序。
//!
//! 实现即任务管理器「效率模式」的两件事：EcoQoS（`PROCESS_POWER_THROTTLING_EXECUTION_SPEED`）
//! 加 `IDLE_PRIORITY_CLASS`，全是原生 Win32，不依赖外部工具。
//! Win10 1709+ 该 API 即存在（降级为执行速度节流），Win11 22000+ 才是完整 EcoQoS。

use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    GetPriorityClass, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, OpenProcess,
    PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
    PROCESS_POWER_THROTTLING_STATE, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION,
    ProcessPowerThrottling, SetPriorityClass, SetProcessInformation,
};

#[derive(Debug, thiserror::Error)]
pub enum EfficiencyError {
    /// `OpenProcess` 失败，多为权限不足或进程已退出；带上系统错误码便于排查。
    #[error("OpenProcess 失败（需要 PROCESS_SET_INFORMATION 权限）: {0}")]
    OpenFailed(String),
    #[error("设置 EcoQoS 失败: {0}")]
    ThrottlingFailed(String),
}

/// 打开进程，要到改信息与读优先级所需的权限。
fn open(pid: u32) -> Result<windows::Win32::Foundation::HANDLE, EfficiencyError> {
    unsafe {
        OpenProcess(
            PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        )
        .map_err(|e| EfficiencyError::OpenFailed(crate::util::win_err(&e)))
    }
}

/// 施加或撤销 EcoQoS。`on=false` 时保留 ControlMask 只清 StateMask，
/// 这是文档规定的「交还系统托管」写法，不能连 ControlMask 一起清。
unsafe fn set_eco(
    handle: windows::Win32::Foundation::HANDLE,
    on: bool,
) -> Result<(), EfficiencyError> {
    let state = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        StateMask: if on {
            PROCESS_POWER_THROTTLING_EXECUTION_SPEED
        } else {
            0
        },
    };
    unsafe {
        SetProcessInformation(
            handle,
            ProcessPowerThrottling,
            &state as *const _ as *const c_void,
            size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )
        .map_err(|e| EfficiencyError::ThrottlingFailed(crate::util::win_err(&e)))
    }
}

/// 开启效率模式。
///
/// 只有原本是普通优先级的进程才降到 Idle —— 进程自己设过高／低优先级，
/// 说明它对调度有诉求，压下去可能坏事，那就只给 EcoQoS。
/// 正因为只动普通优先级这一档，还原目标恒为普通，[`disable`] 不必记账。
pub fn enable(pid: u32) -> Result<(), EfficiencyError> {
    unsafe {
        let handle = open(pid)?;
        let result = set_eco(handle, true);
        if result.is_ok() && GetPriorityClass(handle) == NORMAL_PRIORITY_CLASS.0 {
            let _ = SetPriorityClass(handle, IDLE_PRIORITY_CLASS);
        }
        let _ = CloseHandle(handle);
        result
    }
}

/// 关闭效率模式，并把 [`enable`] 压下去的优先级抬回普通。
///
/// 只在当前确实是 Idle 时才抬 —— 期间进程自己改过优先级就不越俎代庖。
/// 无状态，因此崩溃重启后的恢复流程可以直接调用。
pub fn disable(pid: u32) -> Result<(), EfficiencyError> {
    unsafe {
        let handle = open(pid)?;
        let result = set_eco(handle, false);
        if GetPriorityClass(handle) == IDLE_PRIORITY_CLASS.0 {
            let _ = SetPriorityClass(handle, NORMAL_PRIORITY_CLASS);
        }
        let _ = CloseHandle(handle);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Threading::{
        ABOVE_NORMAL_PRIORITY_CLASS, GetProcessInformation, PROCESS_CREATION_FLAGS,
    };

    /// 读回进程当前的 EcoQoS 状态位；用来确认标志确实落到了进程上，
    /// 而不是只看 SetProcessInformation 返回成功。
    fn eco_enabled(pid: u32) -> bool {
        unsafe {
            let h = open(pid).expect("应能打开自身进程");
            let mut state = PROCESS_POWER_THROTTLING_STATE {
                Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
                ..Default::default()
            };
            let ok = GetProcessInformation(
                h,
                ProcessPowerThrottling,
                &mut state as *mut _ as *mut c_void,
                size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
            )
            .is_ok();
            let _ = CloseHandle(h);
            ok && state.StateMask & PROCESS_POWER_THROTTLING_EXECUTION_SPEED != 0
        }
    }

    /// 拿当前进程走一遍开关。两种情形合在一个用例里跑：它们共用同一个进程的
    /// 优先级，拆开会被测试框架并行调度而互相打架。
    #[test]
    fn efficiency_mode_round_trips_on_the_current_process() {
        let pid = std::process::id();
        let priority_now = || unsafe {
            let h = open(pid).expect("应能打开自身进程");
            let p = GetPriorityClass(h);
            let _ = CloseHandle(h);
            p
        };
        let set_priority = |class| unsafe {
            let h = open(pid).expect("应能打开自身进程");
            let ok = SetPriorityClass(h, class).is_ok();
            let _ = CloseHandle(h);
            ok
        };

        let original = priority_now();

        // 情形一：普通优先级会被压到 Idle，关闭后抬回普通。
        if original == NORMAL_PRIORITY_CLASS.0 {
            enable(pid).expect("对自身进程开启效率模式应当成功");
            assert!(eco_enabled(pid), "EcoQoS 标志应已落到进程上");
            assert_eq!(priority_now(), IDLE_PRIORITY_CLASS.0, "优先级应已降到 Idle");
            disable(pid).expect("关闭效率模式应当成功");
            assert!(!eco_enabled(pid), "EcoQoS 标志应已撤销");
            assert_eq!(priority_now(), NORMAL_PRIORITY_CLASS.0, "优先级应抬回普通");
        }

        // 情形二：进程自设过优先级的，只吃 EcoQoS，优先级不受摆布。
        if set_priority(ABOVE_NORMAL_PRIORITY_CLASS) {
            enable(pid).expect("开启效率模式应当成功");
            assert!(eco_enabled(pid), "即便不动优先级，EcoQoS 也该照常施加");
            assert_eq!(
                priority_now(),
                ABOVE_NORMAL_PRIORITY_CLASS.0,
                "非普通优先级不该被压到 Idle"
            );
            disable(pid).expect("关闭效率模式应当成功");
            assert!(!eco_enabled(pid), "EcoQoS 标志应已撤销");
            assert_eq!(
                priority_now(),
                ABOVE_NORMAL_PRIORITY_CLASS.0,
                "没被压过的优先级不该被 disable 改成普通"
            );
            set_priority(PROCESS_CREATION_FLAGS(original));
        }
    }

    #[test]
    fn invalid_pid_reports_error_instead_of_panicking() {
        assert!(matches!(
            enable(u32::MAX),
            Err(EfficiencyError::OpenFailed(_))
        ));
        assert!(matches!(
            disable(u32::MAX),
            Err(EfficiencyError::OpenFailed(_))
        ));
    }
}
