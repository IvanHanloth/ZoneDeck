use serde::{Deserialize, Serialize};

use crate::NO_TITLE;

fn default_title() -> String {
    NO_TITLE.to_string()
}

fn de_null_default<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(de)?.unwrap_or_default())
}

fn de_title<'de, D>(de: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(de)?.unwrap_or_else(default_title))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowInfo {
    #[serde(default = "default_title", deserialize_with = "de_title")]
    pub title: String,
    #[serde(default, deserialize_with = "de_null_default")]
    pub hwnd: i64,
    #[serde(default, deserialize_with = "de_null_default")]
    pub process: String,
    #[serde(rename = "PID", default, deserialize_with = "de_null_default")]
    pub pid: u32,
    #[serde(default, deserialize_with = "de_null_default")]
    pub path: String,
}

impl WindowInfo {
    pub fn new(
        title: impl Into<String>,
        hwnd: i64,
        process: impl Into<String>,
        pid: u32,
        path: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            hwnd,
            process: process.into(),
            pid,
            path: path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_pid_uppercase_key() {
        let json = r#"{"title":"记事本","hwnd":123,"process":"notepad.exe","PID":4567,"path":"C:\\notepad.exe"}"#;
        let w: WindowInfo = serde_json::from_str(json).unwrap();
        assert_eq!(w.title, "记事本");
        assert_eq!(w.hwnd, 123);
        assert_eq!(w.process, "notepad.exe");
        assert_eq!(w.pid, 4567);
        assert_eq!(w.path, "C:\\notepad.exe");
    }

    #[test]
    fn serializes_pid_uppercase_key() {
        let w = WindowInfo::new("t", 1, "p.exe", 9, "C:\\p.exe");
        let json = serde_json::to_string(&w).unwrap();
        assert!(
            json.contains("\"PID\":9"),
            "序列化应输出大写 PID 字段: {json}"
        );
        assert!(!json.contains("\"pid\""), "不应输出小写 pid: {json}");
    }

    #[test]
    fn missing_title_defaults_to_no_title() {
        let json = r#"{"hwnd":1,"process":"p.exe","PID":2,"path":""}"#;
        let w: WindowInfo = serde_json::from_str(json).unwrap();
        assert_eq!(w.title, NO_TITLE);
    }

    #[test]
    fn null_fields_fall_back_to_defaults() {
        let json = r#"{"title":null,"hwnd":null,"process":null,"PID":null,"path":null}"#;
        let w: WindowInfo = serde_json::from_str(json).unwrap();
        assert_eq!(w.title, NO_TITLE);
        assert_eq!(w.hwnd, 0);
        assert_eq!(w.process, "");
        assert_eq!(w.pid, 0);
        assert_eq!(w.path, "");
    }

    #[test]
    fn round_trip_preserves_values() {
        let w = WindowInfo::new("窗口", 42, "app.exe", 100, "D:\\app.exe");
        let json = serde_json::to_string(&w).unwrap();
        let back: WindowInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(w, back);
    }

    #[test]
    fn empty_title_string_is_preserved() {
        let json = r#"{"title":"","hwnd":1,"process":"p.exe","PID":2,"path":""}"#;
        let w: WindowInfo = serde_json::from_str(json).unwrap();
        assert_eq!(w.title, "");
    }
}
