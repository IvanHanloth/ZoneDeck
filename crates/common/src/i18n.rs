//! 界面语言标识与解析；核心与配置程序共用同一套语言标签。

/// 配置中表示「跟随系统」的语言偏好值。
pub const LANG_AUTO: &str = "auto";

/// 界面语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    /// 简体中文。
    #[default]
    ZhCn,
    /// 英文。
    En,
    /// 繁体中文（台湾用语）。
    ZhTw,
}

impl Lang {
    /// 该语言的规范标签，与配置文件、前端 catalog 文件名一致。
    pub const fn tag(self) -> &'static str {
        match self {
            Lang::ZhCn => "zh-CN",
            Lang::En => "en",
            Lang::ZhTw => "zh-TW",
        }
    }

    /// 所有可选语言，顺序即配置界面的展示顺序。
    pub const ALL: [Lang; 3] = [Lang::ZhCn, Lang::En, Lang::ZhTw];

    /// 按 BCP-47 标签解析语言，无法归类时返回 `None`。
    /// 中文按 `Hant`/`TW`/`HK`/`MO` 归为繁体，其余变体归为简体。
    pub fn from_tag(tag: &str) -> Option<Lang> {
        let tag = tag.trim().replace('_', "-").to_ascii_lowercase();
        let mut parts = tag.split('-').filter(|p| !p.is_empty());
        match parts.next()? {
            "en" => Some(Lang::En),
            "zh" => {
                let traditional = parts.any(|p| matches!(p, "hant" | "tw" | "hk" | "mo"));
                Some(if traditional { Lang::ZhTw } else { Lang::ZhCn })
            }
            _ => None,
        }
    }
}

/// 归一化配置里的语言偏好：合法语言标签归一为规范写法，其余一律回落到 [`LANG_AUTO`]。
pub fn normalize_pref(pref: &str) -> String {
    if pref.trim().eq_ignore_ascii_case(LANG_AUTO) {
        return LANG_AUTO.to_string();
    }
    match Lang::from_tag(pref) {
        Some(lang) => lang.tag().to_string(),
        None => LANG_AUTO.to_string(),
    }
}

/// 解析实际生效的语言：`pref` 为具体语言时直接采用，否则依据 `system_tag` 推断，
/// 无法推断时回落简体中文。
pub fn resolve(pref: &str, system_tag: Option<&str>) -> Lang {
    if !pref.trim().eq_ignore_ascii_case(LANG_AUTO)
        && let Some(lang) = Lang::from_tag(pref)
    {
        return lang;
    }
    system_tag.and_then(Lang::from_tag).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simplified_chinese_variants() {
        for tag in [
            "zh",
            "zh-CN",
            "zh_CN",
            "zh-Hans",
            "zh-Hans-CN",
            "ZH-cn",
            " zh-SG ",
        ] {
            assert_eq!(Lang::from_tag(tag), Some(Lang::ZhCn), "{tag}");
        }
    }

    #[test]
    fn parses_traditional_chinese_variants() {
        for tag in ["zh-TW", "zh_TW", "zh-Hant", "zh-Hant-TW", "zh-HK", "zh-MO"] {
            assert_eq!(Lang::from_tag(tag), Some(Lang::ZhTw), "{tag}");
        }
    }

    #[test]
    fn parses_english_variants() {
        for tag in ["en", "en-US", "en_GB", "EN"] {
            assert_eq!(Lang::from_tag(tag), Some(Lang::En), "{tag}");
        }
    }

    #[test]
    fn unsupported_language_is_not_recognized() {
        for tag in ["ja", "ko", "fr-FR", "", "   ", "auto"] {
            assert_eq!(Lang::from_tag(tag), None, "{tag}");
        }
    }

    #[test]
    fn tags_round_trip() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_tag(lang.tag()), Some(lang));
        }
    }

    #[test]
    fn normalize_keeps_auto_and_canonicalizes_tags() {
        assert_eq!(normalize_pref("auto"), "auto");
        assert_eq!(normalize_pref("AUTO"), "auto");
        assert_eq!(normalize_pref("zh_tw"), "zh-TW");
        assert_eq!(normalize_pref("en-US"), "en");
    }

    #[test]
    fn normalize_falls_back_to_auto_for_unsupported() {
        assert_eq!(normalize_pref("ja-JP"), "auto");
        assert_eq!(normalize_pref(""), "auto");
    }

    #[test]
    fn explicit_preference_overrides_system() {
        assert_eq!(resolve("en", Some("zh-CN")), Lang::En);
        assert_eq!(resolve("zh-TW", Some("en-US")), Lang::ZhTw);
    }

    #[test]
    fn auto_follows_system() {
        assert_eq!(resolve(LANG_AUTO, Some("en-US")), Lang::En);
        assert_eq!(resolve(LANG_AUTO, Some("zh-Hant-TW")), Lang::ZhTw);
        assert_eq!(resolve(LANG_AUTO, Some("zh-CN")), Lang::ZhCn);
    }

    #[test]
    fn auto_falls_back_to_simplified_chinese() {
        assert_eq!(resolve(LANG_AUTO, None), Lang::ZhCn);
        assert_eq!(resolve(LANG_AUTO, Some("ja-JP")), Lang::ZhCn);
    }

    #[test]
    fn unsupported_preference_falls_back_to_system() {
        assert_eq!(resolve("ja-JP", Some("en-US")), Lang::En);
    }
}
