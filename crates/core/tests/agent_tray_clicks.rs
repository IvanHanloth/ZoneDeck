//! 端到端：托盘点击按配置分发动作，且单击与双击不互相串味。
//!
//! 托盘回调是 shell 发给代理窗口的普通消息，这里直接投递同样的消息序列，
//! 从而覆盖真人点不出来的边角 —— 尤其是双击尾部那次抬起必须被吞掉。

use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DestroyWindow, DispatchMessageW, EnumThreadWindows,
    GetClassNameW, GetMessageW, MSG, PostMessageW, PostThreadMessageW, SW_SHOW, ShowWindow,
    TranslateMessage, WINDOW_EX_STYLE, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_QUIT,
    WS_OVERLAPPEDWINDOW,
};
use windows::core::w;
use zonedeck_common::ipc::{Command, PipeClient, Response};
use zonedeck_common::{Config, WindowRule};
use zonedeck_core::agent::{self, AgentOptions, WM_APP_TRAY};

/// 代理窗口的类名，见 `create_agent_window`。
const AGENT_CLASS: &str = "ZoneDeckAgentWindow";
/// 测试窗口的标题，配置里的规则按它匹配。
const TEST_WINDOW_TITLE: &str = "ZoneDeckTrayClickTestWindow";
/// 「什么都没发生」的观察窗口，须长于系统双击判定时间（默认 500 毫秒）。
const SETTLE: Duration = Duration::from_millis(1500);

/// 在带消息循环的独立线程上创建一个可见窗口，收到 WM_QUIT 后销毁。
/// 返回线程 id，供测试结束时通知它退出。
fn spawn_visible_window() -> (u32, std::thread::JoinHandle<()>) {
    let (tx, rx) = channel::<u32>();
    let handle = std::thread::spawn(move || unsafe {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            // w! 只吃字面量，须与 TEST_WINDOW_TITLE 一字不差。
            w!("ZoneDeckTrayClickTestWindow"),
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
        tx.send(GetCurrentThreadId()).unwrap();

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = DestroyWindow(hwnd);
    });
    let tid = rx.recv().expect("窗口线程未上报线程 id");
    (tid, handle)
}

unsafe extern "system" fn collect_agent_window(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    unsafe {
        let mut buf = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut buf);
        if len > 0 && String::from_utf16_lossy(&buf[..len as usize]) == AGENT_CLASS {
            *(lparam.0 as *mut isize) = hwnd.0 as isize;
            return false.into();
        }
    }
    true.into()
}

/// 找出指定线程上的代理窗口。按线程定位，避免误伤本机正在运行的真实核心。
fn agent_window(tid: u32) -> HWND {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let mut found: isize = 0;
        unsafe {
            let _ = EnumThreadWindows(
                tid,
                Some(collect_agent_window),
                LPARAM(&mut found as *mut isize as isize),
            );
        }
        if found != 0 {
            return HWND(found as *mut std::ffi::c_void);
        }
        assert!(Instant::now() < deadline, "未能在代理线程上找到代理窗口");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// 投递一条托盘回调消息，等同 shell 在用户点击图标时做的事。
fn post_tray(hwnd: HWND, mouse_msg: u32) {
    unsafe {
        PostMessageW(
            Some(hwnd),
            WM_APP_TRAY,
            WPARAM(1),
            LPARAM(mouse_msg as isize),
        )
        .expect("投递托盘回调消息失败");
    }
}

fn hidden(client: &PipeClient) -> bool {
    matches!(
        client.send(&Command::GetState).unwrap(),
        Response::State { hidden: true }
    )
}

/// 等到隐藏态变为 `want`；超时即失败。
fn wait_hidden(client: &PipeClient, want: bool, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while hidden(client) != want {
        assert!(Instant::now() < deadline, "{what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// 写入托盘点击配置并让核心重载。
fn set_clicks(client: &PipeClient, path: &std::path::Path, left: &str, double: &str) {
    let mut config = Config::load(path).unwrap();
    config.setting.tray_clicks.left = left.to_string();
    config.setting.tray_clicks.double = double.to_string();
    config.save(path).unwrap();
    assert_eq!(
        client.send(&Command::ReloadConfig).unwrap(),
        Response::Ok,
        "重载配置应成功"
    );
}

#[test]
fn tray_clicks_run_the_configured_actions() {
    let (window_tid, window_thread) = spawn_visible_window();

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let mut config = Config::default();
    // 隐藏只针对下面这条规则命中的测试窗口，别碰跑测试的人的桌面。
    config.setting.hide_current = false;
    config.window_rules = vec![WindowRule::from_regex(format!("^{TEST_WINDOW_TITLE}$"))];
    config.save(&config_path).unwrap();

    let pipe = r"\\.\pipe\zonedeck_test_tray_clicks";
    let options = AgentOptions {
        pipe_name: pipe.to_string(),
        enable_tray: false,
        auto_quit_ms: Some(30_000),
        ..AgentOptions::standard(config_path.clone())
    };
    let (tid_tx, tid_rx) = channel::<u32>();
    let agent_thread = std::thread::spawn(move || {
        tid_tx.send(unsafe { GetCurrentThreadId() }).unwrap();
        agent::run(options);
    });
    let agent_tid = tid_rx.recv().unwrap();
    let client = PipeClient::new(pipe);
    let agent_hwnd = agent_window(agent_tid);
    assert!(!hidden(&client), "核心应已就绪且初始未隐藏");

    // 1) 双击不做事时，单击立即执行左键动作，一次隐藏一次恢复。
    set_clicks(&client, &config_path, "toggle", "none");
    post_tray(agent_hwnd, WM_LBUTTONUP);
    wait_hidden(&client, true, "单击托盘未触发隐藏");
    post_tray(agent_hwnd, WM_LBUTTONUP);
    wait_hidden(&client, false, "再次单击托盘未触发恢复");

    // 2) 左键置空时，单击什么都不做。
    set_clicks(&client, &config_path, "none", "none");
    post_tray(agent_hwnd, WM_LBUTTONUP);
    std::thread::sleep(SETTLE);
    assert!(!hidden(&client), "左键置空时单击不该动窗口");

    // 3) 完整的双击序列「抬起 → 双击 → 抬起」只该执行一次动作：
    //    尾部那次抬起若没被吞掉，会在双击判定过后再翻转一次。
    set_clicks(&client, &config_path, "toggle", "toggle");
    post_tray(agent_hwnd, WM_LBUTTONUP);
    post_tray(agent_hwnd, WM_LBUTTONDBLCLK);
    post_tray(agent_hwnd, WM_LBUTTONUP);
    wait_hidden(&client, true, "双击托盘未触发双击动作");
    // 等过双击判定窗口，确认没有第二次翻转。
    std::thread::sleep(SETTLE);
    assert!(hidden(&client), "双击尾部的抬起未被吞掉，动作被执行了两次");

    // 收尾：结束窗口线程，核心会实时移除记录。
    unsafe {
        let _ = PostThreadMessageW(window_tid, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    window_thread.join().unwrap();

    assert_eq!(client.send(&Command::Quit).unwrap(), Response::Ok);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !agent_thread.is_finished() {
        assert!(Instant::now() < deadline, "代理线程未在退出命令后结束");
        std::thread::sleep(Duration::from_millis(50));
    }
    agent_thread.join().unwrap();
}
