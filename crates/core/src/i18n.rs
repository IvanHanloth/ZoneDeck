//! 核心的用户可见文案（托盘菜单、气泡通知、IPC 错误信息）。
//!
//! 日志不走此模块，一律使用中文。

use std::sync::RwLock;

use bosskey_common::i18n::{Lang, resolve};
use windows::Win32::Globalization::GetUserDefaultLocaleName;

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
    HiddenBody,
    ShownBody,
    ConfigExeMissing,
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
    ErrReloadConfig,
    ErrCoreExited,
    ErrNotifyCore,
    ErrCoreTimeout,
    ErrCoreExeMissing,
    ErrFreezePartial,
    ErrResumePartial,
    ErrUrlSchemeNotAllowed,
    ErrFeedbackEmpty,
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
            Msg::HiddenBody => "已隐藏窗口",
            Msg::ShownBody => "已恢复显示窗口",
            Msg::ConfigExeMissing => "未找到配置程序",
            Msg::RecoveryPersistFailedBody => "无法写入崩溃恢复文件，异常退出后将无法自动找回窗口",
            Msg::AutostartOffTitle => "开机自启已关闭",
            Msg::AutostartOffBody => "Boss Key 将不再随系统启动",
            Msg::AutostartOnTitle => "开机自启已开启",
            Msg::AutostartOnTaskAdmin => "已注册计划任务（管理员权限）",
            Msg::AutostartOnTaskUser => "已注册计划任务（普通权限）",
            Msg::AutostartOnRegistry => "已写入注册表启动项",
            Msg::AutostartFailTitle => "开机自启设置失败",
            Msg::AutostartFailBody => "计划任务与注册表方式均失败",
            Msg::StartTitle => "Boss Key 正在运行！",
            Msg::StartBody => "Boss Key 正在为您服务，您可通过托盘图标看到我",
            Msg::QuitTitle => "Boss Key 已停止服务",
            Msg::QuitBody => "Boss Key 已成功退出",
            Msg::ErrReloadConfig => "重载配置失败：{err}",
            Msg::ErrCoreExited => "核心已退出",
            Msg::ErrNotifyCore => "无法通知核心",
            Msg::ErrCoreTimeout => "核心响应超时",
            Msg::ErrCoreExeMissing => "未找到核心程序 {exe}。它很可能被杀毒软件拦截或隔离了：请尝试将 Boss Key 的程序目录加入杀毒软件的白名单 / 信任区，再从隔离区恢复该文件；若无法恢复，请重新下载完整程序包。",
            Msg::ErrFreezePartial => "{failed}/{total} 个进程冻结失败",
            Msg::ErrResumePartial => "{failed}/{total} 个进程解冻失败",
            Msg::ErrUrlSchemeNotAllowed => "只允许打开 http/https/mailto 链接",
            Msg::ErrFeedbackEmpty => "请先填写反馈内容",
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
            Msg::HiddenBody => "Windows hidden",
            Msg::ShownBody => "Windows restored",
            Msg::ConfigExeMissing => "Settings app not found",
            Msg::RecoveryPersistFailedBody => "Cannot write the crash-recovery file; windows cannot be restored automatically after an abnormal exit",
            Msg::AutostartOffTitle => "Startup disabled",
            Msg::AutostartOffBody => "Boss Key will no longer start with Windows",
            Msg::AutostartOnTitle => "Startup enabled",
            Msg::AutostartOnTaskAdmin => "Scheduled task registered (administrator)",
            Msg::AutostartOnTaskUser => "Scheduled task registered (standard user)",
            Msg::AutostartOnRegistry => "Registry startup entry written",
            Msg::AutostartFailTitle => "Could not configure startup",
            Msg::AutostartFailBody => "Both the scheduled task and the registry method failed",
            Msg::StartTitle => "Boss Key is running",
            Msg::StartBody => "Boss Key is active — find it in the notification area",
            Msg::QuitTitle => "Boss Key has stopped",
            Msg::QuitBody => "Boss Key exited successfully",
            Msg::ErrReloadConfig => "Failed to reload configuration: {err}",
            Msg::ErrCoreExited => "The core has exited",
            Msg::ErrNotifyCore => "Cannot reach the core",
            Msg::ErrCoreTimeout => "The core did not respond in time",
            Msg::ErrCoreExeMissing => "Core program {exe} not found. It was most likely blocked or quarantined by antivirus software: add the Boss Key program folder to your antivirus allowlist, then restore the file from quarantine; if that is not possible, download the full package again.",
            Msg::ErrFreezePartial => "Failed to freeze {failed} of {total} processes",
            Msg::ErrResumePartial => "Failed to resume {failed} of {total} processes",
            Msg::ErrUrlSchemeNotAllowed => "Only http/https/mailto links may be opened",
            Msg::ErrFeedbackEmpty => "Please write your feedback first",
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
            Msg::HiddenBody => "已隱藏視窗",
            Msg::ShownBody => "已復原顯示視窗",
            Msg::ConfigExeMissing => "找不到設定程式",
            Msg::RecoveryPersistFailedBody => "無法寫入當機復原檔案，異常結束後將無法自動找回視窗",
            Msg::AutostartOffTitle => "已關閉開機自動啟動",
            Msg::AutostartOffBody => "Boss Key 將不再隨系統啟動",
            Msg::AutostartOnTitle => "已開啟開機自動啟動",
            Msg::AutostartOnTaskAdmin => "已註冊排程工作（系統管理員權限）",
            Msg::AutostartOnTaskUser => "已註冊排程工作（一般權限）",
            Msg::AutostartOnRegistry => "已寫入登錄檔啟動項目",
            Msg::AutostartFailTitle => "開機自動啟動設定失敗",
            Msg::AutostartFailBody => "排程工作與登錄檔方式均失敗",
            Msg::StartTitle => "Boss Key 正在執行！",
            Msg::StartBody => "Boss Key 正在為您服務，您可透過通知區域圖示找到我",
            Msg::QuitTitle => "Boss Key 已停止服務",
            Msg::QuitBody => "Boss Key 已成功結束",
            Msg::ErrReloadConfig => "重新載入設定失敗：{err}",
            Msg::ErrCoreExited => "核心已結束",
            Msg::ErrNotifyCore => "無法通知核心",
            Msg::ErrCoreTimeout => "核心回應逾時",
            Msg::ErrCoreExeMissing => "找不到核心程式 {exe}。它很可能被防毒軟體攔截或隔離了：請將 Boss Key 的程式資料夾加入防毒軟體的信任區／白名單，再從隔離區還原該檔案；若無法還原，請重新下載完整程式包。",
            Msg::ErrFreezePartial => "{failed}/{total} 個程序凍結失敗",
            Msg::ErrResumePartial => "{failed}/{total} 個程序解除凍結失敗",
            Msg::ErrUrlSchemeNotAllowed => "僅允許開啟 http/https/mailto 連結",
            Msg::ErrFeedbackEmpty => "請先填寫意見回饋內容",
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
    // 返回值含结尾的 NUL，截断后再转字符串。
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

/// 当前生效语言。
pub fn lang() -> Lang {
    LANG.read().map(|g| *g).unwrap_or_default()
}

/// 取当前语言下的文案。
pub fn t(msg: Msg) -> &'static str {
    msg.text(lang())
}

/// 取当前语言下的文案，并把 `{名字}` 占位符替换为 `params` 中的同名值。
///
/// 与前端 `t(key, params)` 同构；未提供的占位符原样保留。
pub fn tf(msg: Msg, params: &[(&str, &str)]) -> String {
    let mut text = t(msg).to_string();
    for (name, value) in params {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 全部文案键；新增 Msg 变体后必须同步登记，否则跨语言校验会漏掉它。
    const ALL_MSGS: [Msg; 33] = [
        Msg::MenuSettings,
        Msg::MenuShowWindows,
        Msg::MenuHideWindows,
        Msg::MenuRestoreTool,
        Msg::MenuAutoHide,
        Msg::MenuAutostart,
        Msg::MenuAbout,
        Msg::MenuQuit,
        Msg::HiddenBody,
        Msg::ShownBody,
        Msg::ConfigExeMissing,
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
        Msg::ErrReloadConfig,
        Msg::ErrCoreExited,
        Msg::ErrNotifyCore,
        Msg::ErrCoreTimeout,
        Msg::ErrCoreExeMissing,
        Msg::ErrFreezePartial,
        Msg::ErrResumePartial,
        Msg::ErrUrlSchemeNotAllowed,
        Msg::ErrFeedbackEmpty,
    ];

    /// 任一语言缺翻译都会退化成中英混排，故逐条校验三种语言均非空且互不相同。
    #[test]
    fn every_message_is_translated_in_all_languages() {
        for msg in ALL_MSGS {
            for lang in Lang::ALL {
                assert!(!msg.text(lang).trim().is_empty(), "{msg:?} / {lang:?} 为空");
            }
            assert_ne!(msg.text(Lang::ZhCn), msg.text(Lang::En), "{msg:?} 未译英文");
        }
    }

    /// 占位符跨语言必须一致，否则换语言后参数会静默丢失。
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
        set_from_pref("zh-CN");
        assert_eq!(
            tf(Msg::ErrFreezePartial, &[("failed", "2"), ("total", "5")]),
            "2/5 个进程冻结失败"
        );
        // 未提供的占位符原样保留，便于发现漏传参数。
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
