//! Win10/11 的 Toast 通知。与托盘气泡不同，它不需要托盘图标存在。
//!
//! 代价是 Toast 只认 AppUserModelID：未打包的 Win32 程序必须在开始菜单里有一个
//! 带该 AUMID 属性的快捷方式，否则 `Show` 会返回成功但什么都不弹。安装版由安装器
//! 建好；旧版安装器没写该属性的，给它补上；便携版则在真要弹第一条通知时自建一个
//! （见 [`ensure_registered`]）。
//!
//! 通知走专职线程：WinRT 要求单元化的 COM，而代理线程是消息循环，不宜改动它的
//! 单元模型；顺带也把首次注册那一两秒的等待挪出了消息循环。

use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize, IPersistFile, STGM, STGM_READ, STGM_READWRITE,
};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{
    FOLDERID_CommonPrograms, FOLDERID_Programs, IShellLinkW, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
    SetCurrentProcessExplicitAppUserModelID, ShellLink,
};
use windows::core::{HSTRING, Interface, PCWSTR};
use zonedeck_common::APP_NAME;

use crate::{log_warn, logging};

/// 通知平台据此认领本程序。安装器写进快捷方式的值必须与它逐字相同，
/// 见 `.github/inno-script/ZoneDeck.iss` 的 `AppUserModelID`。
pub const AUMID: &str = "IvanHanloth.ZoneDeck";

/// 自建快捷方式的文件名。与安装版建的那个同名，装过的机器上不会多出一个。
const SHORTCUT_NAME: &str = "ZoneDeck.lnk";

/// 新建快捷方式后等 shell 把 AUMID 收进通知平台的时长。实测 1 秒偏紧，留些余量。
const REGISTER_SETTLE: Duration = Duration::from_millis(1500);

enum Task {
    Show { title: String, body: String },
    Quit,
}

/// 通知专职线程句柄；`shutdown` 排干队列后退出。
pub struct ToastWorker {
    tx: Sender<Task>,
    handle: Option<JoinHandle<()>>,
}

impl ToastWorker {
    /// 启动通知线程。
    pub fn spawn() -> Self {
        Self::start(true)
    }

    /// 只收不发的通知线程：冒烟测试不该往用户的开始菜单写快捷方式，也不该弹通知。
    pub fn silent() -> Self {
        Self::start(false)
    }

    fn start(deliver: bool) -> Self {
        let (tx, rx) = channel::<Task>();
        let handle = std::thread::Builder::new()
            .name("zonedeck-toast".into())
            .spawn(move || unsafe {
                if !deliver {
                    // 照常排干队列，让 shutdown 的语义保持一致。
                    while let Ok(task) = rx.recv() {
                        if matches!(task, Task::Quit) {
                            break;
                        }
                    }
                    return;
                }

                // WinRT 要走单元化 COM；本线程只做通知，单元模型不影响别处。
                let com = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                let _ = SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(AUMID));

                // 首条通知才去确认注册，全程不弹通知的用户不会被写快捷方式。
                let mut registered: Option<bool> = None;
                while let Ok(task) = rx.recv() {
                    match task {
                        Task::Show { title, body } => {
                            if *registered.get_or_insert_with(ensure_registered) {
                                show_now(&title, &body);
                            }
                        }
                        Task::Quit => break,
                    }
                }

                if com.is_ok() {
                    CoUninitialize();
                }
            })
            .expect("创建通知线程失败");
        Self {
            tx,
            handle: Some(handle),
        }
    }

    pub fn sender(&self) -> ToastSender {
        ToastSender {
            tx: self.tx.clone(),
        }
    }

    /// 排干队列并结束线程；超时放弃等待。
    pub fn shutdown(mut self, timeout: Duration) {
        let _ = self.tx.send(Task::Quit);
        let Some(handle) = self.handle.take() else {
            return;
        };
        let deadline = Instant::now() + timeout;
        while !handle.is_finished() {
            if Instant::now() >= deadline {
                logging::debug("通知线程未在时限内排干队列，放弃等待；末尾的通知可能没弹出来");
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = handle.join();
    }
}

/// 把通知入队的发送端。
#[derive(Clone)]
pub struct ToastSender {
    tx: Sender<Task>,
}

impl ToastSender {
    pub fn show(&self, title: &str, body: &str) {
        let _ = self.tx.send(Task::Show {
            title: title.to_string(),
            body: body.to_string(),
        });
    }
}

/// 确保开始菜单里有带本程序 AUMID 的快捷方式，返回随后是否值得尝试发送。
///
/// 判据刻意选「文件在不在」，而不是问通知平台 `ToastNotifier::Setting()`：
/// 后者只在 shell 把新快捷方式索引进通知平台之后才为真，而那是异步的、时机不定；
/// 拿它当发送门禁会让首次运行的通知无谓地丢掉。`Show` 在平台尚未认领时只是
/// 静默不弹，不会有别的副作用，所以宁可发了不响，也不要该响的不发。
fn ensure_registered() -> bool {
    if existing_shortcut().is_some() {
        return true;
    }

    // 先试着认领已有的那条，别急着另建：3.1.0 及更早的安装器不写 AUMID，
    // 直接自建会让开始菜单里并排出现两个 ZoneDeck。
    if let Some(path) = adoptable_shortcut() {
        match set_shortcut_aumid(&path) {
            Ok(()) => {
                settle_for_shell();
                logging::debug(&format!(
                    "已给开始菜单里的快捷方式补上 AppUserModelID（旧版安装器没写）: {}｜通知平台已认领={}",
                    path.display(),
                    notifier_ready()
                ));
                return true;
            }
            // 所有用户目录下的那份要管理员权限才改得动；改不了就退回自建。
            Err(e) => log_warn!(
                "给已有快捷方式补写 AppUserModelID 失败，改为在开始菜单另建一条: {} — {}",
                path.display(),
                crate::util::win_err(&e)
            ),
        }
    }

    let Some(path) = user_shortcut_path() else {
        log_warn!("定位不到开始菜单目录，无法注册通知，本次运行不弹通知");
        return false;
    };
    if let Err(e) = create_shortcut(&path) {
        log_warn!(
            "创建通知所需的快捷方式失败，本次运行不弹通知: {} — {}",
            path.display(),
            crate::util::win_err(&e)
        );
        return false;
    }

    settle_for_shell();
    logging::debug(&format!(
        "已在开始菜单创建通知所需的快捷方式（Toast 只认注册过的 AppUserModelID）: {}｜通知平台已认领={}",
        path.display(),
        notifier_ready()
    ));
    true
}

/// 给 shell 一点时间把快捷方式上的 AUMID 收进通知平台。等不到也照发不误，
/// 最坏是这一条不响，下次启动时快捷方式已经在了。
fn settle_for_shell() {
    std::thread::sleep(REGISTER_SETTLE);
}

/// 通知平台是否已认领本程序的 AUMID。仅用于日志：它为假不代表这条通知一定不响。
fn notifier_ready() -> bool {
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))
        .and_then(|n| n.Setting())
        .is_ok()
}

/// 开始菜单里可能放着本程序快捷方式的位置。
///
/// 覆盖安装版与便携版各自会用到的地方：安装器把快捷方式放进 `{group}`，
/// 即 `<开始菜单>\Programs\ZoneDeck\`，按安装权限落在当前用户或所有用户下；
/// 便携版自建的那个则直接放在 `Programs\` 根下。
fn shortcut_candidates() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    if let Some(dir) = known_folder(&FOLDERID_Programs) {
        roots.push(dir);
    }
    if let Some(dir) = known_folder(&FOLDERID_CommonPrograms) {
        roots.push(dir);
    }
    roots
        .into_iter()
        .flat_map(|root| {
            [
                root.join(SHORTCUT_NAME),
                root.join(APP_NAME).join(SHORTCUT_NAME),
            ]
        })
        .collect()
}

/// 开始菜单里已有的、带本程序 AUMID 的快捷方式。
fn existing_shortcut() -> Option<std::path::PathBuf> {
    shortcut_candidates()
        .into_iter()
        .find(|p| shortcut_has_aumid(p))
}

/// 开始菜单里指向本程序、却没有 AUMID 的快捷方式。
///
/// 只认目标就是本 exe 的那条：便携版不该去篡改安装版的快捷方式，
/// 反之亦然，两边指向的 exe 不同，各自另建才是对的。
fn adoptable_shortcut() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    shortcut_candidates()
        .into_iter()
        .find(|p| is_adoptable(p, &exe))
}

/// 这条快捷方式是不是「指向 `exe` 却缺 AUMID」的那一种。
fn is_adoptable(path: &std::path::Path, exe: &std::path::Path) -> bool {
    path.exists()
        && !shortcut_has_aumid(path)
        && read_shortcut_target(path).is_ok_and(|target| same_path(&target, exe))
}

/// Windows 路径不区分大小写。
fn same_path(a: &std::path::Path, b: &std::path::Path) -> bool {
    a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
}

/// 快捷方式存在且它的 AppUserModelID 正是本程序的。
///
/// 旧版本安装的快捷方式没有这个属性，光看文件在不在会误判成已注册。
fn shortcut_has_aumid(path: &std::path::Path) -> bool {
    if !path.exists() {
        return false;
    }
    read_shortcut_aumid(path).is_ok_and(|id| id == AUMID)
}

fn read_shortcut_aumid(path: &std::path::Path) -> windows::core::Result<String> {
    unsafe {
        let link = load_shortcut(path, STGM_READ)?;
        let store: IPropertyStore = link.cast()?;
        let value = store.GetValue(&PKEY_AppUserModel_ID)?;
        Ok(value.to_string())
    }
}

/// 快捷方式指向的可执行文件路径。
fn read_shortcut_target(path: &std::path::Path) -> windows::core::Result<std::path::PathBuf> {
    unsafe {
        let link = load_shortcut(path, STGM_READ)?;
        let mut buf = [0u16; MAX_PATH as usize];
        link.GetPath(&mut buf, std::ptr::null_mut(), 0)?;
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(std::path::PathBuf::from(String::from_utf16_lossy(
            &buf[..len],
        )))
    }
}

/// 给已有的快捷方式补上本程序的 AUMID，其余属性原样保留。
fn set_shortcut_aumid(path: &std::path::Path) -> windows::core::Result<()> {
    unsafe {
        let link = load_shortcut(path, STGM_READWRITE)?;
        let store: IPropertyStore = link.cast()?;
        store.SetValue(&PKEY_AppUserModel_ID, &PROPVARIANT::from(AUMID))?;
        store.Commit()?;

        let file: IPersistFile = link.cast()?;
        let wide = HSTRING::from(path.as_os_str());
        file.Save(PCWSTR(wide.as_ptr()), true)
    }
}

/// 把磁盘上的快捷方式读进一个 `IShellLinkW`。
/// 要改写它就得用 [`STGM_READWRITE`] 打开，只读句柄存不回同一个文件。
fn load_shortcut(path: &std::path::Path, mode: STGM) -> windows::core::Result<IShellLinkW> {
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        let file: IPersistFile = link.cast()?;
        let wide = HSTRING::from(path.as_os_str());
        file.Load(PCWSTR(wide.as_ptr()), mode)?;
        Ok(link)
    }
}

/// per-user 开始菜单里自建快捷方式的目标路径。
fn user_shortcut_path() -> Option<std::path::PathBuf> {
    known_folder(&FOLDERID_Programs).map(|dir| dir.join(SHORTCUT_NAME))
}

fn known_folder(id: &windows::core::GUID) -> Option<std::path::PathBuf> {
    unsafe {
        let raw = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let dir = raw.to_string().ok();
        CoTaskMemFree(Some(raw.0 as *const _));
        Some(std::path::PathBuf::from(dir?))
    }
}

/// 建一个指向核心的快捷方式，并把 AUMID 写进它的属性。
fn create_shortcut(path: &std::path::Path) -> windows::core::Result<()> {
    let exe = std::env::current_exe().map_err(|e| {
        windows::core::Error::new(windows::Win32::Foundation::E_FAIL, e.to_string())
    })?;
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(&HSTRING::from(exe.as_os_str()))?;
        if let Some(dir) = exe.parent() {
            link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
        }

        let store: IPropertyStore = link.cast()?;
        store.SetValue(&PKEY_AppUserModel_ID, &PROPVARIANT::from(AUMID))?;
        store.Commit()?;

        let file: IPersistFile = link.cast()?;
        let wide = HSTRING::from(path.as_os_str());
        file.Save(PCWSTR(wide.as_ptr()), true)?;
    }
    Ok(())
}

fn show_now(title: &str, body: &str) {
    if let Err(e) = try_show(title, body) {
        log_warn!("弹出通知失败: {}", crate::util::win_err(&e));
    }
}

fn try_show(title: &str, body: &str) -> windows::core::Result<()> {
    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(toast_xml(title, body)))?;
    let toast = ToastNotification::CreateToastNotification(&doc)?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))?.Show(&toast)
}

/// 两行文本的 ToastGeneric 模板。
fn toast_xml(title: &str, body: &str) -> String {
    format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        escape_xml(title),
        escape_xml(body)
    )
}

/// 文案是内置的，但仍不能把 `&` 之类直接塞进 XML。
fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 建一条不带 AUMID 的快捷方式；3.1.0 及更早的安装器留下的就是这种。
    fn create_bare_shortcut(
        path: &std::path::Path,
        target: &std::path::Path,
    ) -> windows::core::Result<()> {
        unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            link.SetPath(&HSTRING::from(target.as_os_str()))?;
            let file: IPersistFile = link.cast()?;
            let wide = HSTRING::from(path.as_os_str());
            file.Save(PCWSTR(wide.as_ptr()), true)
        }
    }

    /// 认领旧快捷方式：补上 AUMID，指向的 exe 原样不动。
    #[test]
    fn adopting_a_shortcut_adds_the_aumid_and_keeps_its_target() {
        let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let path = dir.path().join(SHORTCUT_NAME);
        let target = std::env::current_exe().expect("取当前可执行文件路径失败");

        create_bare_shortcut(&path, &target).expect("建裸快捷方式失败");
        assert!(!shortcut_has_aumid(&path), "裸快捷方式本就不该有 AUMID");
        assert!(
            same_path(&read_shortcut_target(&path).expect("应能读出目标"), &target),
            "裸快捷方式该指向刚写进去的 exe"
        );

        set_shortcut_aumid(&path).expect("补写 AUMID 失败");
        assert!(shortcut_has_aumid(&path), "补写后应认得出本程序的 AUMID");
        assert!(
            same_path(&read_shortcut_target(&path).expect("应能读出目标"), &target),
            "补写 AUMID 不该动到快捷方式指向的 exe"
        );

        if com.is_ok() {
            unsafe { CoUninitialize() };
        }
    }

    /// 只认「指向本 exe 且没有 AUMID」的那条：已注册的不必再动，
    /// 指向别处的（另一份安装、另一个便携目录）更不该被篡改。
    #[test]
    fn only_an_unregistered_shortcut_to_our_own_exe_is_adoptable() {
        let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let exe = std::env::current_exe().expect("取当前可执行文件路径失败");

        let bare = dir.path().join("bare.lnk");
        create_bare_shortcut(&bare, &exe).expect("建裸快捷方式失败");
        assert!(is_adoptable(&bare, &exe), "旧安装器留下的那种应当认领");

        let elsewhere = dir.path().join("elsewhere.lnk");
        create_bare_shortcut(&elsewhere, std::path::Path::new("C:\\Windows\\notepad.exe"))
            .expect("建裸快捷方式失败");
        assert!(
            !is_adoptable(&elsewhere, &exe),
            "指向别处的快捷方式不该被认领"
        );

        let registered = dir.path().join(SHORTCUT_NAME);
        create_bare_shortcut(&registered, &exe).expect("建裸快捷方式失败");
        set_shortcut_aumid(&registered).expect("补写 AUMID 失败");
        assert!(
            !is_adoptable(&registered, &exe),
            "已经带 AUMID 的不必再认领一次"
        );

        assert!(
            !is_adoptable(&dir.path().join("missing.lnk"), &exe),
            "不存在的路径不该被认领"
        );

        if com.is_ok() {
            unsafe { CoUninitialize() };
        }
    }

    #[test]
    fn paths_compare_case_insensitively() {
        assert!(same_path(
            std::path::Path::new("C:\\ZoneDeck\\ZoneDeck.exe"),
            std::path::Path::new("c:\\zonedeck\\zonedeck.EXE")
        ));
        assert!(!same_path(
            std::path::Path::new("C:\\ZoneDeck\\ZoneDeck.exe"),
            std::path::Path::new("D:\\ZoneDeck\\ZoneDeck.exe")
        ));
    }

    #[test]
    fn xml_special_characters_are_escaped() {
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("<x>"), "&lt;x&gt;");
        assert_eq!(escape_xml("中文原样"), "中文原样");
    }

    #[test]
    fn toast_xml_embeds_both_lines() {
        let xml = toast_xml("标题", "正文");
        assert!(xml.contains("<text>标题</text>"), "{xml}");
        assert!(xml.contains("<text>正文</text>"), "{xml}");
        assert!(
            xml.starts_with("<toast>") && xml.ends_with("</toast>"),
            "{xml}"
        );
    }

    /// 注入尝试不得撑破 XML 结构。
    #[test]
    fn a_title_that_looks_like_markup_stays_text() {
        let xml = toast_xml("</text><audio silent=\"true\"/><text>", "正文");
        assert!(!xml.contains("<audio"), "标记必须被转义: {xml}");
        assert_eq!(xml.matches("<text>").count(), 2, "只该有两段文本: {xml}");
    }
}
