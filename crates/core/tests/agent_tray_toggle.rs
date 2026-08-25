//! 托盘图标随配置动态挂载 / 撤下：核心须全程应答，且能干净退出。

use std::time::{Duration, Instant};

use zonedeck_common::Config;
use zonedeck_common::ipc::{Command, PipeClient, Response};
use zonedeck_core::agent::{self, AgentOptions};

/// 改写配置里的托盘开关并让核心重载，返回重载的应答。
fn set_tray_enabled(client: &PipeClient, path: &std::path::Path, enabled: bool) -> Response {
    let mut config = Config::load(path).unwrap();
    config.setting.tray_enabled = enabled;
    config.save(path).unwrap();
    client.send(&Command::ReloadConfig).unwrap()
}

#[test]
fn toggling_the_tray_icon_keeps_the_core_responsive() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    Config::default().save(&config_path).unwrap();

    let pipe = r"\\.\pipe\zonedeck_test_agent_tray_toggle";
    let options = AgentOptions {
        pipe_name: pipe.to_string(),
        // 本例的重点就是托盘，不能像其余用例那样关掉它。
        enable_tray: true,
        auto_quit_ms: Some(20_000),
        ..AgentOptions::standard(config_path.clone())
    };

    let handle = std::thread::spawn(move || agent::run(options));

    let client = PipeClient::new(pipe);
    assert_eq!(
        client.send(&Command::GetState).unwrap(),
        Response::State { hidden: false },
        "核心应已就绪"
    );

    // 撤下 → 挂回 → 再撤下，每一步都得走通创建与销毁两条路径。
    for enabled in [false, true, false] {
        assert_eq!(
            set_tray_enabled(&client, &config_path, enabled),
            Response::Ok,
            "托盘开关改为 {enabled} 后重载配置应成功"
        );
        assert_eq!(
            client.send(&Command::GetState).unwrap(),
            Response::State { hidden: false },
            "托盘开关改为 {enabled} 后核心仍须应答"
        );
    }

    assert_eq!(client.send(&Command::Quit).unwrap(), Response::Ok);

    let deadline = Instant::now() + Duration::from_secs(10);
    while !handle.is_finished() {
        assert!(Instant::now() < deadline, "代理线程未在退出命令后结束");
        std::thread::sleep(Duration::from_millis(50));
    }
    handle.join().unwrap();
}
