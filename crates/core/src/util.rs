pub fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 以当前语言的文案追加一个菜单项。
///
/// 菜单文案在运行时才确定，无法用编译期的 `w!`；`AppendMenuW` 会复制字符串，
/// 因此临时缓冲区只需活到调用结束。
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

/// 当前按下的修饰键位掩码（[`crate::hotkey::MOD_CONTROL`] 等的组合）。
/// 键盘 / 鼠标底层钩子共用，用来判定触发条件里的修饰键是否吻合。
pub fn pressed_modifiers() -> u32 {
    use crate::hotkey::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };

    let down = |vk: u16| unsafe { (GetAsyncKeyState(i32::from(vk)) as u16 & 0x8000) != 0 };
    let mut m = 0;
    if down(VK_CONTROL.0) {
        m |= MOD_CONTROL;
    }
    if down(VK_MENU.0) {
        m |= MOD_ALT;
    }
    if down(VK_SHIFT.0) {
        m |= MOD_SHIFT;
    }
    if down(VK_LWIN.0) || down(VK_RWIN.0) {
        m |= MOD_WIN;
    }
    m
}

/// 把 Win32/COM 错误格式化为「系统消息 (0x错误码)」。
/// 只有消息文本时无法区分「拒绝访问」的来源，只有码值又难以判读，故两者都写出。
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

/// 把系统 ANSI 代码页（CP_ACP，中文系统为 GBK/936）编码的字节解码成 `String`。
/// 用于读取只输出本地代码页文本的旧式控制台程序（如 pssuspend），避免直接按
/// UTF-8 解码得到乱码。空输入返回空串；解码失败退回 UTF-8 有损解码。
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
        // 0x80070005 = HRESULT_FROM_WIN32(ERROR_ACCESS_DENIED)。消息文本随系统语言变化，
        // 故只断言码值与「消息非空」，不断言具体文案。
        let e = windows::core::Error::from_hresult(windows::core::HRESULT(0x8007_0005u32 as i32));
        let text = win_err(&e);
        assert!(text.ends_with("(0x80070005)"), "应带十六进制错误码: {text}");
        assert!(!text.starts_with('('), "应同时带系统消息: {text}");
    }

    #[test]
    fn win_err_without_message_still_reports_code() {
        // 自定义 HRESULT 通常没有对应的系统消息文本，此时不应只剩空串。
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
