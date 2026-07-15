pub fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
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
    fn from_ansi_decodes_ascii_and_empty() {
        assert_eq!(from_ansi(b""), "");
        assert_eq!(from_ansi(b"Unable to suspend process 42:"), "Unable to suspend process 42:");
    }
}
