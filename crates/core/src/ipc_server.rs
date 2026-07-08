use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::FromRawHandle;

use bosskey_common::ipc::{Command, Response};
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows::core::PCWSTR;

use crate::util::to_wide_null;

const PIPE_BUF_SIZE: u32 = 4096;

pub fn spawn<F>(pipe_name: String, executor: F)
where
    F: Fn(Command) -> Response + Send + 'static,
{
    std::thread::Builder::new()
        .name("bosskey-ipc".to_string())
        .spawn(move || serve_loop(&pipe_name, executor))
        .expect("无法启动 IPC 线程");
}

fn serve_loop<F>(pipe_name: &str, executor: F)
where
    F: Fn(Command) -> Response,
{
    let wide_name = to_wide_null(pipe_name);
    loop {
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(wide_name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                1,
                PIPE_BUF_SIZE,
                PIPE_BUF_SIZE,
                0,
                None,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            eprintln!("创建命名管道失败: {pipe_name}");
            return;
        }

        let connected = unsafe { ConnectNamedPipe(handle, None) };
        if connected.is_err() {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(handle);
            }
            continue;
        }

        let file = unsafe { File::from_raw_handle(handle.0) };
        serve_client(file, &executor);
    }
}

fn serve_client<F>(file: File, executor: &F)
where
    F: Fn(Command) -> Response,
{
    let mut writer = match file.try_clone() {
        Ok(f) => f,
        Err(_) => return,
    };
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = match Command::from_line(&line) {
            Ok(cmd) => executor(cmd),
            Err(e) => Response::Error {
                message: format!("无法解析命令: {e}"),
            },
        };
        let Ok(mut out) = response.to_line() else {
            break;
        };
        out.push('\n');
        if writer.write_all(out.as_bytes()).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn connect_with_retry(pipe_name: &str) -> File {
        for _ in 0..50 {
            if let Ok(f) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(pipe_name)
            {
                return f;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("无法连接测试管道 {pipe_name}");
    }

    fn request(file: &mut File, line: &str) -> String {
        writeln!(file, "{line}").unwrap();
        file.flush().unwrap();
        let mut reader = BufReader::new(file.try_clone().unwrap());
        let mut buf = String::new();
        reader.read_line(&mut buf).unwrap();
        buf.trim_end().to_string()
    }

    #[test]
    fn get_state_round_trips_over_a_real_pipe() {
        let pipe = r"\\.\pipe\bosskey_test_get_state";
        spawn(pipe.to_string(), |cmd| match cmd {
            Command::GetState => Response::State { hidden: true },
            _ => Response::Ok,
        });

        let mut client = connect_with_retry(pipe);
        let reply = request(&mut client, r#"{"cmd":"get_state"}"#);
        assert_eq!(reply, r#"{"type":"state","hidden":true}"#);
    }

    #[test]
    fn invalid_json_gets_error_response_and_connection_survives() {
        let pipe = r"\\.\pipe\bosskey_test_bad_json";
        spawn(pipe.to_string(), |_| Response::Ok);

        let mut client = connect_with_retry(pipe);
        let reply = request(&mut client, "not json at all");
        let parsed = Response::from_line(&reply).unwrap();
        assert!(matches!(parsed, Response::Error { .. }));

        let reply2 = request(&mut client, r#"{"cmd":"toggle"}"#);
        assert_eq!(reply2, r#"{"type":"ok"}"#);
    }

    #[test]
    fn sequential_clients_are_served() {
        let pipe = r"\\.\pipe\bosskey_test_sequential";
        spawn(pipe.to_string(), |_| Response::Ok);

        for _ in 0..3 {
            let mut client = connect_with_retry(pipe);
            let reply = request(&mut client, r#"{"cmd":"hide"}"#);
            assert_eq!(reply, r#"{"type":"ok"}"#);
            drop(client);
        }
    }

    #[test]
    fn multiple_commands_on_one_connection() {
        let pipe = r"\\.\pipe\bosskey_test_multi_cmd";
        spawn(pipe.to_string(), |cmd| match cmd {
            Command::Hide => Response::Ok,
            Command::GetState => Response::State { hidden: false },
            _ => Response::Error {
                message: "unexpected".into(),
            },
        });

        let mut client = connect_with_retry(pipe);
        assert_eq!(
            request(&mut client, r#"{"cmd":"hide"}"#),
            r#"{"type":"ok"}"#
        );
        assert_eq!(
            request(&mut client, r#"{"cmd":"get_state"}"#),
            r#"{"type":"state","hidden":false}"#
        );
    }
}
