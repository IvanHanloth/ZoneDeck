//! 能效统计的重置走 IPC：核心在运行时必须由它自己清，否则内存里的旧值会覆盖回文件。

use std::time::{Duration, Instant};

use zonedeck_common::ipc::{Command, PipeClient, Response};
use zonedeck_core::agent::{self, AgentOptions};
use zonedeck_core::stats::{self, PowerStatsStore, STATS_FILE_NAME};

#[test]
fn resetting_power_stats_over_ipc_clears_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    zonedeck_common::Config::default()
        .save(&config_path)
        .unwrap();

    // 预置一份非空成绩单，核心启动时会把它读进内存。
    let stats_path = dir.path().join(STATS_FILE_NAME);
    let seeded = PowerStatsStore::load(stats_path.clone());
    seeded.on_suspend(1234);
    seeded.flush();
    drop(seeded);
    assert!(
        !stats::read(&stats_path)
            .expect("预置的统计应能读回")
            .is_empty(),
        "重置前该有数据可清"
    );

    let pipe = r"\\.\pipe\zonedeck_test_power_stats_reset";
    let options = AgentOptions {
        pipe_name: pipe.to_string(),
        enable_tray: false,
        auto_quit_ms: Some(15_000),
        ..AgentOptions::standard(config_path)
    };
    let handle = std::thread::spawn(move || agent::run(options));

    let client = PipeClient::new(pipe);
    assert_eq!(
        client.send(&Command::ResetPowerStats).unwrap(),
        Response::Ok,
        "重置命令应被确认"
    );

    let after = stats::read(&stats_path).expect("重置应立即落盘而非等到退出");
    assert!(after.is_empty(), "重置后累计量应清零");
    assert!(after.since > 0, "重置后应重新起算");

    assert_eq!(client.send(&Command::Quit).unwrap(), Response::Ok);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !handle.is_finished() {
        assert!(Instant::now() < deadline, "代理线程未在退出命令后结束");
        std::thread::sleep(Duration::from_millis(50));
    }
    handle.join().unwrap();

    // 退出时的 flush 不得把清零前的旧值又写回去。
    assert!(
        stats::read(&stats_path)
            .expect("退出后统计文件应还在")
            .is_empty(),
        "退出时的落盘应沿用重置后的状态"
    );
}
