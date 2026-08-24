pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_WIN: u32 = 0x0008;
pub const MOD_NOREPEAT: u32 = 0x4000;

/// 一条热键最多带几个主键。
pub const MAX_KEYS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHotkey {
    pub modifiers: u32,
    /// 主键，按虚拟键码升序去重；为空表示纯修饰键热键。
    pub keys: Vec<u16>,
}

impl ParsedHotkey {
    /// 只有低级键盘钩子能表达的组合：纯修饰键或多主键。
    /// `RegisterHotKey` 只收「修饰键 + 单个主键」。
    pub fn requires_hook(&self) -> bool {
        self.keys.len() != 1
    }

    /// 唯一的主键；多主键或纯修饰键时为 None。
    pub fn single(&self) -> Option<u16> {
        match self.keys.as_slice() {
            [vk] => Some(*vk),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HotkeyParseError {
    #[error("热键为空")]
    Empty,
    #[error("无法识别的按键: {0}")]
    UnknownKey(String),
    #[error("热键的主键超过 {MAX_KEYS} 个")]
    TooManyKeys,
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

    if let Some(rest) = norm.strip_prefix("NUMPAD")
        && let Ok(n) = rest.parse::<u16>()
        && n <= 9
    {
        return Some(0x60 + n);
    }

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
        "APPS" => 0x5D,
        "NUMPADMULTIPLY" => 0x6A,
        "NUMPADADD" => 0x6B,
        "NUMPADSEPARATOR" => 0x6C,
        "NUMPADSUBTRACT" => 0x6D,
        "NUMPADDECIMAL" => 0x6E,
        "NUMPADDIVIDE" => 0x6F,
        "VOLUMEMUTE" => 0xAD,
        "VOLUMEDOWN" => 0xAE,
        "VOLUMEUP" => 0xAF,
        "MEDIANEXT" => 0xB0,
        "MEDIAPREV" => 0xB1,
        "MEDIASTOP" => 0xB2,
        "MEDIAPLAYPAUSE" => 0xB3,
        // OEM 键随键盘布局改变字面含义，故存位置名，界面另按当前布局显示实际字符。
        "OEM1" => 0xBA,
        "OEMPLUS" => 0xBB,
        "OEMCOMMA" => 0xBC,
        "OEMMINUS" => 0xBD,
        "OEMPERIOD" => 0xBE,
        "OEM2" => 0xBF,
        "OEM3" => 0xC0,
        "OEM4" => 0xDB,
        "OEM5" => 0xDC,
        "OEM6" => 0xDD,
        "OEM7" => 0xDE,
        "OEM8" => 0xDF,
        "OEM102" => 0xE2,
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
    if (0x60..=0x69).contains(&vk) {
        return Some(format!("Numpad{}", vk - 0x60));
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
        0x5D => "Apps",
        0x6A => "NumpadMultiply",
        0x6B => "NumpadAdd",
        0x6C => "NumpadSeparator",
        0x6D => "NumpadSubtract",
        0x6E => "NumpadDecimal",
        0x6F => "NumpadDivide",
        0xAD => "VolumeMute",
        0xAE => "VolumeDown",
        0xAF => "VolumeUp",
        0xB0 => "MediaNext",
        0xB1 => "MediaPrev",
        0xB2 => "MediaStop",
        0xB3 => "MediaPlayPause",
        0xBA => "OEM_1",
        0xBB => "OEM_PLUS",
        0xBC => "OEM_COMMA",
        0xBD => "OEM_MINUS",
        0xBE => "OEM_PERIOD",
        0xBF => "OEM_2",
        0xC0 => "OEM_3",
        0xDB => "OEM_4",
        0xDC => "OEM_5",
        0xDD => "OEM_6",
        0xDE => "OEM_7",
        0xDF => "OEM_8",
        0xE2 => "OEM_102",
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

/// 组装热键字符串；`keys` 为空时只输出修饰键（纯修饰键热键或录制中间态）。
/// 表外的主键当作没按到，直接略去。
pub fn format_hotkey(modifiers: u32, keys: &[u16]) -> String {
    let mut parts = Vec::new();
    let mods = format_modifiers(modifiers);
    if !mods.is_empty() {
        parts.push(mods);
    }
    parts.extend(keys.iter().copied().filter_map(vk_to_key));
    parts.join("+")
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
    let mut keys: Vec<u16> = Vec::new();

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
        if !keys.contains(&key) {
            keys.push(key);
        }
    }

    if keys.len() > MAX_KEYS {
        return Err(HotkeyParseError::TooManyKeys);
    }
    if modifiers == 0 && keys.is_empty() {
        return Err(HotkeyParseError::Empty);
    }
    // 归一顺序，让 "Q+W" 与 "W+Q" 是同一条热键。
    keys.sort_unstable();
    Ok(ParsedHotkey { modifiers, keys })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hk(modifiers: u32, keys: &[u16]) -> ParsedHotkey {
        ParsedHotkey {
            modifiers,
            keys: keys.to_vec(),
        }
    }

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
        assert_eq!(parse_hotkey("Ctrl+Q"), Ok(hk(MOD_CONTROL, &[0x51])));
    }

    #[test]
    fn parses_default_close_hotkey() {
        assert_eq!(parse_hotkey("Win+Esc"), Ok(hk(MOD_WIN, &[0x1B])));
    }

    #[test]
    fn parses_multiple_modifiers_and_function_key() {
        assert_eq!(
            parse_hotkey("Ctrl+Shift+F1"),
            Ok(hk(MOD_CONTROL | MOD_SHIFT, &[0x70]))
        );
    }

    #[test]
    fn f24_maps_to_correct_vk() {
        assert_eq!(parse_hotkey("F24").unwrap().keys, vec![0x87]);
    }

    #[test]
    fn digits_and_letters_map_to_ascii_vk() {
        assert_eq!(parse_hotkey("Alt+5").unwrap().keys, vec![0x35]);
        assert_eq!(parse_hotkey("Ctrl+A").unwrap().keys, vec![0x41]);
    }

    #[test]
    fn is_case_insensitive_and_whitespace_tolerant() {
        let a = parse_hotkey("ctrl + shift + q").unwrap();
        let b = parse_hotkey("CTRL+SHIFT+Q").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.modifiers, MOD_CONTROL | MOD_SHIFT);
        assert_eq!(a.keys, vec![0x51]);
    }

    #[test]
    fn accepts_alternate_named_keys() {
        assert_eq!(parse_hotkey("Ctrl+Page_Up").unwrap().keys, vec![0x21]);
        assert_eq!(parse_hotkey("Ctrl+PageUp").unwrap().keys, vec![0x21]);
        assert_eq!(parse_hotkey("Alt+Space").unwrap().keys, vec![0x20]);
        assert_eq!(parse_hotkey("Shift+Print Screen").unwrap().keys, vec![0x2C]);
    }

    #[test]
    fn accepts_numpad_oem_and_media_keys() {
        assert_eq!(parse_hotkey("Ctrl+Numpad0").unwrap().keys, vec![0x60]);
        assert_eq!(parse_hotkey("Ctrl+Numpad9").unwrap().keys, vec![0x69]);
        assert_eq!(parse_hotkey("Ctrl+NumpadAdd").unwrap().keys, vec![0x6B]);
        assert_eq!(parse_hotkey("Ctrl+OEM_1").unwrap().keys, vec![0xBA]);
        assert_eq!(parse_hotkey("Ctrl+OEM_102").unwrap().keys, vec![0xE2]);
        assert_eq!(parse_hotkey("VolumeUp").unwrap().keys, vec![0xAF]);
        assert_eq!(parse_hotkey("MediaPlayPause").unwrap().keys, vec![0xB3]);
        assert_eq!(parse_hotkey("Shift+Apps").unwrap().keys, vec![0x5D]);
    }

    #[test]
    fn blank_hotkey_counts_as_disabled() {
        assert!(is_disabled(""));
        assert!(is_disabled("   "));
        assert!(!is_disabled("Ctrl+Q"));
        assert!(!is_disabled("Ctrl+Shift"));
    }

    #[test]
    fn empty_is_rejected() {
        assert_eq!(parse_hotkey("   "), Err(HotkeyParseError::Empty));
        assert_eq!(parse_hotkey("+ + +"), Err(HotkeyParseError::Empty));
    }

    #[test]
    fn modifier_only_hotkeys_are_accepted() {
        assert_eq!(
            parse_hotkey("Ctrl+Shift"),
            Ok(hk(MOD_CONTROL | MOD_SHIFT, &[]))
        );
        assert_eq!(parse_hotkey("Win"), Ok(hk(MOD_WIN, &[])));
    }

    #[test]
    fn multiple_main_keys_are_accepted_and_sorted() {
        assert_eq!(parse_hotkey("W+Q"), Ok(hk(0, &[0x51, 0x57])));
        assert_eq!(
            parse_hotkey("Q+W"),
            parse_hotkey("W+Q"),
            "主键顺序不影响热键身份"
        );
        assert_eq!(
            parse_hotkey("Q+Q").unwrap().keys,
            vec![0x51],
            "重复主键去重"
        );
    }

    #[test]
    fn too_many_main_keys_is_rejected() {
        assert_eq!(
            parse_hotkey("Q+W+E+R+T"),
            Err(HotkeyParseError::TooManyKeys)
        );
        assert!(parse_hotkey("Q+W+E+R").is_ok(), "MAX_KEYS 个仍然合法");
    }

    #[test]
    fn unknown_key_is_rejected() {
        assert_eq!(
            parse_hotkey("Ctrl+Frobnicate"),
            Err(HotkeyParseError::UnknownKey("Frobnicate".to_string()))
        );
    }

    #[test]
    fn requires_hook_only_when_the_main_key_count_is_not_one() {
        assert!(!parse_hotkey("Ctrl+Q").unwrap().requires_hook());
        assert!(!parse_hotkey("Ctrl+Numpad0").unwrap().requires_hook());
        assert!(parse_hotkey("Ctrl+Shift").unwrap().requires_hook());
        assert!(parse_hotkey("Q+W").unwrap().requires_hook());
    }

    #[test]
    fn single_yields_the_key_only_for_register_hotkey_shaped_combos() {
        assert_eq!(parse_hotkey("Ctrl+Q").unwrap().single(), Some(0x51));
        assert_eq!(parse_hotkey("Ctrl+Shift").unwrap().single(), None);
        assert_eq!(parse_hotkey("Q+W").unwrap().single(), None);
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
        assert_eq!(format_hotkey(MOD_CONTROL | MOD_SHIFT, &[]), "Ctrl+Shift");
        assert_eq!(format_hotkey(0, &[]), "");
        // 表外的主键当作还没按到主键。
        assert_eq!(format_hotkey(MOD_ALT, &[0xFF]), "Alt");
    }

    #[test]
    fn format_hotkey_without_modifiers_yields_the_bare_keys() {
        assert_eq!(format_hotkey(0, &[0x70]), "F1");
        assert_eq!(format_hotkey(0, &[0x51, 0x57]), "Q+W");
    }

    /// 录制出来的字符串必须能被 `parse_hotkey` 原样解回来。
    #[test]
    fn every_supported_vk_round_trips_through_parse_hotkey() {
        let modifiers = MOD_CONTROL | MOD_SHIFT;
        let mut count = 0;
        for vk in 0..=0xFFu16 {
            let Some(_) = vk_to_key(vk) else { continue };
            count += 1;
            let text = format_hotkey(modifiers, &[vk]);
            assert_eq!(
                parse_hotkey(&text),
                Ok(hk(modifiers, &[vk])),
                "{text} 解析结果与录制来源不符"
            );
        }
        // 26 字母 + 10 数字 + 24 功能键 + 21 命名键 + 16 小键盘 + 13 OEM + 7 媒体键 + Apps
        assert_eq!(count, 118, "支持的按键数量变了，确认改动是有意的");
    }

    #[test]
    fn vk_to_key_rejects_keys_outside_the_table() {
        assert_eq!(vk_to_key(0xA2), None, "修饰键不是主键");
        assert_eq!(vk_to_key(0xFF), None);
    }
}
