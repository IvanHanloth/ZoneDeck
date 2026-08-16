use std::time::{Duration, Instant};

use zonedeck_common::ipc::{Command, PipeClient, Response};
use zonedeck_core::agent::{self, AgentOptions};

#[test]
fn agent_answers_ipc_and_quits_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let mut config = zonedeck_common::Config::default();
    // 开启自动隐藏，验证 GetStatus 会如实回传该开关（供设置界面与托盘对齐）。
    config.setting.auto_hide_enabled = true;
    config.save(&config_path).unwrap();

    let pipe = r"\\.\pipe\zonedeck_test_agent_e2e";
    let options = AgentOptions {
        pipe_name: pipe.to_string(),
        enable_tray: false,
        auto_quit_ms: Some(15_000),
        ..AgentOptions::standard(config_path)
    };

    let handle = std::thread::spawn(move || agent::run(options));

    let client = PipeClient::new(pipe);

    let state = client.send(&Command::GetState).unwrap();
    assert_eq!(
        state,
        Response::State { hidden: false },
        "初始状态应为未隐藏"
    );

    let status = client.send(&Command::GetStatus).unwrap();
    assert!(
        matches!(
            status,
            Response::Status {
                hidden: false,
                auto_hide_enabled: true,
                ..
            }
        ),
        "合并状态应一次往返返回隐藏态、权限与自动隐藏开关: {status:?}"
    );

    let reload = client.send(&Command::ReloadConfig).unwrap();
    assert_eq!(reload, Response::Ok, "重载配置应成功");

    let toggle = client.send(&Command::Toggle).unwrap();
    assert_eq!(toggle, Response::Ok, "切换命令应被确认");

    let quit = client.send(&Command::Quit).unwrap();
    assert_eq!(quit, Response::Ok, "退出命令应被确认");

    let deadline = Instant::now() + Duration::from_secs(10);
    while !handle.is_finished() {
        assert!(Instant::now() < deadline, "代理线程未在退出命令后结束");
        std::thread::sleep(Duration::from_millis(50));
    }
    handle.join().unwrap();
}
