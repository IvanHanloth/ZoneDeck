use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::FromRawHandle;

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
use zonedeck_common::ipc::{Command, Response};

use crate::log_error;
use crate::util::to_wide_null;

const PIPE_BUF_SIZE: u32 = 4096;

/// 创建管道失败后的重试间隔，逐级退避，最后一档一直沿用。
const RETRY_DELAYS: [std::time::Duration; 3] = [
    std::time::Duration::from_secs(1),
    std::time::Duration::from_secs(5),
    std::time::Duration::from_secs(30),
];

fn retry_delay(attempt: u32) -> std::time::Duration {
    RETRY_DELAYS[(attempt as usize).min(RETRY_DELAYS.len() - 1)]
}

/// 命名管道的安全描述符（SDDL）：DACL 只授予当前用户，完整性标签定在 Medium。
fn pipe_sddl(user_sid: &str) -> String {
    format!("D:(A;;GRGW;;;{user_sid})S:(ML;;NW;;;ME)")
}

/// 进程令牌句柄，析构时关闭。
struct TokenHandle(windows::Win32::Foundation::HANDLE);

impl Drop for TokenHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

/// 当前进程所属用户的 SID 字符串（形如 `S-1-5-21-…-1001`）。
fn current_user_sid() -> Option<String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows::core::PWSTR;

    unsafe {
        let mut raw = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw).ok()?;
        let token = TokenHandle(raw);

        // 先问长度。
        let mut len = 0u32;
        let _ = GetTokenInformation(token.0, TokenUser, None, 0, &mut len);
        if len == 0 {
            return None;
        }
        // TOKEN_USER 内含指针，缓冲区须按指针对齐。
        let words = (len as usize).div_ceil(std::mem::size_of::<usize>());
        let mut buf = vec![0usize; words.max(1)];
        GetTokenInformation(
            token.0,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            len,
            &mut len,
        )
        .ok()?;

        let user = &*(buf.as_ptr() as *const TOKEN_USER);
        let mut text = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut text).ok()?;
        if text.is_null() {
            return None;
        }
        let sid = text.to_string().ok();
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
        sid
    }
}

/// 持有由 SDDL 解析出的安全描述符，并在析构时释放其内存（`LocalFree`）。
struct PipeSecurity {
    psd: PSECURITY_DESCRIPTOR,
    sa: SECURITY_ATTRIBUTES,
}

impl PipeSecurity {
    fn new() -> Option<Self> {
        Self::from_sddl(&pipe_sddl(&current_user_sid()?))
    }

    fn from_sddl(sddl: &str) -> Option<Self> {
        let sddl = to_wide_null(sddl);
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
        .name("zonedeck-ipc".to_string())
        .spawn(move || serve_loop(&pipe_name, executor))
        .expect("无法启动 IPC 线程");
}

fn serve_loop<F>(pipe_name: &str, executor: F)
where
    F: Fn(Command) -> Response,
{
    let wide_name = to_wide_null(pipe_name);
    let security = PipeSecurity::new();
    if security.is_none() {
        crate::logging::warn(
            "命名管道安全描述符构造失败，已回退系统默认；若核心以管理员运行，普通权限的配置程序将无法连接",
        );
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
            // 首次失败记 error，后续重试记 debug。
            if failures == 1 {
                log_error!(
                    "创建命名管道失败，配置程序将无法连接核心，{delay:?} 后重试: {pipe_name} — {err}"
                );
            } else {
                crate::logging::debug(&format!(
                    "创建命名管道失败（第 {failures} 次），{delay:?} 后重试: {pipe_name} — {err}"
                ));
            }
            std::thread::sleep(delay);
            continue;
        }
        if failures > 0 {
            crate::logging::warn(&format!(
                "命名管道重试 {failures} 次后创建成功，配置程序已可连接"
            ));
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
            Err(e) => {
                // 命令内容来路不明，只记开头一小段。
                crate::log_warn!(
                    "收到无法解析的 IPC 命令，已忽略: {} — {e}",
                    crate::util::head_chars(&line, 120)
                );
                Response::Error {
                    message: format!("无法解析命令: {e}"),
                }
            }
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

    #[test]
    fn the_current_user_sid_is_a_real_sid() {
        let sid = current_user_sid().expect("取当前用户 SID 失败");
        assert!(
            sid.starts_with("S-1-"),
            "应是标准 SID 字符串形式，实际 {sid}"
        );
        assert_eq!(sid, current_user_sid().unwrap(), "同一进程内两次取应一致");
    }

    #[test]
    fn the_pipe_is_scoped_to_the_current_user_not_everyone() {
        let sddl = pipe_sddl(&current_user_sid().unwrap());
        assert!(!sddl.contains(";WD)"), "DACL 不得再授予 Everyone: {sddl}");
        assert!(sddl.contains(";S-1-"), "DACL 须按具体用户 SID 授权: {sddl}");
    }

    #[test]
    fn the_integrity_label_stops_at_medium() {
        let sddl = pipe_sddl("S-1-5-21-1-2-3-1001");
        assert!(
            sddl.contains("S:(ML;;NW;;;ME)"),
            "完整性标签应为 Medium: {sddl}"
        );
        assert!(!sddl.contains(";LW)"), "不得再降到 Low 完整性: {sddl}");
    }

    #[test]
    fn a_malformed_sddl_is_reported_rather_than_silently_ignored() {
        assert!(PipeSecurity::from_sddl("这不是 SDDL").is_none());
    }

    /// 真造一个管道，把 Windows 实际存下的安全描述符读回来比对。
    #[test]
    fn the_created_pipe_really_carries_the_intended_acl() {
        use windows::Win32::Security::Authorization::{
            ConvertSecurityDescriptorToStringSecurityDescriptorW, GetSecurityInfo, SE_KERNEL_OBJECT,
        };
        use windows::Win32::Security::{
            DACL_SECURITY_INFORMATION, LABEL_SECURITY_INFORMATION, OBJECT_SECURITY_INFORMATION,
        };
        use windows::core::PWSTR;

        let sid = current_user_sid().unwrap();
        let security = PipeSecurity::new().expect("安全描述符应能构造");
        let name = to_wide_null(r"\\.\pipe\zonedeck_test_acl_readback");
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                1,
                PIPE_BUF_SIZE,
                PIPE_BUF_SIZE,
                0,
                Some(security.as_ptr()),
            )
        };
        assert!(handle != INVALID_HANDLE_VALUE, "测试管道应能创建");

        let wanted = DACL_SECURITY_INFORMATION.0 | LABEL_SECURITY_INFORMATION.0;
        let mut psd = PSECURITY_DESCRIPTOR::default();
        let rc = unsafe {
            GetSecurityInfo(
                handle,
                SE_KERNEL_OBJECT,
                OBJECT_SECURITY_INFORMATION(wanted),
                None,
                None,
                None,
                None,
                Some(&mut psd),
            )
        };
        assert!(rc.is_ok(), "应能读回管道的安全描述符: {rc:?}");

        let mut text = PWSTR::null();
        let converted = unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                psd,
                SDDL_REVISION_1,
                OBJECT_SECURITY_INFORMATION(wanted),
                &mut text,
                None,
            )
        };
        assert!(converted.is_ok(), "应能把描述符转回 SDDL");
        let readback = unsafe { text.to_string().unwrap() };
        unsafe {
            let _ = LocalFree(Some(HLOCAL(text.0.cast())));
            let _ = LocalFree(Some(HLOCAL(psd.0)));
            let _ = windows::Win32::Foundation::CloseHandle(handle);
        }

        assert!(
            readback.contains(&sid),
            "管道上实际生效的 DACL 应授权给当前用户 SID: {readback}"
        );
        assert!(
            !readback.contains(";WD)"),
            "管道上实际生效的 DACL 不得再包含 Everyone: {readback}"
        );
        assert!(
            readback.contains("(ML;;NW;;;ME)"),
            "管道上实际生效的完整性标签应为 Medium: {readback}"
        );
    }

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
        let pipe = r"\\.\pipe\zonedeck_test_reconnect_race";
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
        let pipe = r"\\.\pipe\zonedeck_test_get_state";
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
        let pipe = r"\\.\pipe\zonedeck_test_bad_json";
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
        let pipe = r"\\.\pipe\zonedeck_test_sequential";
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
        let pipe = r"\\.\pipe\zonedeck_test_multi_cmd";
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
