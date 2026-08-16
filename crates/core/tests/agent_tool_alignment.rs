//! 端到端：窗口恢复工具经 IPC 隐藏 / 释放窗口，核心记录与恢复文件同步更新。

use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, IsWindowVisible,
    MSG, PostThreadMessageW, SW_SHOW, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WM_QUIT,
    WS_OVERLAPPEDWINDOW,
};
use windows::core::w;
use zonedeck_common::ipc::{Command, PipeClient, Response};
use zonedeck_core::agent::{self, AgentOptions};
use zonedeck_core::recovery;

/// 在带消息循环的独立线程上创建一个可见窗口。
fn spawn_visible_window() -> (i64, u32, std::thread::JoinHandle<()>) {
    let (tx, rx) = std::sync::mpsc::channel::<(i64, u32)>();
    let handle = std::thread::spawn(move || unsafe {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            w!("ZoneDeckToolAlignmentTestWindow"),
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
fn adopt_and_release_keep_core_records_and_recovery_file_in_sync() {
    let (hwnd, window_tid, window_thread) = spawn_visible_window();
    assert!(is_visible(hwnd), "测试前提：窗口可见");

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    zonedeck_common::Config::default()
        .save(&config_path)
        .unwrap();
    let recovery_path = dir.path().join(recovery::RECOVERY_FILE_NAME);

    let pipe = r"\\.\pipe\zonedeck_test_tool_alignment";
    let options = AgentOptions {
        pipe_name: pipe.to_string(),
        enable_tray: false,
        auto_quit_ms: Some(15_000),
        ..AgentOptions::standard(config_path)
    };
    let agent_thread = std::thread::spawn(move || agent::run(options));
    let client = PipeClient::new(pipe);

    // 恢复工具隐藏：窗口不可见、核心记录为隐藏态、恢复文件已写出。
    let reply = client
        .send(&Command::AdoptWindows { hwnds: vec![hwnd] })
        .unwrap();
    assert_eq!(reply, Response::Ok);
    assert!(!is_visible(hwnd), "经核心隐藏后窗口应不可见");
    assert_eq!(
        client.send(&Command::GetState).unwrap(),
        Response::State { hidden: true },
        "核心记录应包含工具隐藏的窗口"
    );
    assert!(recovery_path.exists(), "工具隐藏的窗口应受崩溃恢复保护");

    // 恢复工具释放：窗口找回、记录清空、恢复文件删除。
    let reply = client
        .send(&Command::ReleaseWindows { hwnds: vec![hwnd] })
        .unwrap();
    assert_eq!(reply, Response::Ok);
    assert!(is_visible(hwnd), "释放后窗口应恢复可见");
    assert_eq!(
        client.send(&Command::GetState).unwrap(),
        Response::State { hidden: false }
    );
    assert!(!recovery_path.exists(), "记录清空后恢复文件应删除");

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
