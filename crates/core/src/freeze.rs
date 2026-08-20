use std::path::Path;
use std::sync::OnceLock;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{
    SETPROCESSWORKINGSETSIZEEX_FLAGS, SetProcessWorkingSetSizeEx,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_SUSPEND_RESUME};
use windows::core::{PCSTR, s, w};

pub const PSSUSPEND_EXE: &str = "pssuspend64.exe";

type NtProc = unsafe extern "system" fn(HANDLE) -> i32;

#[derive(Debug, thiserror::Error)]
pub enum FreezeError {
    #[error("ntdll 中未找到 {0}")]
    NtdllUnavailable(&'static str),
    /// `OpenProcess` 失败，多为权限不足或进程已退出；带上系统错误码便于排查。
    #[error("OpenProcess 失败（需要 PROCESS_SUSPEND_RESUME 权限）: {0}")]
    OpenFailed(String),
    #[error("{call} 失败，NTSTATUS=0x{status:08X}")]
    NtFailed { call: &'static str, status: i32 },
    /// 清空工作集失败，多为权限不足或进程已退出。
    #[error("清空工作集失败: {0}")]
    TrimFailed(String),
    #[error("未找到 pssuspend64.exe")]
    PssuspendMissing,
    #[error("pssuspend64 执行失败: {0}")]
    PssuspendFailed(String),
}

fn resolve(name: PCSTR) -> Option<NtProc> {
    unsafe {
        let ntdll = GetModuleHandleW(w!("ntdll.dll")).ok()?;
        let addr = GetProcAddress(ntdll, name)?;
        Some(std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            NtProc,
        >(addr))
    }
}

fn call_nt(pid: u32, proc: NtProc, call: &'static str) -> Result<(), FreezeError> {
    unsafe {
        let handle = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid)
            .map_err(|e| FreezeError::OpenFailed(crate::util::win_err(&e)))?;
        let status = proc(handle);
        let _ = CloseHandle(handle);
        if status < 0 {
            return Err(FreezeError::NtFailed { call, status });
        }
        Ok(())
    }
}

pub fn suspend_process(pid: u32) -> Result<(), FreezeError> {
    static SUSPEND: OnceLock<Option<NtProc>> = OnceLock::new();
    let proc = (*SUSPEND.get_or_init(|| resolve(s!("NtSuspendProcess"))))
        .ok_or(FreezeError::NtdllUnavailable("NtSuspendProcess"))?;
    call_nt(pid, proc, "NtSuspendProcess")
}

pub fn resume_process(pid: u32) -> Result<(), FreezeError> {
    static RESUME: OnceLock<Option<NtProc>> = OnceLock::new();
    let proc = (*RESUME.get_or_init(|| resolve(s!("NtResumeProcess"))))
        .ok_or(FreezeError::NtdllUnavailable("NtResumeProcess"))?;
    call_nt(pid, proc, "NtResumeProcess")
}

/// 清空进程工作集：把驻留物理内存的页赶到备用页表 / 页文件。
/// 挂起不触碰内存管理器，冻结本身不会让内存下降，须显式做这一步。
/// 只对已被挂起的进程调用；解冻时的硬缺页会让恢复变慢。
pub fn trim_working_set(pid: u32) -> Result<(), FreezeError> {
    unsafe {
        let handle = OpenProcess(PROCESS_SET_QUOTA, false, pid)
            .map_err(|e| FreezeError::OpenFailed(crate::util::win_err(&e)))?;
        // 上下限同时取 usize::MAX 是 Windows 约定的哨兵值，表示清空工作集。
        // flags 取 0，不启用硬性上下限。
        let result = SetProcessWorkingSetSizeEx(
            handle,
            usize::MAX,
            usize::MAX,
            SETPROCESSWORKINGSETSIZEEX_FLAGS(0),
        );
        let _ = CloseHandle(handle);
        result.map_err(|e| FreezeError::TrimFailed(crate::util::win_err(&e)))
    }
}

pub fn pssuspend_available(exe_dir: &Path) -> bool {
    exe_dir.join(PSSUSPEND_EXE).exists()
}

/// 枚举系统全部进程，返回 `(pid, 父 pid)` 列表；快照打不开时返回空表。
pub fn process_tree() -> Vec<(u32, u32)> {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    let mut edges = Vec::new();
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return edges;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                edges.push((entry.th32ProcessID, entry.th32ParentProcessID));
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    edges
}

/// 枚举系统全部进程，返回 `pid → 映像名` 的映射；快照打不开时返回空表。
pub fn process_names() -> std::collections::HashMap<u32, String> {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    let mut names = std::collections::HashMap::new();
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return names;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
                names.insert(entry.th32ProcessID, name);
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    names
}

fn run_pssuspend(exe_dir: &Path, args: &[&str]) -> Result<(), FreezeError> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe = exe_dir.join(PSSUSPEND_EXE);
    if !exe.exists() {
        return Err(FreezeError::PssuspendMissing);
    }
    // `-accepteula` 必不可少，否则未接受 EULA 时会弹对话框阻塞。旗标须排在 PID 之前。
    let output = std::process::Command::new(exe)
        .arg("-accepteula")
        .arg("-nobanner")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| FreezeError::PssuspendFailed(format!("无法启动进程: {e}")))?;

    if output.status.success() {
        Ok(())
    } else {
        // pssuspend 把失败原因打到 stdout 而非 stderr，且按本地代码页输出。
        let mut detail = crate::util::from_ansi(&output.stdout).trim().to_string();
        let stderr = crate::util::from_ansi(&output.stderr);
        let stderr = stderr.trim();
        if !stderr.is_empty() {
            if detail.is_empty() {
                detail = stderr.to_string();
            } else {
                detail.push('；');
                detail.push_str(stderr);
            }
        }
        let detail = if detail.is_empty() {
            format!("退出码 {}", output.status)
        } else {
            format!("退出码 {}，{detail}", output.status)
        };
        Err(FreezeError::PssuspendFailed(detail))
    }
}

pub fn suspend_enhanced(exe_dir: &Path, pid: u32) -> Result<(), FreezeError> {
    run_pssuspend(exe_dir, &[&pid.to_string()])
}

pub fn resume_enhanced(exe_dir: &Path, pid: u32) -> Result<(), FreezeError> {
    run_pssuspend(exe_dir, &["-r", &pid.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_child() -> std::process::Child {
        std::process::Command::new("cmd")
            .args(["/c", "ping -n 30 127.0.0.1 >nul"])
            .spawn()
            .expect("无法启动测试子进程")
    }

    #[test]
    fn suspend_and_resume_a_real_child_process() {
        let mut child = spawn_child();
        let pid = child.id();

        assert!(suspend_process(pid).is_ok(), "冻结有效子进程应成功");
        assert!(resume_process(pid).is_ok(), "解冻有效子进程应成功");

        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn suspend_invalid_pid_fails_gracefully() {
        let result = suspend_process(0xFFFF_FFF0);
        assert!(matches!(result, Err(FreezeError::OpenFailed(_))));
    }

    #[test]
    fn open_failure_reports_system_error_code() {
        // 错误须带系统错误码，才能分辨「权限不足」与「进程已退出」。
        let Err(e) = suspend_process(0xFFFF_FFF0) else {
            panic!("无效 PID 应失败");
        };
        let text = e.to_string();
        assert!(text.contains("OpenProcess"), "应指明失败的调用: {text}");
        assert!(text.contains("0x"), "应带系统错误码: {text}");
    }

    #[test]
    fn nt_failure_names_the_failing_call() {
        let e = FreezeError::NtFailed {
            call: "NtSuspendProcess",
            status: 0xC000_0022u32 as i32,
        };
        assert_eq!(e.to_string(), "NtSuspendProcess 失败，NTSTATUS=0xC0000022");
    }

    #[test]
    fn pssuspend_missing_reports_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!pssuspend_available(dir.path()));
        assert!(matches!(
            suspend_enhanced(dir.path(), 1234),
            Err(FreezeError::PssuspendMissing)
        ));
    }

    #[test]
    fn trim_working_set_succeeds_on_our_own_process() {
        // 对自己调用总是有权限的，此处只验证调用链通。
        assert!(
            trim_working_set(std::process::id()).is_ok(),
            "对自身进程清空工作集应成功"
        );
    }

    #[test]
    fn trim_working_set_on_invalid_pid_fails_gracefully() {
        let Err(e) = trim_working_set(0xFFFF_FFF0) else {
            panic!("无效 PID 应失败");
        };
        assert!(matches!(e, FreezeError::OpenFailed(_)));
        assert!(e.to_string().contains("0x"), "应带系统错误码: {e}");
    }
}
