//! 核心的用户可见文案（托盘菜单、系统通知、IPC 错误信息）；日志不走此模块。

use std::sync::RwLock;

use windows::Win32::Globalization::GetUserDefaultLocaleName;
use zonedeck_common::i18n::{Lang, resolve};

/// 用户可见文案的键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    MenuSettings,
    MenuShowWindows,
    MenuHideWindows,
    MenuRestoreTool,
    MenuAutoHide,
    MenuAutostart,
    MenuAbout,
    MenuQuit,
    // 托盘气泡文案一律成对：`*Title` 是状态短语，`*Body` 是补充详情。
    // 正文不得为空，`szInfo` 为空时 Shell_NotifyIcon 不弹气泡。
    HiddenTitle,
    HiddenBody,
    ShownTitle,
    ShownBody,
    ConfigExeMissingTitle,
    ConfigExeMissingBody,
    RecoveryPersistFailedTitle,
    RecoveryPersistFailedBody,
    AutostartOffTitle,
    AutostartOffBody,
    AutostartOnTitle,
    AutostartOnTaskAdmin,
    AutostartOnTaskUser,
    AutostartOnRegistry,
    AutostartFailTitle,
    AutostartFailBody,
    StartTitle,
    StartBody,
    QuitTitle,
    QuitBody,
    LegacyCoreRunningTitle,
    LegacyCoreRunningBody,
    ErrReloadConfig,
    ErrCoreExited,
    ErrNotifyCore,
    ErrCoreTimeout,
    ErrCoreExeMissing,
    ErrFreezePartial,
    ErrResumePartial,
    ErrUrlSchemeNotAllowed,
    ErrFeedbackEmpty,
    ErrFeedbackContactRequired,
    ErrKeyCaptureFailed,
}

impl Msg {
    /// 该文案在指定语言下的内容。
    pub const fn text(self, lang: Lang) -> &'static str {
        match lang {
            Lang::ZhCn => self.zh_cn(),
            Lang::En => self.en(),
            Lang::ZhTw => self.zh_tw(),
        }
    }

    const fn zh_cn(self) -> &'static str {
        match self {
            Msg::MenuSettings => "设置",
            Msg::MenuShowWindows => "显示窗口",
            Msg::MenuHideWindows => "隐藏窗口",
            Msg::MenuRestoreTool => "窗口恢复工具",
            Msg::MenuAutoHide => "自动隐藏",
            Msg::MenuAutostart => "开机自启",
            Msg::MenuAbout => "关于",
            Msg::MenuQuit => "退出",
            Msg::HiddenTitle => "已隐藏窗口",
            Msg::HiddenBody => "按隐藏 / 显示热键即可恢复",
            Msg::ShownTitle => "已恢复显示窗口",
            Msg::ShownBody => "隐藏的窗口已重新出现在桌面上",
            Msg::ConfigExeMissingTitle => "未找到配置程序",
            Msg::ConfigExeMissingBody => "请确认配置程序与核心位于同一目录，或重新安装",
            Msg::RecoveryPersistFailedTitle => "无法写入崩溃恢复文件",
            Msg::RecoveryPersistFailedBody => "核心若异常退出，隐藏的窗口将无法自动找回",
            Msg::AutostartOffTitle => "开机自启已关闭",
            Msg::AutostartOffBody => "核心将不再随系统启动",
            Msg::AutostartOnTitle => "开机自启已开启",
            Msg::AutostartOnTaskAdmin => "已注册计划任务（管理员权限）",
            Msg::AutostartOnTaskUser => "已注册计划任务（普通权限）",
            Msg::AutostartOnRegistry => "已写入注册表启动项",
            Msg::AutostartFailTitle => "开机自启设置失败",
            Msg::AutostartFailBody => "计划任务与注册表方式均失败",
            Msg::StartTitle => "核心已启动",
            Msg::StartBody => "热键与鼠标触发已开始监听",
            Msg::QuitTitle => "核心已退出",
            Msg::QuitBody => "热键监控已停止，隐藏的窗口已恢复显示",
            Msg::LegacyCoreRunningTitle => "检测到旧版本正在运行",
            Msg::LegacyCoreRunningBody => {
                "旧版本核心（Boss Key）仍在运行。请先退出旧版本（托盘图标，或任务管理器中的 Boss Key.exe），再启动 ZoneDeck。"
            }
            Msg::ErrReloadConfig => "重载配置失败：{err}",
            Msg::ErrCoreExited => "核心已退出",
            Msg::ErrNotifyCore => "无法通知核心",
            Msg::ErrCoreTimeout => "核心响应超时",
            Msg::ErrCoreExeMissing => {
                "未找到核心程序 {exe}。它很可能被杀毒软件拦截或隔离了：请尝试将 ZoneDeck 的程序目录加入杀毒软件的白名单 / 信任区，再从隔离区恢复该文件；若无法恢复，请重新下载完整程序包。"
            }
            Msg::ErrFreezePartial => "{failed}/{total} 个进程冻结失败",
            Msg::ErrResumePartial => "{failed}/{total} 个进程解冻失败",
            Msg::ErrUrlSchemeNotAllowed => "只允许打开 http/https/mailto 链接",
            Msg::ErrFeedbackEmpty => "请先填写反馈内容",
            Msg::ErrFeedbackContactRequired => "转换为 Issue 需要留下 GitHub 账号",
            Msg::ErrKeyCaptureFailed => "无法独占键盘，录制期间的按键可能触发其他程序",
        }
    }

    const fn en(self) -> &'static str {
        match self {
            Msg::MenuSettings => "Settings",
            Msg::MenuShowWindows => "Show Windows",
            Msg::MenuHideWindows => "Hide Windows",
            Msg::MenuRestoreTool => "Window Recovery Tool",
            Msg::MenuAutoHide => "Auto Hide",
            Msg::MenuAutostart => "Start with Windows",
            Msg::MenuAbout => "About",
            Msg::MenuQuit => "Exit",
            Msg::HiddenTitle => "Windows hidden",
            Msg::HiddenBody => "Press the hide / show hotkey to bring them back",
            Msg::ShownTitle => "Windows restored",
            Msg::ShownBody => "The hidden windows are back on your desktop",
            Msg::ConfigExeMissingTitle => "Settings app not found",
            Msg::ConfigExeMissingBody => {
                "Make sure the settings app sits next to the core, or reinstall"
            }
            Msg::RecoveryPersistFailedTitle => "Cannot write the crash-recovery file",
            Msg::RecoveryPersistFailedBody => {
                "If the core exits abnormally, hidden windows cannot be restored automatically"
            }
            Msg::AutostartOffTitle => "Startup disabled",
            Msg::AutostartOffBody => "The core will no longer start with Windows",
            Msg::AutostartOnTitle => "Startup enabled",
            Msg::AutostartOnTaskAdmin => "Scheduled task registered (administrator)",
            Msg::AutostartOnTaskUser => "Scheduled task registered (standard user)",
            Msg::AutostartOnRegistry => "Registry startup entry written",
            Msg::AutostartFailTitle => "Could not configure startup",
            Msg::AutostartFailBody => "Both the scheduled task and the registry method failed",
            Msg::StartTitle => "The core is running",
            Msg::StartBody => "Hotkey and mouse triggers are now being watched",
            Msg::QuitTitle => "The core has exited",
            Msg::QuitBody => "Hotkey monitoring stopped; hidden windows have been restored",
            Msg::LegacyCoreRunningTitle => "An old version is still running",
            Msg::LegacyCoreRunningBody => {
                "The previous Boss Key core is still running. Quit it first (via its tray icon, or Boss Key.exe in Task Manager), then start ZoneDeck."
            }
            Msg::ErrReloadConfig => "Failed to reload configuration: {err}",
            Msg::ErrCoreExited => "The core has exited",
            Msg::ErrNotifyCore => "Cannot reach the core",
            Msg::ErrCoreTimeout => "The core did not respond in time",
            Msg::ErrCoreExeMissing => {
                "Core program {exe} not found. It was most likely blocked or quarantined by antivirus software: add the ZoneDeck program folder to your antivirus allowlist, then restore the file from quarantine; if that is not possible, download the full package again."
            }
            Msg::ErrFreezePartial => "Failed to freeze {failed} of {total} processes",
            Msg::ErrResumePartial => "Failed to resume {failed} of {total} processes",
            Msg::ErrUrlSchemeNotAllowed => "Only http/https/mailto links may be opened",
            Msg::ErrFeedbackEmpty => "Please write your feedback first",
            Msg::ErrFeedbackContactRequired => {
                "Converting feedback into an issue requires a GitHub account"
            }
            Msg::ErrKeyCaptureFailed => {
                "Cannot capture the keyboard exclusively; keys pressed while recording may trigger other programs"
            }
        }
    }

    const fn zh_tw(self) -> &'static str {
        match self {
            Msg::MenuSettings => "設定",
            Msg::MenuShowWindows => "顯示視窗",
            Msg::MenuHideWindows => "隱藏視窗",
            Msg::MenuRestoreTool => "視窗復原工具",
            Msg::MenuAutoHide => "自動隱藏",
            Msg::MenuAutostart => "開機自動啟動",
            Msg::MenuAbout => "關於",
            Msg::MenuQuit => "結束",
            Msg::HiddenTitle => "已隱藏視窗",
            Msg::HiddenBody => "按隱藏／顯示快速鍵即可復原",
            Msg::ShownTitle => "已復原顯示視窗",
            Msg::ShownBody => "隱藏的視窗已重新出現在桌面上",
            Msg::ConfigExeMissingTitle => "找不到設定程式",
            Msg::ConfigExeMissingBody => "請確認設定程式與核心位於同一資料夾，或重新安裝",
            Msg::RecoveryPersistFailedTitle => "無法寫入當機復原檔案",
            Msg::RecoveryPersistFailedBody => "核心若異常結束，隱藏的視窗將無法自動找回",
            Msg::AutostartOffTitle => "已關閉開機自動啟動",
            Msg::AutostartOffBody => "核心將不再隨系統啟動",
            Msg::AutostartOnTitle => "已開啟開機自動啟動",
            Msg::AutostartOnTaskAdmin => "已註冊排程工作（系統管理員權限）",
            Msg::AutostartOnTaskUser => "已註冊排程工作（一般權限）",
            Msg::AutostartOnRegistry => "已寫入登錄檔啟動項目",
            Msg::AutostartFailTitle => "開機自動啟動設定失敗",
            Msg::AutostartFailBody => "排程工作與登錄檔方式均失敗",
            Msg::StartTitle => "核心已啟動",
            Msg::StartBody => "快速鍵與滑鼠觸發已開始監聽",
            Msg::QuitTitle => "核心已結束",
            Msg::QuitBody => "熱鍵監控已停止，隱藏的視窗已復原顯示",
            Msg::LegacyCoreRunningTitle => "偵測到舊版本正在執行",
            Msg::LegacyCoreRunningBody => {
                "舊版本核心（Boss Key）仍在執行。請先結束舊版本（通知區域圖示，或工作管理員中的 Boss Key.exe），再啟動 ZoneDeck。"
            }
            Msg::ErrReloadConfig => "重新載入設定失敗：{err}",
            Msg::ErrCoreExited => "核心已結束",
            Msg::ErrNotifyCore => "無法通知核心",
            Msg::ErrCoreTimeout => "核心回應逾時",
            Msg::ErrCoreExeMissing => {
                "找不到核心程式 {exe}。它很可能被防毒軟體攔截或隔離了：請將 ZoneDeck 的程式資料夾加入防毒軟體的信任區／白名單，再從隔離區還原該檔案；若無法還原，請重新下載完整程式包。"
            }
            Msg::ErrFreezePartial => "{failed}/{total} 個程序凍結失敗",
            Msg::ErrResumePartial => "{failed}/{total} 個程序解除凍結失敗",
            Msg::ErrUrlSchemeNotAllowed => "僅允許開啟 http/https/mailto 連結",
            Msg::ErrFeedbackEmpty => "請先填寫意見回饋內容",
            Msg::ErrFeedbackContactRequired => "轉換為 Issue 需要留下 GitHub 帳號",
            Msg::ErrKeyCaptureFailed => "無法獨佔鍵盤，錄製期間的按鍵可能觸發其他程式",
        }
    }
}

static LANG: RwLock<Lang> = RwLock::new(Lang::ZhCn);

/// 系统 UI 语言的 BCP-47 标签（如 `zh-Hant-TW`）。
fn system_locale() -> Option<String> {
    let mut buf = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(&mut buf) };
    if len <= 0 {
        return None;
    }
    // 返回值含结尾的 NUL。
    let tag = String::from_utf16_lossy(&buf[..(len as usize).saturating_sub(1)]);
    (!tag.is_empty()).then_some(tag)
}

/// 按配置里的语言偏好设定当前语言；`pref` 为 `auto` 时跟随系统。
pub fn set_from_pref(pref: &str) {
    let lang = resolve(pref, system_locale().as_deref());
    if let Ok(mut guard) = LANG.write() {
        *guard = lang;
    }
}

pub fn lang() -> Lang {
    LANG.read().map(|g| *g).unwrap_or_default()
}

/// 取当前语言下的文案。
pub fn t(msg: Msg) -> &'static str {
    msg.text(lang())
}

/// 取当前语言下的文案，并把 `{名字}` 占位符替换为 `params` 中的同名值；
/// 未提供的占位符原样保留。
pub fn tf(msg: Msg, params: &[(&str, &str)]) -> String {
    let mut text = t(msg).to_string();
    for (name, value) in params {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::*;

    /// `LANG` 是进程级全局状态，改动它的测试须串行。
    static LANG_LOCK: Mutex<()> = Mutex::new(());

    fn lock_lang() -> MutexGuard<'static, ()> {
        LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 全部文案键；新增 Msg 变体后必须同步登记。
    const ALL_MSGS: [Msg; 41] = [
        Msg::MenuSettings,
        Msg::MenuShowWindows,
        Msg::MenuHideWindows,
        Msg::MenuRestoreTool,
        Msg::MenuAutoHide,
        Msg::MenuAutostart,
        Msg::MenuAbout,
        Msg::MenuQuit,
        Msg::HiddenTitle,
        Msg::HiddenBody,
        Msg::ShownTitle,
        Msg::ShownBody,
        Msg::ConfigExeMissingTitle,
        Msg::ConfigExeMissingBody,
        Msg::RecoveryPersistFailedTitle,
        Msg::RecoveryPersistFailedBody,
        Msg::AutostartOffTitle,
        Msg::AutostartOffBody,
        Msg::AutostartOnTitle,
        Msg::AutostartOnTaskAdmin,
        Msg::AutostartOnTaskUser,
        Msg::AutostartOnRegistry,
        Msg::AutostartFailTitle,
        Msg::AutostartFailBody,
        Msg::StartTitle,
        Msg::StartBody,
        Msg::QuitTitle,
        Msg::QuitBody,
        Msg::LegacyCoreRunningTitle,
        Msg::LegacyCoreRunningBody,
        Msg::ErrReloadConfig,
        Msg::ErrCoreExited,
        Msg::ErrNotifyCore,
        Msg::ErrCoreTimeout,
        Msg::ErrCoreExeMissing,
        Msg::ErrFreezePartial,
        Msg::ErrResumePartial,
        Msg::ErrUrlSchemeNotAllowed,
        Msg::ErrFeedbackEmpty,
        Msg::ErrFeedbackContactRequired,
        Msg::ErrKeyCaptureFailed,
    ];

    /// 逐条校验三种语言均非空且互不相同。
    #[test]
    fn every_message_is_translated_in_all_languages() {
        for msg in ALL_MSGS {
            for lang in Lang::ALL {
                assert!(!msg.text(lang).trim().is_empty(), "{msg:?} / {lang:?} 为空");
            }
            assert_ne!(msg.text(Lang::ZhCn), msg.text(Lang::En), "{msg:?} 未译英文");
        }
    }

    /// 占位符跨语言必须一致。
    #[test]
    fn placeholders_match_across_languages() {
        fn holders(text: &str) -> Vec<&str> {
            let mut found: Vec<&str> = text
                .match_indices('{')
                .filter_map(|(i, _)| {
                    let rest = &text[i + 1..];
                    rest.find('}').map(|end| &rest[..end])
                })
                .collect();
            found.sort_unstable();
            found
        }
        for msg in ALL_MSGS {
            let expected = holders(msg.text(Lang::ZhCn));
            for lang in Lang::ALL {
                assert_eq!(holders(msg.text(lang)), expected, "{msg:?} / {lang:?}");
            }
        }
    }

    #[test]
    fn formats_placeholders() {
        let _guard = lock_lang();
        set_from_pref("zh-CN");
        assert_eq!(
            tf(Msg::ErrFreezePartial, &[("failed", "2"), ("total", "5")]),
            "2/5 个进程冻结失败"
        );
        assert_eq!(
            tf(Msg::ErrFreezePartial, &[]),
            "{failed}/{total} 个进程冻结失败"
        );
    }

    #[test]
    fn system_locale_is_a_parsable_tag_or_absent() {
        if let Some(tag) = system_locale() {
            assert!(!tag.contains('\0'), "标签不应含 NUL：{tag:?}");
            assert!(
                tag.starts_with(|c: char| c.is_ascii_alphabetic()),
                "{tag:?}"
            );
        }
    }

    #[test]
    fn explicit_pref_takes_effect() {
        let _guard = lock_lang();
        set_from_pref("en");
        assert_eq!(lang(), Lang::En);
        assert_eq!(t(Msg::MenuQuit), "Exit");
        set_from_pref("zh-TW");
        assert_eq!(lang(), Lang::ZhTw);
        assert_eq!(t(Msg::MenuQuit), "結束");
        set_from_pref("zh-CN");
        assert_eq!(lang(), Lang::ZhCn);
        assert_eq!(t(Msg::MenuQuit), "退出");
    }
}
