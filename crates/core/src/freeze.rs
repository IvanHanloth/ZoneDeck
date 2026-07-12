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
    #[error("pssuspend64 执行失败")]
    PssuspendFailed,
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

fn run_pssuspend(exe_dir: &Path, args: &[&str]) -> Result<(), FreezeError> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe = exe_dir.join(PSSUSPEND_EXE);
    if !exe.exists() {
        return Err(FreezeError::PssuspendMissing);
    }
    let ok = std::process::Command::new(exe)
        .args(args)
        .arg("-nobanner")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(FreezeError::PssuspendFailed)
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
