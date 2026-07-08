use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::time::{Duration, Instant};

use bosskey_core::agent::{self, AgentOptions};

fn connect_with_retry(pipe_name: &str) -> File {
    for _ in 0..100 {
        if let Ok(f) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(pipe_name)
        {
            return f;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    panic!("无法连接代理管道 {pipe_name}");
}

fn request(file: &mut File, line: &str) -> String {
    writeln!(file, "{line}").unwrap();
    file.flush().unwrap();
    let mut reader = BufReader::new(file.try_clone().unwrap());
    let mut buf = String::new();
    reader.read_line(&mut buf).unwrap();
    buf.trim_end().to_string()
}

#[test]
fn agent_answers_ipc_and_quits_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    bosskey_common::Config::default()
        .save(&config_path)
        .unwrap();

    let pipe = r"\\.\pipe\bosskey_test_agent_e2e";
    let options = AgentOptions {
        config_path,
        pipe_name: pipe.to_string(),
        enable_tray: false,
        auto_quit_ms: Some(15_000),
    };

    let handle = std::thread::spawn(move || agent::run(options));

    let mut client = connect_with_retry(pipe);

    let state = request(&mut client, r#"{"cmd":"get_state"}"#);
    assert_eq!(
        state, r#"{"type":"state","hidden":false}"#,
        "初始状态应为未隐藏"
    );

    let reload = request(&mut client, r#"{"cmd":"reload_config"}"#);
    assert_eq!(reload, r#"{"type":"ok"}"#, "重载配置应成功");

    let quit = request(&mut client, r#"{"cmd":"quit"}"#);
    assert_eq!(quit, r#"{"type":"ok"}"#, "退出命令应被确认");

    let deadline = Instant::now() + Duration::from_secs(10);
    while !handle.is_finished() {
        assert!(Instant::now() < deadline, "代理线程未在退出命令后结束");
        std::thread::sleep(Duration::from_millis(50));
    }
    handle.join().unwrap();
}
