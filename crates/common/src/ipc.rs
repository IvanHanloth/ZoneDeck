use serde::{Deserialize, Serialize};

pub const PIPE_NAME: &str = r"\\.\pipe\bosskey";

/// 监控停用的看门狗时长：超过这么久没收到心跳，核心自动恢复监控。
pub const SUSPEND_TIMEOUT_MS: u32 = 15_000;
/// 配置程序重发停用心跳的建议间隔，须显著小于 `SUSPEND_TIMEOUT_MS`。
pub const SUSPEND_HEARTBEAT_MS: u32 = 4_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    ReloadConfig,
    GetState,
    GetElevation,
    /// 一次往返取回全部状态（隐藏态 + 权限），替代 GetState + GetElevation 两连发。
    GetStatus,
    Hide,
    Show,
    Toggle,
    SetAutostart {
        enabled: bool,
        /// `true` 注册最高权限计划任务，`false` 用普通权限。仅在 `enabled=true` 时有意义。
        admin: bool,
    },
    /// 临时停用 / 恢复全局热键与鼠标监控。停用有状态，须持续心跳续期。
    SetHotkeys {
        enabled: bool,
    },
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Ok,
    State {
        hidden: bool,
    },
    Elevated {
        elevated: bool,
    },
    /// `monitoring`：核心是否正在监听热键与鼠标（被 `SetHotkeys` 停用时为 false）。
    Status {
        hidden: bool,
        elevated: bool,
        monitoring: bool,
    },
    Error {
        message: String,
    },
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

/// 连接常驻核心命名管道服务端的客户端。
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

    /// 快速失败模式：只尝试连接一次，不重试。
    pub fn fast(mut self) -> Self {
        self.connect_attempts = 1;
        self
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
            Command::GetStatus,
            Command::Hide,
            Command::Show,
            Command::Toggle,
            Command::SetAutostart {
                enabled: true,
                admin: true,
            },
            Command::SetAutostart {
                enabled: false,
                admin: false,
            },
            Command::SetHotkeys { enabled: true },
            Command::SetHotkeys { enabled: false },
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
            Command::SetAutostart {
                enabled: false,
                admin: false
            }
            .to_line()
            .unwrap(),
            r#"{"cmd":"set_autostart","enabled":false,"admin":false}"#
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
            Response::Status {
                hidden: true,
                elevated: false,
                monitoring: true,
            },
            Response::Status {
                hidden: false,
                elevated: true,
                monitoring: false,
            },
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
