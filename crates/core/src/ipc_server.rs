use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::FromRawHandle;

use bosskey_common::ipc::{Command, Response};
use windows::Win32::Foundation::{ERROR_PIPE_CONNECTED, HLOCAL, INVALID_HANDLE_VALUE, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows::core::{HRESULT, PCWSTR};

use crate::log_error;
use crate::util::to_wide_null;

const PIPE_BUF_SIZE: u32 = 4096;

/// 创建管道失败后的重试间隔：逐级退避，最后一档一直沿用，线程不退出。
const RETRY_DELAYS: [std::time::Duration; 3] = [
    std::time::Duration::from_secs(1),
    std::time::Duration::from_secs(5),
    std::time::Duration::from_secs(30),
];

fn retry_delay(attempt: u32) -> std::time::Duration {
    RETRY_DELAYS[(attempt as usize).min(RETRY_DELAYS.len() - 1)]
}

/// 命名管道安全描述符（SDDL）：Everyone 可读写，完整性标签设为 Low，
/// 使普通配置程序能连上以管理员运行的核心。
const PIPE_SDDL: &str = "D:(A;;GRGW;;;WD)S:(ML;;NW;;;LW)";

/// 持有由 SDDL 解析出的安全描述符，并在析构时释放其内存（`LocalFree`）。
struct PipeSecurity {
    psd: PSECURITY_DESCRIPTOR,
    sa: SECURITY_ATTRIBUTES,
}

impl PipeSecurity {
    fn new() -> Option<Self> {
        let sddl = to_wide_null(PIPE_SDDL);
        let mut psd = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(sddl.as_ptr()),
                SDDL_REVISION_1,
                &mut psd,
                None,
            )
            .ok()?;
        }
        if psd.0.is_null() {
            return None;
        }
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: psd.0,
            bInheritHandle: false.into(),
        };
        Some(Self { psd, sa })
    }

    fn as_ptr(&self) -> *const SECURITY_ATTRIBUTES {
        &self.sa
    }
}

impl Drop for PipeSecurity {
    fn drop(&mut self) {
        if !self.psd.0.is_null() {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.psd.0)));
            }
        }
    }
}

fn is_client_connected(result: windows::core::Result<()>) -> bool {
    match result {
        Ok(()) => true,
        Err(e) => e.code() == HRESULT::from_win32(ERROR_PIPE_CONNECTED.0),
    }
}

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
    // 安全描述符构造一次，供所有管道实例复用。
    let security = PipeSecurity::new();
    if security.is_none() {
        crate::logging::warn("命名管道安全描述符构造失败，管理员核心下普通配置程序可能无法连接");
    }
    let sa_ptr = security.as_ref().map(PipeSecurity::as_ptr);
    let mut failures: u32 = 0;
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
                sa_ptr,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            let err = std::io::Error::last_os_error();
            let delay = retry_delay(failures);
            failures += 1;
            log_error!(
                "创建命名管道失败（第 {failures} 次），{delay:?} 后重试: {pipe_name} — {err}"
            );
            std::thread::sleep(delay);
            continue;
        }
        failures = 0;

        if !is_client_connected(unsafe { ConnectNamedPipe(handle, None) }) {
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
    use windows::Win32::Foundation::{ERROR_BROKEN_PIPE, WIN32_ERROR};

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
    fn retry_delay_backs_off_and_caps_at_the_last_step() {
        assert_eq!(retry_delay(0), Duration::from_secs(1));
        assert_eq!(retry_delay(1), Duration::from_secs(5));
        assert_eq!(retry_delay(2), Duration::from_secs(30));
        assert_eq!(retry_delay(3), Duration::from_secs(30));
        assert_eq!(retry_delay(1000), Duration::from_secs(30));
    }

    #[test]
    fn pipe_security_descriptor_builds_from_sddl() {
        let sec = PipeSecurity::new().expect("SDDL 应能解析出安全描述符");
        assert!(!sec.psd.0.is_null(), "安全描述符指针不应为空");
        assert_eq!(
            sec.sa.lpSecurityDescriptor, sec.psd.0,
            "SECURITY_ATTRIBUTES 应指向解析出的安全描述符"
        );
        assert_eq!(
            sec.sa.nLength as usize,
            std::mem::size_of::<SECURITY_ATTRIBUTES>()
        );
    }

    #[test]
    fn a_client_winning_the_connect_race_counts_as_connected() {
        let err =
            |code: WIN32_ERROR| windows::core::Error::from_hresult(HRESULT::from_win32(code.0));

        assert!(is_client_connected(Ok(())), "常规连接");
        assert!(
            is_client_connected(Err(err(ERROR_PIPE_CONNECTED))),
            "客户端抢在 ConnectNamedPipe 之前 open，属于连接已建立"
        );
        assert!(
            !is_client_connected(Err(err(ERROR_BROKEN_PIPE))),
            "其他错误仍应视为连接失败"
        );
    }

    #[test]
    fn back_to_back_reconnects_all_get_served() {
        let pipe = r"\\.\pipe\bosskey_test_reconnect_race";
        spawn(pipe.to_string(), |_| Response::Ok);

        let mut client = connect_with_retry(pipe);
        for i in 0..200 {
            let reply = request(&mut client, r#"{"cmd":"hide"}"#);
            assert_eq!(reply, r#"{"type":"ok"}"#, "第 {i} 轮应答异常");
            drop(client);
            client = connect_with_retry(pipe);
        }
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
