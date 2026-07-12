use std::path::{Path, PathBuf};

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::core::PCWSTR;

use crate::util::to_wide_null;

const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const REG_VALUE_NAME: &str = "Boss Key Application";
pub const TASK_NAME: &str = "BossKeyAutostart";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    TaskScheduler,
    Registry,
}

#[derive(Debug, thiserror::Error)]
pub enum AutostartError {
    #[error("无法确定程序路径")]
    NoExePath,
    #[error("计划任务与注册表方式均失败（可能被安全软件拦截或需要管理员权限）")]
    AllMethodsFailed,
}

pub struct Autostart {
    pub task_name: String,
    pub run_subkey: String,
    pub reg_value_name: String,
    pub exe_path: PathBuf,
}

impl Autostart {
    pub fn standard() -> Result<Self, AutostartError> {
        let exe_path = std::env::current_exe().map_err(|_| AutostartError::NoExePath)?;
        Ok(Self {
            task_name: TASK_NAME.to_string(),
            run_subkey: RUN_SUBKEY.to_string(),
            reg_value_name: REG_VALUE_NAME.to_string(),
            exe_path,
        })
    }

    pub fn status(&self) -> Option<Method> {
        if task_exists(&self.task_name) {
            return Some(Method::TaskScheduler);
        }
        if let Some(value) = registry_read(&self.run_subkey, &self.reg_value_name) {
            let stored = value.trim().trim_matches('"');
            if Path::new(stored) == self.exe_path {
                return Some(Method::Registry);
            }
        }
        None
    }

    pub fn enable(&self) -> Result<Method, AutostartError> {
        let exe = self
            .exe_path
            .to_str()
            .ok_or(AutostartError::NoExePath)?
            .to_string();

        if task_create_highest(&self.task_name, &exe) {
            registry_delete(&self.run_subkey, &self.reg_value_name);
            return Ok(Method::TaskScheduler);
        }

        if registry_write(
            &self.run_subkey,
            &self.reg_value_name,
            &format!("\"{exe}\""),
        ) {
            return Ok(Method::Registry);
        }

        Err(AutostartError::AllMethodsFailed)
    }

    pub fn disable(&self) {
        if task_exists(&self.task_name) {
            task_delete(&self.task_name);
        }
        registry_delete(&self.run_subkey, &self.reg_value_name);
    }
}

fn registry_write(subkey: &str, value_name: &str, data: &str) -> bool {
    let subkey = to_wide_null(subkey);
    let name = to_wide_null(value_name);
    let wide_data = to_wide_null(data);
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(name.as_ptr()),
            REG_SZ.0,
            Some(wide_data.as_ptr() as *const _),
            (wide_data.len() * 2) as u32,
        ) == ERROR_SUCCESS
    }
}

fn registry_read(subkey: &str, value_name: &str) -> Option<String> {
    let subkey = to_wide_null(subkey);
    let name = to_wide_null(value_name);
    let mut buf = vec![0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };
    if result != ERROR_SUCCESS {
        return None;
    }
    let chars = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..chars]))
}

fn registry_delete(subkey: &str, value_name: &str) -> bool {
    let subkey = to_wide_null(subkey);
    let name = to_wide_null(value_name);
    unsafe {
        RegDeleteKeyValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(name.as_ptr()),
        ) == ERROR_SUCCESS
    }
}

fn schtasks(args: &[&str]) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("schtasks")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn task_exists(task_name: &str) -> bool {
    schtasks(&["/Query", "/TN", task_name])
}

fn task_create_highest(task_name: &str, exe: &str) -> bool {
    let tr = format!("\"{exe}\"");
    schtasks(&[
        "/Create", "/F", "/TN", task_name, "/TR", &tr, "/SC", "ONLOGON", "/RL", "HIGHEST",
    ])
}

fn task_delete(task_name: &str) -> bool {
    schtasks(&["/Delete", "/F", "/TN", task_name])
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SUBKEY: &str = r"Software\BossKeyTest\Autostart";

    struct RegCleanup(&'static str);
    impl Drop for RegCleanup {
        fn drop(&mut self) {
            registry_delete(TEST_SUBKEY, self.0);
        }
    }

    struct TaskCleanup(&'static str);
    impl Drop for TaskCleanup {
        fn drop(&mut self) {
            task_delete(self.0);
        }
    }

    #[test]
    fn registry_helpers_round_trip_on_neutral_key() {
        let name = "RegRoundTrip";
        let _guard = RegCleanup(name);

        assert!(registry_read(TEST_SUBKEY, name).is_none());
        assert!(
            registry_write(TEST_SUBKEY, name, "\"C:\\test\\bosskey.exe\""),
            "写普通键应成功（不受启动项防护影响）"
        );
        assert_eq!(
            registry_read(TEST_SUBKEY, name).as_deref(),
            Some("\"C:\\test\\bosskey.exe\"")
        );
        assert!(registry_delete(TEST_SUBKEY, name));
        assert!(registry_read(TEST_SUBKEY, name).is_none());
    }

    #[test]
    fn status_detects_registry_entry_with_and_without_quotes() {
        let name = "StatusQuotes";
        let _guard = RegCleanup(name);

        let auto = Autostart {
            task_name: "BossKeyTest_NoSuchTask".to_string(),
            run_subkey: TEST_SUBKEY.to_string(),
            reg_value_name: name.to_string(),
            exe_path: PathBuf::from("C:\\test\\bosskey.exe"),
        };

        assert_eq!(auto.status(), None);

        registry_write(TEST_SUBKEY, name, "\"C:\\test\\bosskey.exe\"");
        assert_eq!(auto.status(), Some(Method::Registry), "带引号路径应识别");

        registry_write(TEST_SUBKEY, name, "C:\\test\\bosskey.exe");
        assert_eq!(
            auto.status(),
            Some(Method::Registry),
            "旧版不带引号路径应兼容识别"
        );
    }

    #[test]
    fn enable_then_disable_round_trip() {
        let reg_name = "EnableDisable";
        let task_name = "BossKeyTest_EnableDisableTask";
        let _guard1 = RegCleanup(reg_name);
        let _guard2 = TaskCleanup(task_name);

        let auto = Autostart {
            task_name: task_name.to_string(),
            run_subkey: TEST_SUBKEY.to_string(),
            reg_value_name: reg_name.to_string(),
            exe_path: std::env::current_exe().unwrap(),
        };

        let method = auto
            .enable()
            .expect("指向普通键时 enable 应至少通过注册表方式成功");
        assert_eq!(
            auto.status(),
            Some(method),
            "enable 后 status 应报告相同方式"
        );

        auto.disable();
        assert_eq!(auto.status(), None, "disable 后 status 应为 None");
    }

    #[test]
    fn nonexistent_task_is_not_reported() {
        assert!(!task_exists("BossKeyTest_DefinitelyNotExists_42"));
    }
}
