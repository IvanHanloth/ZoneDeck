use serde::{Deserialize, Serialize};

pub const PIPE_NAME: &str = r"\\.\pipe\bosskey";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    ReloadConfig,
    GetState,
    GetElevation,
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
    Elevated { elevated: bool },
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

/// 配置程序连接常驻核心命名管道服务端的客户端。
/// Windows 命名管道的客户端等价于按路径打开文件，因此这里仅用标准库实现，保持跨平台可编译。
pub struct PipeClient {
    pipe_name: String,
    connect_attempts: u32,
    connect_interval: std::time::Duration,
}

impl PipeClient {
    pub fn new(pipe_name: impl Into<String>) -> Self {
        Self {
            pipe_name: pipe_name.into(),
            connect_attempts: 25,
            connect_interval: std::time::Duration::from_millis(40),
        }
    }

    pub fn connect_default() -> Self {
        Self::new(PIPE_NAME)
    }

    pub fn send(&self, command: &Command) -> std::io::Result<Response> {
        use std::io::{BufRead, BufReader, Write};

        let file = self.open_with_retry()?;
        let mut writer = file.try_clone()?;
        let mut line = command.to_line().map_err(json_to_io)?;
        line.push('\n');
        writer.write_all(line.as_bytes())?;
        writer.flush()?;

        let mut reader = BufReader::new(file);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        Response::from_line(response.trim_end()).map_err(json_to_io)
    }

    fn open_with_retry(&self) -> std::io::Result<std::fs::File> {
        let mut last_err = None;
        for _ in 0..self.connect_attempts.max(1) {
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.pipe_name)
            {
                Ok(file) => return Ok(file),
                Err(e) => {
                    last_err = Some(e);
                    std::thread::sleep(self.connect_interval);
                }
            }
        }
        Err(last_err
            .unwrap_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "命名管道不可用")))
    }
}

fn json_to_io(e: serde_json::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_round_trip_all_variants() {
        let cases = [
            Command::ReloadConfig,
            Command::GetState,
            Command::GetElevation,
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
            Response::Elevated { elevated: true },
            Response::Elevated { elevated: false },
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
