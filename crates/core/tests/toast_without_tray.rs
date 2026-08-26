//! 端到端：Toast 通知在没有托盘图标的情况下也能走完发送流程。
//!
//! 这是本功能的全部意义所在 —— 托盘气泡做不到（图标不在时 `Shell_NotifyIcon`
//! 直接返回失败），所以这条路径必须真的跑一遍，不能只测纯函数。
//!
//! 断言落在「开始菜单里有没有带本程序 AUMID 的快捷方式」上，而不是「通知平台
//! 认不认」：后者由 shell 异步索引，时机不由我们决定，拿来断言只会得到一个
//! 时灵时不灵的用例。快捷方式就位是我们这边能负责到底的部分。
//!
//! 用例会碰真实的开始菜单，结束时按「跑之前有没有」还原。

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, IPersistFile, STGM_READ,
};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{
    FOLDERID_Programs, IShellLinkW, KF_FLAG_DEFAULT, SHGetKnownFolderPath, ShellLink,
};
use windows::core::{HSTRING, Interface, PCWSTR};
use zonedeck_core::toast::{AUMID, ToastWorker};

/// 两个用例都以「跑之前那个快捷方式在不在」为准来收尾，并发跑会互相看错状态。
static SHORTCUT: Mutex<()> = Mutex::new(());

fn lock_shortcut() -> MutexGuard<'static, ()> {
    SHORTCUT.lock().unwrap_or_else(|e| e.into_inner())
}

fn shortcut_path() -> std::path::PathBuf {
    unsafe {
        let raw = SHGetKnownFolderPath(&FOLDERID_Programs, KF_FLAG_DEFAULT, None).unwrap();
        let dir = raw.to_string().unwrap();
        CoTaskMemFree(Some(raw.0 as *const _));
        std::path::PathBuf::from(dir).join("ZoneDeck.lnk")
    }
}

/// 读快捷方式上的 AppUserModelID。没有该属性时返回空串。
fn shortcut_aumid(path: &std::path::Path) -> String {
    unsafe {
        let Ok(link) = CoCreateInstance::<_, IShellLinkW>(&ShellLink, None, CLSCTX_INPROC_SERVER)
        else {
            return String::new();
        };
        let Ok(file) = link.cast::<IPersistFile>() else {
            return String::new();
        };
        let wide = HSTRING::from(path.as_os_str());
        if file.Load(PCWSTR(wide.as_ptr()), STGM_READ).is_err() {
            return String::new();
        }
        link.cast::<IPropertyStore>()
            .and_then(|s| s.GetValue(&PKEY_AppUserModel_ID))
            .map(|v| v.to_string())
            .unwrap_or_default()
    }
}

#[test]
fn a_toast_registers_itself_without_any_tray_icon() {
    let _guard = lock_shortcut();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let lnk = shortcut_path();
    let existed_before = lnk.exists();

    // 全程没有 TrayIcon：这里没有任何托盘图标被挂上。
    let worker = ToastWorker::spawn();
    worker
        .sender()
        .show("ZoneDeck 自测", "这条通知来自集成测试，可以忽略");
    // shutdown 会排干队列：返回时该建的已经建了、该发的已经发了。
    worker.shutdown(Duration::from_secs(10));

    let exists = lnk.exists();
    let aumid = if exists {
        shortcut_aumid(&lnk)
    } else {
        String::new()
    };

    if !existed_before {
        let _ = std::fs::remove_file(&lnk);
    }

    assert!(
        exists,
        "发过通知之后，开始菜单里应当有快捷方式；没有它 Toast 只会静默不弹：{}",
        lnk.display()
    );
    assert_eq!(
        aumid, AUMID,
        "快捷方式上的 AppUserModelID 必须与核心用的那个逐字相同，否则通知平台认不出来"
    );
}

/// 一条都不发时不该动用户的开始菜单。
#[test]
fn a_worker_that_never_notifies_touches_nothing() {
    let _guard = lock_shortcut();
    let lnk = shortcut_path();
    let existed_before = lnk.exists();

    let worker = ToastWorker::spawn();
    worker.shutdown(Duration::from_secs(5));

    assert_eq!(
        lnk.exists(),
        existed_before,
        "没发过通知就不该注册 AUMID，更不该往开始菜单写东西"
    );
}
