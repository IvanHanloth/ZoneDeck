use std::path::Path;
use std::sync::OnceLock;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};
use windows::core::{PCSTR, s, w};

const PSSUSPEND_EXE: &str = "pssuspend64.exe";

type NtProc = unsafe extern "system" fn(HANDLE) -> i32;

#[derive(Debug, thiserror::Error)]
pub enum FreezeError {
    #[error("ntdll 中未找到 {0}")]
    NtdllUnavailable(&'static str),
    #[error("无法打开进程 PID {0}")]
    OpenFailed(u32),
    #[error("Nt 调用失败，状态码: {0:#x}")]
    NtFailed(i32),
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

fn call_nt(pid: u32, proc: NtProc) -> Result<(), FreezeError> {
    unsafe {
        let handle = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid)
            .map_err(|_| FreezeError::OpenFailed(pid))?;
        let status = proc(handle);
        let _ = CloseHandle(handle);
        if status < 0 {
            return Err(FreezeError::NtFailed(status));
        }
        Ok(())
    }
}

pub fn suspend_process(pid: u32) -> Result<(), FreezeError> {
    static SUSPEND: OnceLock<Option<NtProc>> = OnceLock::new();
    let proc = (*SUSPEND.get_or_init(|| resolve(s!("NtSuspendProcess"))))
        .ok_or(FreezeError::NtdllUnavailable("NtSuspendProcess"))?;
    call_nt(pid, proc)
}

pub fn resume_process(pid: u32) -> Result<(), FreezeError> {
    static RESUME: OnceLock<Option<NtProc>> = OnceLock::new();
    let proc = (*RESUME.get_or_init(|| resolve(s!("NtResumeProcess"))))
        .ok_or(FreezeError::NtdllUnavailable("NtResumeProcess"))?;
    call_nt(pid, proc)
}

pub fn pssuspend_available(exe_dir: &Path) -> bool {
    exe_dir.join(PSSUSPEND_EXE).exists()
}

/// 枚举系统全部进程，返回 `(pid, 父 pid)` 列表。用于把冻结目标展开到整棵子进程树。
/// 失败（快照打不开）时返回空表，调用方据此退化为「只冻结目标自身」。
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

fn run_pssuspend(exe_dir: &Path, args: &[&str]) -> Result<(), FreezeError> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe = exe_dir.join(PSSUSPEND_EXE);
    if !exe.exists() {
        return Err(FreezeError::PssuspendMissing);
    }
    // `-accepteula` 必不可少：首次运行若未接受 EULA，pssuspend 会弹出许可
    // 对话框并阻塞/失败（并写入 HKCU\Software\Sysinternals\PsSuspend）。
    // 缺了它，增强冻结在没接受过 EULA 的机器上永远失败。旗标须在 PID 之前。
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
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
    fn pssuspend_missing_reports_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!pssuspend_available(dir.path()));
        assert!(matches!(
            suspend_enhanced(dir.path(), 1234),
            Err(FreezeError::PssuspendMissing)
        ));
    }
}
