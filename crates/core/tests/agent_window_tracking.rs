//! 端到端：被隐藏的窗口自行销毁后，窗口事件追踪应实时移除记录并更新恢复文件。

use std::time::{Duration, Instant};

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, MSG,
    PostThreadMessageW, SW_SHOW, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WM_QUIT,
    WS_OVERLAPPEDWINDOW,
};
use windows::core::w;
use zonedeck_common::ipc::{Command, PipeClient, Response};
use zonedeck_core::agent::{self, AgentOptions};
use zonedeck_core::recovery;

/// 在带消息循环的独立线程上创建一个可见窗口，收到 WM_QUIT 后销毁。
fn spawn_visible_window() -> (i64, u32, std::thread::JoinHandle<()>) {
    let (tx, rx) = std::sync::mpsc::channel::<(i64, u32)>();
    let handle = std::thread::spawn(move || unsafe {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            w!("ZoneDeckTrackingTestWindow"),
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

#[test]
fn destroying_a_hidden_window_clears_the_record_in_real_time() {
    let (hwnd, window_tid, window_thread) = spawn_visible_window();

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    zonedeck_common::Config::default()
        .save(&config_path)
        .unwrap();
    let recovery_path = dir.path().join(recovery::RECOVERY_FILE_NAME);

    let pipe = r"\\.\pipe\zonedeck_test_window_tracking";
    let options = AgentOptions {
        pipe_name: pipe.to_string(),
        enable_tray: false,
        auto_quit_ms: Some(15_000),
        ..AgentOptions::standard(config_path)
    };
    let agent_thread = std::thread::spawn(move || agent::run(options));
    let client = PipeClient::new(pipe);

    let reply = client
        .send(&Command::AdoptWindows { hwnds: vec![hwnd] })
        .unwrap();
    assert_eq!(reply, Response::Ok);
    assert_eq!(
        client.send(&Command::GetState).unwrap(),
        Response::State { hidden: true }
    );
    assert!(recovery_path.exists());

    // 结束窗口线程后事件钩子应让核心实时移除记录。
    unsafe {
        let _ = PostThreadMessageW(window_tid, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    window_thread.join().unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if client.send(&Command::GetState).unwrap() == (Response::State { hidden: false }) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "窗口销毁后核心未在时限内移除隐藏记录"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!recovery_path.exists(), "记录清空后恢复文件应同步清除");

    let quit = client.send(&Command::Quit).unwrap();
    assert_eq!(quit, Response::Ok);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !agent_thread.is_finished() {
        assert!(Instant::now() < deadline, "代理线程未在退出命令后结束");
        std::thread::sleep(Duration::from_millis(50));
    }
    agent_thread.join().unwrap();
}
