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
/// 跨线程 ShowWindow 会向窗口属主线程发消息，属主必须泵消息才能响应。
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
    // 造一个“崩溃现场”：真实窗口已被隐藏，恢复文件记录着它。
    let (hwnd, window_tid, window_thread) = spawn_hidden_window();
    assert!(!is_visible(hwnd), "测试前提：窗口已隐藏");

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    bosskey_common::Config::default()
        .save(&config_path)
        .unwrap();

    let recovery_path = dir.path().join(recovery::RECOVERY_FILE_NAME);
    recovery::save(
        &recovery_path,
        &Snapshot {
            hidden: vec![Target { hwnd, pid: 0 }],
            frozen: vec![],
            muted: vec![],
            enhanced: false,
        },
    )
    .unwrap();

    let pipe = r"\\.\pipe\bosskey_test_agent_recovery";
    let options = AgentOptions {
        config_path,
        pipe_name: pipe.to_string(),
        enable_tray: false,
        auto_quit_ms: Some(15_000),
    };
    let agent_thread = std::thread::spawn(move || agent::run(options));

    // 能应答 IPC 即代表启动流程（含崩溃恢复）已完成。
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
