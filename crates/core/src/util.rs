pub fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 以当前语言的文案追加一个菜单项。
///
/// # Safety
/// `menu` 必须是有效的菜单句柄。
pub unsafe fn append_menu_item(
    menu: windows::Win32::UI::WindowsAndMessaging::HMENU,
    flags: windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS,
    id: usize,
    msg: crate::i18n::Msg,
) {
    let text = to_wide_null(crate::i18n::t(msg));
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::AppendMenuW(
            menu,
            flags,
            id,
            windows::core::PCWSTR(text.as_ptr()),
        );
    }
}

/// 在光标处弹出一个右键菜单，返回是否成功弹出。
///
/// `build` 负责往菜单里塞条目（用 [`append_menu_item`]），菜单的创建、定位与销毁
/// 由本函数统一处理。选中项通过 `WM_COMMAND` 投递给 `hwnd`。
/// 弹出前须 `SetForegroundWindow`。
pub fn show_popup_menu(
    hwnd: windows::Win32::Foundation::HWND,
    flags: windows::Win32::UI::WindowsAndMessaging::TRACK_POPUP_MENU_FLAGS,
    build: impl FnOnce(windows::Win32::UI::WindowsAndMessaging::HMENU),
) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow, TrackPopupMenu,
    };

    unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return false;
        };
        build(menu);

        let mut pt = windows::Win32::Foundation::POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let shown = TrackPopupMenu(menu, flags, pt.x, pt.y, None, hwnd, None).as_bool();
        let _ = DestroyMenu(menu);
        shown
    }
}

/// 把 Win32/COM 错误格式化为「系统消息 (0x错误码)」。
pub fn win_err(e: &windows::core::Error) -> String {
    let message = e.message();
    let message = message.trim();
    let code = e.code().0 as u32;
    if message.is_empty() {
        format!("(0x{code:08X})")
    } else {
        format!("{message} (0x{code:08X})")
    }
}

/// 取字符串开头至多 `max` 个字符，其余以「…（共 N 字符）」代替，按字符切。
pub fn head_chars(text: &str, max: usize) -> String {
    let total = text.chars().count();
    if total <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max).collect();
    format!("{head}…（共 {total} 字符）")
}

/// 把系统 ANSI 代码页编码的字节解码成 `String`，用于读取只输出本地代码页文本的
/// 旧式控制台程序。空输入返回空串；解码失败退回 UTF-8 有损解码。
pub fn from_ansi(bytes: &[u8]) -> String {
    use windows::Win32::Globalization::{CP_ACP, MB_ERR_INVALID_CHARS, MultiByteToWideChar};

    if bytes.is_empty() {
        return String::new();
    }
    unsafe {
        let len = MultiByteToWideChar(CP_ACP, MB_ERR_INVALID_CHARS, bytes, None);
        if len <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        let mut buf = vec![0u16; len as usize];
        let written = MultiByteToWideChar(CP_ACP, MB_ERR_INVALID_CHARS, bytes, Some(&mut buf));
        if written <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        String::from_utf16_lossy(&buf[..written as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_null_terminator() {
        let w = to_wide_null("AB");
        assert_eq!(w, vec![0x41, 0x42, 0x00]);
    }

    #[test]
    fn empty_string_is_single_null() {
        assert_eq!(to_wide_null(""), vec![0x00]);
    }

    #[test]
    fn win_err_keeps_both_message_and_code() {
        // 消息文本随系统语言变化，故只断言码值与「消息非空」。
        let e = windows::core::Error::from_hresult(windows::core::HRESULT(0x8007_0005u32 as i32));
        let text = win_err(&e);
        assert!(text.ends_with("(0x80070005)"), "应带十六进制错误码: {text}");
        assert!(!text.starts_with('('), "应同时带系统消息: {text}");
    }

    #[test]
    fn head_chars_limits_length_without_breaking_characters() {
        assert_eq!(head_chars("短内容", 10), "短内容", "不超长时原样返回");
        assert_eq!(
            head_chars("中文中文中文", 3),
            "中文中…（共 6 字符）",
            "应按字符切并注明原长"
        );
        assert_eq!(head_chars("", 5), "");
    }

    #[test]
    fn win_err_without_message_still_reports_code() {
        // 自定义 HRESULT 通常没有对应的系统消息文本。
        let e = windows::core::Error::from_hresult(windows::core::HRESULT(0xDEAD_BEEFu32 as i32));
        assert!(win_err(&e).contains("0xDEADBEEF"));
    }

    #[test]
    fn from_ansi_decodes_ascii_and_empty() {
        assert_eq!(from_ansi(b""), "");
        assert_eq!(
            from_ansi(b"Unable to suspend process 42:"),
            "Unable to suspend process 42:"
        );
    }
}
