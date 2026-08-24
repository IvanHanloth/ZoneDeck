pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_WIN: u32 = 0x0008;
pub const MOD_NOREPEAT: u32 = 0x4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedHotkey {
    pub modifiers: u32,
    pub vk: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HotkeyParseError {
    #[error("热键为空")]
    Empty,
    #[error("热键缺少主键（仅有修饰键）")]
    NoKey,
    #[error("无法识别的按键: {0}")]
    UnknownKey(String),
    #[error("热键包含多个主键")]
    MultipleKeys,
}

fn modifier_bit(token: &str) -> Option<u32> {
    match token.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some(MOD_CONTROL),
        "alt" => Some(MOD_ALT),
        "shift" => Some(MOD_SHIFT),
        "win" | "windows" | "cmd" | "super" | "meta" => Some(MOD_WIN),
        _ => None,
    }
}

fn key_to_vk(token: &str) -> Option<u16> {
    let upper = token.to_ascii_uppercase();

    if upper.len() == 1 {
        let c = upper.as_bytes()[0];
        if c.is_ascii_uppercase() || c.is_ascii_digit() {
            return Some(c as u16);
        }
    }

    if let Some(rest) = upper.strip_prefix('F')
        && let Ok(n) = rest.parse::<u16>()
        && (1..=24).contains(&n)
    {
        return Some(0x70 + (n - 1));
    }

    let norm = upper.replace([' ', '_'], "");
    let vk: u16 = match norm.as_str() {
        "ESC" | "ESCAPE" => 0x1B,
        "SPACE" => 0x20,
        "ENTER" | "RETURN" => 0x0D,
        "TAB" => 0x09,
        "BACKSPACE" | "BACK" => 0x08,
        "DELETE" | "DEL" => 0x2E,
        "INSERT" | "INS" => 0x2D,
        "HOME" => 0x24,
        "END" => 0x23,
        "PAGEUP" | "PRIOR" => 0x21,
        "PAGEDOWN" | "NEXT" => 0x22,
        "UP" => 0x26,
        "DOWN" => 0x28,
        "LEFT" => 0x25,
        "RIGHT" => 0x27,
        "CAPSLOCK" => 0x14,
        "NUMLOCK" => 0x90,
        "SCROLLLOCK" => 0x91,
        "PRINTSCREEN" | "PRTSC" | "SNAPSHOT" => 0x2C,
        "PAUSE" => 0x13,
        "CLEAR" => 0x0C,
        _ => return None,
    };
    Some(vk)
}

/// [`key_to_vk`] 的逆向：虚拟键码 → 热键字符串里的按键名。表外的键返回 None。
pub fn vk_to_key(vk: u16) -> Option<String> {
    if (0x30..=0x39).contains(&vk) || (0x41..=0x5A).contains(&vk) {
        return Some((vk as u8 as char).to_string());
    }
    if (0x70..=0x87).contains(&vk) {
        return Some(format!("F{}", vk - 0x70 + 1));
    }
    let name = match vk {
        0x1B => "Esc",
        0x20 => "Space",
        0x0D => "Enter",
        0x09 => "Tab",
        0x08 => "Backspace",
        0x2E => "Delete",
        0x2D => "Insert",
        0x24 => "Home",
        0x23 => "End",
        0x21 => "PageUp",
        0x22 => "PageDown",
        0x26 => "Up",
        0x28 => "Down",
        0x25 => "Left",
        0x27 => "Right",
        0x14 => "CapsLock",
        0x90 => "NumLock",
        0x91 => "ScrollLock",
        0x2C => "PrintScreen",
        0x13 => "Pause",
        0x0C => "Clear",
        _ => return None,
    };
    Some(name.to_string())
}

/// 修饰键位掩码 → `"Ctrl+Alt+Shift+Win"` 形式；顺序固定，无修饰键时为空串。
pub fn format_modifiers(modifiers: u32) -> String {
    let mut parts = Vec::new();
    if modifiers & MOD_CONTROL != 0 {
        parts.push("Ctrl");
    }
    if modifiers & MOD_ALT != 0 {
        parts.push("Alt");
    }
    if modifiers & MOD_SHIFT != 0 {
        parts.push("Shift");
    }
    if modifiers & MOD_WIN != 0 {
        parts.push("Win");
    }
    parts.join("+")
}

/// 组装热键字符串；`vk` 为 None 时只输出修饰键（录制过程中的中间态）。
/// 主键不在支持范围内时同样只输出修饰键。
pub fn format_hotkey(modifiers: u32, vk: Option<u16>) -> String {
    let mods = format_modifiers(modifiers);
    let Some(key) = vk.and_then(vk_to_key) else {
        return mods;
    };
    if mods.is_empty() {
        key
    } else {
        format!("{mods}+{key}")
    }
}

/// 热键是否置空；置空表示关闭该热键。
pub fn is_disabled(s: &str) -> bool {
    s.trim().is_empty()
}

/// 只解析修饰键组合（如 `"Ctrl+Shift"`）为位掩码，忽略无法识别的片段。
pub fn parse_modifiers(s: &str) -> u32 {
    s.split('+')
        .filter_map(|part| modifier_bit(part.trim()))
        .fold(0, |acc, bit| acc | bit)
}

pub fn parse_hotkey(s: &str) -> Result<ParsedHotkey, HotkeyParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(HotkeyParseError::Empty);
    }

    let mut modifiers = 0u32;
    let mut vk: Option<u16> = None;

    for part in s.split('+') {
        let token = part.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(bit) = modifier_bit(token) {
            modifiers |= bit;
            continue;
        }
        let key =
            key_to_vk(token).ok_or_else(|| HotkeyParseError::UnknownKey(token.to_string()))?;
        if vk.is_some() {
            return Err(HotkeyParseError::MultipleKeys);
        }
        vk = Some(key);
    }

    match vk {
        Some(vk) => Ok(ParsedHotkey { modifiers, vk }),
        None => Err(HotkeyParseError::NoKey),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_modifiers_reads_only_modifier_tokens() {
        assert_eq!(parse_modifiers(""), 0);
        assert_eq!(parse_modifiers("Ctrl"), MOD_CONTROL);
        assert_eq!(parse_modifiers("Ctrl+Shift"), MOD_CONTROL | MOD_SHIFT);
        assert_eq!(parse_modifiers("Win+Alt"), MOD_WIN | MOD_ALT);
        assert_eq!(parse_modifiers("Ctrl+Q"), MOD_CONTROL, "主键片段被忽略");
    }

    #[test]
    fn parses_default_hide_hotkey() {
        assert_eq!(
            parse_hotkey("Ctrl+Q"),
            Ok(ParsedHotkey {
                modifiers: MOD_CONTROL,
                vk: 0x51,
            })
        );
    }

    #[test]
    fn parses_default_close_hotkey() {
        assert_eq!(
            parse_hotkey("Win+Esc"),
            Ok(ParsedHotkey {
                modifiers: MOD_WIN,
                vk: 0x1B,
            })
        );
    }

    #[test]
    fn parses_multiple_modifiers_and_function_key() {
        assert_eq!(
            parse_hotkey("Ctrl+Shift+F1"),
            Ok(ParsedHotkey {
                modifiers: MOD_CONTROL | MOD_SHIFT,
                vk: 0x70,
            })
        );
    }

    #[test]
    fn f24_maps_to_correct_vk() {
        assert_eq!(parse_hotkey("F24").unwrap().vk, 0x87);
    }

    #[test]
    fn digits_and_letters_map_to_ascii_vk() {
        assert_eq!(parse_hotkey("Alt+5").unwrap().vk, 0x35);
        assert_eq!(parse_hotkey("Ctrl+A").unwrap().vk, 0x41);
    }

    #[test]
    fn is_case_insensitive_and_whitespace_tolerant() {
        let a = parse_hotkey("ctrl + shift + q").unwrap();
        let b = parse_hotkey("CTRL+SHIFT+Q").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.modifiers, MOD_CONTROL | MOD_SHIFT);
        assert_eq!(a.vk, 0x51);
    }

    #[test]
    fn accepts_alternate_named_keys() {
        assert_eq!(parse_hotkey("Ctrl+Page_Up").unwrap().vk, 0x21);
        assert_eq!(parse_hotkey("Ctrl+PageUp").unwrap().vk, 0x21);
        assert_eq!(parse_hotkey("Alt+Space").unwrap().vk, 0x20);
        assert_eq!(parse_hotkey("Shift+Print Screen").unwrap().vk, 0x2C);
    }

    #[test]
    fn blank_hotkey_counts_as_disabled() {
        assert!(is_disabled(""));
        assert!(is_disabled("   "));
        assert!(!is_disabled("Ctrl+Q"));
        assert!(!is_disabled("Ctrl+Shift"), "缺主键是错误配置，不算置空");
    }

    #[test]
    fn empty_is_rejected() {
        assert_eq!(parse_hotkey("   "), Err(HotkeyParseError::Empty));
    }

    #[test]
    fn only_modifiers_is_rejected() {
        assert_eq!(parse_hotkey("Ctrl+Shift"), Err(HotkeyParseError::NoKey));
    }

    #[test]
    fn unknown_key_is_rejected() {
        assert_eq!(
            parse_hotkey("Ctrl+Frobnicate"),
            Err(HotkeyParseError::UnknownKey("Frobnicate".to_string()))
        );
    }

    #[test]
    fn multiple_main_keys_is_rejected() {
        assert_eq!(
            parse_hotkey("Ctrl+Q+W"),
            Err(HotkeyParseError::MultipleKeys)
        );
    }

    #[test]
    fn format_modifiers_uses_the_same_order_as_the_recorder() {
        assert_eq!(format_modifiers(0), "");
        assert_eq!(format_modifiers(MOD_CONTROL), "Ctrl");
        assert_eq!(
            format_modifiers(MOD_WIN | MOD_SHIFT | MOD_ALT | MOD_CONTROL),
            "Ctrl+Alt+Shift+Win"
        );
    }

    #[test]
    fn format_hotkey_without_a_main_key_yields_modifiers_only() {
        assert_eq!(format_hotkey(MOD_CONTROL | MOD_SHIFT, None), "Ctrl+Shift");
        assert_eq!(format_hotkey(0, None), "");
        // 表外的主键当作还没按到主键。
        assert_eq!(format_hotkey(MOD_ALT, Some(0x60)), "Alt");
    }

    #[test]
    fn format_hotkey_without_modifiers_yields_the_bare_key() {
        assert_eq!(format_hotkey(0, Some(0x70)), "F1");
    }

    /// 录制出来的字符串必须能被 `parse_hotkey` 原样解回来。
    #[test]
    fn every_supported_vk_round_trips_through_parse_hotkey() {
        let modifiers = MOD_CONTROL | MOD_SHIFT;
        let mut count = 0;
        for vk in 0..=0xFFu16 {
            let Some(_) = vk_to_key(vk) else { continue };
            count += 1;
            let text = format_hotkey(modifiers, Some(vk));
            assert_eq!(
                parse_hotkey(&text),
                Ok(ParsedHotkey { modifiers, vk }),
                "{text} 解析结果与录制来源不符"
            );
        }
        // 26 字母 + 10 数字 + 24 功能键 + 21 命名键
        assert_eq!(count, 81, "支持的按键数量变了，确认改动是有意的");
    }

    #[test]
    fn vk_to_key_rejects_keys_outside_the_table() {
        assert_eq!(vk_to_key(0x60), None, "小键盘 0 暂不支持");
        assert_eq!(vk_to_key(0xA2), None, "修饰键不是主键");
        assert_eq!(vk_to_key(0xBA), None, "OEM 分号键随布局变化，不支持");
    }
}
