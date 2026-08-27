//! 能效统计的端到端记账：拿真实子进程走一遍冻结与效率模式，核对落盘的成绩单。
//!
//! 单测只覆盖 store 自身的算术，这里补上 [`WinEffects`] 到文件之间的那一段：
//! 副作用真正生效了才记账、内存释放量是量出来的、时长在解冻时结算。

use std::time::Duration;

use zonedeck_core::effects::{Effects, WinEffects};
use zonedeck_core::stats::{self, PowerStatsStore, STATS_FILE_NAME};

fn spawn_child() -> std::process::Child {
    std::process::Command::new("cmd")
        .args(["/c", "ping -n 30 127.0.0.1 >nul"])
        .spawn()
        .expect("无法启动测试子进程")
}

#[test]
fn a_full_freeze_cycle_lands_in_the_stats_file() {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let path = dir.path().join(STATS_FILE_NAME);
    let store = PowerStatsStore::load(path.clone());
    // exe_dir 指向空目录，没有 pssuspend，本次一律走普通冻结。
    let effects = WinEffects::new(dir.path().to_path_buf(), store.clone());

    let mut child = spawn_child();
    let pid = child.id();

    effects.set_efficiency(pid);
    effects.suspend(pid, false);
    effects.trim_working_set(pid);
    // 时长要有可测的跨度，否则 elapsed 可能舍成 0。
    std::thread::sleep(Duration::from_millis(30));
    effects.resume(pid, false);
    effects.clear_efficiency(pid);
    store.flush();

    let saved = stats::read(&path).expect("统计应已落盘");
    assert_eq!(saved.freeze_count, 1, "一次冻结记一笔");
    assert_eq!(saved.efficiency_count, 1, "一次效率模式记一笔");
    assert!(saved.freeze_seconds > 0.0, "解冻时应结算冻结时长");
    assert!(saved.efficiency_seconds > 0.0, "撤销效率模式时应结算其时长");
    assert!(
        saved.memory_freed_bytes > 0,
        "清空工作集应量出实际换出的内存"
    );
    assert!(saved.since > 0, "首笔记录应盖上起始时刻");

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn a_dead_process_is_not_counted() {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let path = dir.path().join(STATS_FILE_NAME);
    let store = PowerStatsStore::load(path.clone());
    let effects = WinEffects::new(dir.path().to_path_buf(), store.clone());

    // 副作用施加失败时不得记账，否则成绩单里全是没发生过的事。
    effects.suspend(0xFFFF_FFF0, false);
    effects.set_efficiency(0xFFFF_FFF0);
    effects.trim_working_set(0xFFFF_FFF0);
    store.flush();

    assert!(store.snapshot().is_empty(), "失败的副作用不该计入统计");
    assert_eq!(stats::read(&path), None, "一笔都没记时不该落盘");
}
