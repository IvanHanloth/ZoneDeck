use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::PCWSTR;

use crate::util::to_wide_null;

pub struct SingleInstance {
    handle: Option<HANDLE>,
    already_running: bool,
}

impl SingleInstance {
    pub fn acquire(name: &str) -> Self {
        let wide = to_wide_null(name);
        unsafe {
            match CreateMutexW(None, false, PCWSTR(wide.as_ptr())) {
                Ok(handle) => {
                    let already_running = GetLastError() == ERROR_ALREADY_EXISTS;
                    SingleInstance {
                        handle: Some(handle),
                        already_running,
                    }
                }
                Err(_) => SingleInstance {
                    handle: None,
                    already_running: false,
                },
            }
        }
    }

    /// 在超时时间内反复尝试获取单实例，等待上一个实例退出后接管。
    pub fn acquire_waiting(name: &str, timeout: std::time::Duration) -> Self {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let instance = Self::acquire(name);
            if !instance.already_running() || std::time::Instant::now() >= deadline {
                return instance;
            }
            drop(instance);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    pub fn already_running(&self) -> bool {
        self.already_running
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            unsafe {
                let _ = CloseHandle(handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_detects_running_instance() {
        let name = "BossKey_UnitTest_SingleInstance_9f3a";
        let first = SingleInstance::acquire(name);
        assert!(!first.already_running(), "首个实例不应报告已在运行");

        let second = SingleInstance::acquire(name);
        assert!(second.already_running(), "第二个实例应检测到已在运行");

        drop(first);
        drop(second);

        let third = SingleInstance::acquire(name);
        assert!(
            !third.already_running(),
            "前两个实例释放后，新实例不应报告已在运行"
        );
    }
}
