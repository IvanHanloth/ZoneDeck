use serde::{Deserialize, Serialize};

pub const PIPE_NAME: &str = r"\\.\pipe\bosskey";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    ReloadConfig,
    GetState,
    Hide,
    Show,
    Toggle,
    SetAutostart { enabled: bool },
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Ok,
    State { hidden: bool },
    Error { message: String },
}

impl Command {
    pub fn to_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

impl Response {
    pub fn to_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_round_trip_all_variants() {
        let cases = [
            Command::ReloadConfig,
            Command::GetState,
            Command::Hide,
            Command::Show,
            Command::Toggle,
            Command::SetAutostart { enabled: true },
            Command::Quit,
        ];
        for c in cases {
            let line = c.to_line().unwrap();
            assert_eq!(Command::from_line(&line).unwrap(), c);
        }
    }

    #[test]
    fn command_tag_uses_snake_case() {
        assert_eq!(
            Command::ReloadConfig.to_line().unwrap(),
            r#"{"cmd":"reload_config"}"#
        );
        assert_eq!(
            Command::SetAutostart { enabled: false }.to_line().unwrap(),
            r#"{"cmd":"set_autostart","enabled":false}"#
        );
    }

    #[test]
    fn response_round_trip_all_variants() {
        let cases = [
            Response::Ok,
            Response::State { hidden: true },
            Response::State { hidden: false },
            Response::Error {
                message: "出错了".to_string(),
            },
        ];
        for r in cases {
            let line = r.to_line().unwrap();
            assert_eq!(Response::from_line(&line).unwrap(), r);
        }
    }

    #[test]
    fn response_tag_uses_snake_case() {
        assert_eq!(Response::Ok.to_line().unwrap(), r#"{"type":"ok"}"#);
        assert_eq!(
            Response::State { hidden: true }.to_line().unwrap(),
            r#"{"type":"state","hidden":true}"#
        );
    }
}
