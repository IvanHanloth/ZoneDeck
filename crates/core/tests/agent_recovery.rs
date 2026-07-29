//! 端到端：核心启动时发现崩溃残留的 recovery.json，应恢复窗口并清除文件。

use std::time::{Duration, Instant};

use bosskey_common::ipc::{Command, PipeClient, Response};
use bosskey_core::agent::{self, AgentOptions};
use bosskey_core::hide::Target;
use bosskey_core::recovery::{self, Snapshot};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, IsWindowVisible,
    MSG, PostThreadMessageW, SW_HIDE, SW_SHOW, ShowWindow, TranslateMessage, WINDOW_EX_STYLE,
    WM_QUIT, WS_OVERLAPPEDWINDOW,
};
use windows::core::w;

/// 在带消息循环的独立线程上创建一个“已被隐藏”的窗口。
fn spawn_hidden_window() -> (i64, u32, std::thread::JoinHandle<()>) {
    let (tx, rx) = std::sync::mpsc::channel::<(i64, u32)>();
    let handle = std::thread::spawn(move || unsafe {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            w!("BossKeyRecoveryTestWindow"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            200,
            120,
            None,
            None,
            None,
            None,
        )
        .expect("创建测试窗口失败");
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = ShowWindow(hwnd, SW_HIDE);
        tx.send((hwnd.0 as isize as i64, GetCurrentThreadId()))
            .unwrap();

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = DestroyWindow(hwnd);
    });
    let (hwnd, tid) = rx.recv().expect("窗口线程未上报句柄");
    (hwnd, tid, handle)
}

fn is_visible(hwnd: i64) -> bool {
    unsafe { IsWindowVisible(HWND(hwnd as isize as *mut std::ffi::c_void)).as_bool() }
}

#[test]
fn agent_restores_hidden_windows_left_by_a_crash() {
    let (hwnd, window_tid, window_thread) = spawn_hidden_window();
    assert!(!is_visible(hwnd), "测试前提：窗口已隐藏");

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    bosskey_common::Config::default()
        .save(&config_path)
        .unwrap();

    let recovery_path = dir.path().join(recovery::RECOVERY_FILE_NAME);
    // save 会盖上版本与本次开机时刻，等价于核心崩溃前留下的真实快照。
    // 测试窗口属于本进程，pid 须如实填写，否则恢复侧的身份校验会拦下它。
    recovery::save(
        &recovery_path,
        &Snapshot {
            hidden: vec![Target::bare(hwnd, std::process::id())],
            frozen: vec![],
            muted: vec![],
            enhanced: false,
            ..Default::default()
        },
    )
    .unwrap();

    let pipe = r"\\.\pipe\bosskey_test_agent_recovery";
    let options = AgentOptions {
        pipe_name: pipe.to_string(),
        enable_tray: false,
        auto_quit_ms: Some(15_000),
        ..AgentOptions::standard(config_path)
    };
    let agent_thread = std::thread::spawn(move || agent::run(options));

    // 能应答 IPC 即代表启动流程已完成。
    let client = PipeClient::new(pipe);
    let state = client.send(&Command::GetState).unwrap();
    assert_eq!(
        state,
        Response::State { hidden: false },
        "恢复完成后核心应处于未隐藏状态"
    );

    assert!(is_visible(hwnd), "崩溃前被隐藏的窗口应在核心启动时被找回");
    assert!(!recovery_path.exists(), "恢复完成后 recovery.json 应被清除");

    let quit = client.send(&Command::Quit).unwrap();
    assert_eq!(quit, Response::Ok);

    let deadline = Instant::now() + Duration::from_secs(10);
    while !agent_thread.is_finished() {
        assert!(Instant::now() < deadline, "代理线程未在退出命令后结束");
        std::thread::sleep(Duration::from_millis(50));
    }
    agent_thread.join().unwrap();

    unsafe {
        let _ = PostThreadMessageW(window_tid, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    window_thread.join().unwrap();
}

/// 跨重启的快照里 HWND 与 PID 都已失效，核心不得执行恢复动作，只清除文件。
#[test]
fn agent_discards_snapshot_from_a_previous_boot() {
    let (hwnd, window_tid, window_thread) = spawn_hidden_window();
    assert!(!is_visible(hwnd), "测试前提：窗口已隐藏");

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    bosskey_common::Config::default()
        .save(&config_path)
        .unwrap();

    // 手工构造「上一次开机」留下的快照：boot_time_ms 远早于本次开机。
    let recovery_path = dir.path().join(recovery::RECOVERY_FILE_NAME);
    let stale = Snapshot {
        schema: recovery::SCHEMA_CURRENT,
        boot_time_ms: recovery::current_boot_time_ms() - 86_400_000,
        hidden: vec![Target::bare(hwnd, std::process::id())],
        frozen: vec![],
        muted: vec![],
        enhanced: false,
    };
    std::fs::write(&recovery_path, serde_json::to_string(&stale).unwrap()).unwrap();

    let pipe = r"\\.\pipe\bosskey_test_agent_recovery_stale";
    let options = AgentOptions {
        pipe_name: pipe.to_string(),
        enable_tray: false,
        auto_quit_ms: Some(15_000),
        ..AgentOptions::standard(config_path)
    };
    let agent_thread = std::thread::spawn(move || agent::run(options));

    let client = PipeClient::new(pipe);
    let state = client.send(&Command::GetState).unwrap();
    assert_eq!(state, Response::State { hidden: false });

    assert!(
        !is_visible(hwnd),
        "跨重启快照中的句柄不可信，不得执行恢复动作"
    );
    assert!(!recovery_path.exists(), "过期快照应被清除，不再反复触发");

    let quit = client.send(&Command::Quit).unwrap();
    assert_eq!(quit, Response::Ok);

    let deadline = Instant::now() + Duration::from_secs(10);
    while !agent_thread.is_finished() {
        assert!(Instant::now() < deadline, "代理线程未在退出命令后结束");
        std::thread::sleep(Duration::from_millis(50));
    }
    agent_thread.join().unwrap();

    unsafe {
        let _ = PostThreadMessageW(window_tid, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    window_thread.join().unwrap();
}
