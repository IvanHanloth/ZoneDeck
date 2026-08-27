//! 用真实子进程走一遍隐藏 / 恢复，确认效率模式确实在恢复时被撤销。
//!
//! 与 `hide.rs` 里的用例不同：那边用 MockEffects 只验证调用序列，这里接真实的
//! [`WinEffects`]，断言落在进程上的 EcoQoS 标志与优先级上。

#![cfg(windows)]

use std::ffi::c_void;
use std::mem::size_of;
use std::sync::Arc;
use std::time::Duration;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    GetPriorityClass, GetProcessInformation, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
    OpenProcess, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
    PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
    PROCESS_QUERY_LIMITED_INFORMATION, ProcessPowerThrottling,
};
use zonedeck_common::{Setting, WindowInfo};
use zonedeck_core::effects::WinEffects;
use zonedeck_core::hide::{HideController, Target};
use zonedeck_core::platform::{Restore, WindowManager};
use zonedeck_core::stats::PowerStatsStore;

/// 一个时点上进程的能效状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Probe {
    control: u32,
    state: u32,
    priority: u32,
}

impl Probe {
    fn eco(&self) -> bool {
        self.state & PROCESS_POWER_THROTTLING_EXECUTION_SPEED != 0
    }
    /// 电源策略交还给了系统，进程身上没留下本程序的手印。
    fn system_managed(&self) -> bool {
        self.control == 0 && self.state == 0
    }
}

fn probe(pid: u32) -> Probe {
    unsafe {
        let h =
            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).expect("应能打开被测子进程");
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
        let priority = GetPriorityClass(h);
        let _ = CloseHandle(h);
        assert!(ok, "应能读回被测子进程的 EcoQoS 状态");
        Probe {
            control: state.ControlMask,
            state: state.StateMask,
            priority,
        }
    }
}

/// 只认一个窗口的窗口管理器；隐藏 / 显示只改内存里的可见位。
struct OneWindow {
    hwnd: i64,
    pid: u32,
    visible: std::sync::Mutex<bool>,
}

impl WindowManager for OneWindow {
    fn enumerate(&self) -> Vec<WindowInfo> {
        vec![WindowInfo {
            hwnd: self.hwnd,
            pid: self.pid,
            title: "被测窗口".into(),
            process: "test.exe".into(),
            path: "C:\\test.exe".into(),
            visible: *self.visible.lock().unwrap(),
        }]
    }
    fn hide(&self, _hwnd: i64) {
        *self.visible.lock().unwrap() = false;
    }
    fn show(&self, _hwnd: i64) {
        *self.visible.lock().unwrap() = true;
    }
    fn minimize(&self, _hwnd: i64) {}
    fn restore_mode(&self, _hwnd: i64) -> Restore {
        Restore::Normal
    }
    fn restore(&self, _hwnd: i64, _how: Restore) {
        *self.visible.lock().unwrap() = true;
    }
    fn is_visible(&self, _hwnd: i64) -> bool {
        *self.visible.lock().unwrap()
    }
    fn foreground(&self) -> i64 {
        self.hwnd
    }
    fn is_window(&self, hwnd: i64) -> bool {
        hwnd == self.hwnd
    }
    fn window_pid(&self, _hwnd: i64) -> u32 {
        self.pid
    }
    fn process_path(&self, _pid: u32) -> String {
        "C:\\test.exe".into()
    }
    fn window_title(&self, _hwnd: i64) -> String {
        "被测窗口".into()
    }
    fn process_start_time(&self, pid: u32) -> i64 {
        zonedeck_core::platform::win32::WindowsWindowManager.process_start_time(pid)
    }
}

/// 走一遍隐藏 → 恢复，返回基线 / 隐藏后 / 恢复后三个时点的状态。
fn roundtrip(freeze: bool) -> [Probe; 3] {
    // 阻塞在读 stdin 上的子进程：零 CPU、不会自己退出，结束时显式杀掉。
    let mut child = std::process::Command::new("cmd.exe")
        .args(["/c", "set /p x="])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("启动被测子进程失败");
    let pid = child.id();

    let stats = PowerStatsStore::load(
        std::env::temp_dir().join(format!("zonedeck-eco-test-{pid}-stats.json")),
    );
    let wm = OneWindow {
        hwnd: 0x1234,
        pid,
        visible: std::sync::Mutex::new(true),
    };
    let effects = WinEffects::new(std::env::current_dir().unwrap(), Arc::clone(&stats));
    let mut controller = HideController::new(wm, effects);

    let setting = Setting {
        efficiency_after_hide: true,
        freeze_after_hide: freeze,
        mute_after_hide: false,
        send_before_hide: false,
        hide_current: false,
        ..Setting::default()
    };

    let before = probe(pid);
    controller.apply_hide(&setting, &[Target::bare(0x1234, pid)], &[pid]);
    let hidden = probe(pid);
    controller.show();
    std::thread::sleep(Duration::from_millis(50));
    let shown = probe(pid);

    let _ = child.kill();
    let _ = child.wait();
    [before, hidden, shown]
}

/// 恢复后须与基线一致：EcoQoS 摘掉、优先级抬回、电源策略交还系统。
fn assert_round_trips(label: &str, [before, hidden, shown]: [Probe; 3]) {
    println!("{label}：基线 {before:?}｜隐藏后 {hidden:?}｜恢复后 {shown:?}");

    assert!(before.system_managed(), "基线：子进程应是系统托管的");
    assert!(hidden.eco(), "隐藏后子进程应带上 EcoQoS");
    assert_eq!(
        hidden.priority, IDLE_PRIORITY_CLASS.0,
        "隐藏后优先级应压到 Idle"
    );
    assert!(!shown.eco(), "恢复后 EcoQoS 应被撤销");
    assert_eq!(
        shown.priority, NORMAL_PRIORITY_CLASS.0,
        "恢复后优先级应抬回普通"
    );
    assert!(
        shown.system_managed(),
        "恢复后应交还系统托管；ControlMask 留着会让进程从此不再被自动放进 EcoQoS"
    );
}

#[test]
fn efficiency_is_really_lifted_when_windows_come_back() {
    assert_round_trips("只开效率模式", roundtrip(false));
}

/// 冻结与效率模式同开是默认配置，撤销顺序（先解冻再抬待遇）须照样把 EcoQoS 摘干净。
#[test]
fn efficiency_is_lifted_even_when_the_process_was_also_frozen() {
    assert_round_trips("冻结+效率模式", roundtrip(true));
}
