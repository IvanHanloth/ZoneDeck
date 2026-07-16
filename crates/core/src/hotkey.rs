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
}
