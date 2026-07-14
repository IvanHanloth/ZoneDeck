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
        Ok(Self::for_exe(exe_path))
    }

    /// 针对指定 exe 路径构造。配置程序查询自启状态时须传**核心** exe 路径，
    /// 与核心 enable 时写入的路径一致，否则状态永远比对不上。
    pub fn for_exe(exe_path: PathBuf) -> Self {
        Self {
            task_name: TASK_NAME.to_string(),
            run_subkey: RUN_SUBKEY.to_string(),
            reg_value_name: REG_VALUE_NAME.to_string(),
            exe_path,
        }
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

/// 任务权限级别（XML `<RunLevel>` 取值）。
#[derive(Debug, Clone, Copy)]
enum RunLevel {
    Highest,
    /// 仅测试使用：非管理员环境下验证 XML 能被 schtasks 接受。
    #[cfg_attr(not(test), allow(dead_code))]
    Least,
}

impl RunLevel {
    fn as_xml(self) -> &'static str {
        match self {
            RunLevel::Highest => "HighestAvailable",
            RunLevel::Least => "LeastPrivilege",
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// 生成计划任务定义 XML：登录触发 + 失败自动重启（看门狗）。
///
/// 关键设置：
/// - `RestartOnFailure`：核心崩溃（非零退出码）后 1 分钟内自动重启，最多 3 次；
/// - `ExecutionTimeLimit PT0S`：不限制运行时长（默认 3 天后会被强杀）；
/// - `MultipleInstancesPolicy IgnoreNew`：配合命名互斥避免重复实例。
fn task_xml(exe: &str, user_id: &str, run_level: RunLevel) -> String {
    let exe = xml_escape(exe);
    let user = xml_escape(user_id);
    let level = run_level.as_xml();
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Boss Key 开机自启（崩溃后自动重启）</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>{level}</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>3</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
    </Exec>
  </Actions>
</Task>
"#
    )
}

/// 当前登录用户（DOMAIN\name），用于任务 XML 的 Principal。
fn current_user() -> Option<String> {
    let name = std::env::var("USERNAME").ok()?;
    match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => Some(format!("{domain}\\{name}")),
        _ => Some(name),
    }
}

/// 将 XML 以 UTF-16LE（带 BOM）写入文件——schtasks /XML 的规范编码。
fn write_task_xml(path: &Path, xml: &str) -> std::io::Result<()> {
    let mut bytes: Vec<u8> = vec![0xFF, 0xFE];
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(path, bytes)
}

/// 通过 XML 注册计划任务，成功返回 true。
fn task_create_from_xml(task_name: &str, xml: &str) -> bool {
    let path = std::env::temp_dir().join(format!("{task_name}.xml"));
    if write_task_xml(&path, xml).is_err() {
        return false;
    }
    let Some(path_str) = path.to_str().map(str::to_string) else {
        return false;
    };
    let created = schtasks(&["/Create", "/F", "/TN", task_name, "/XML", &path_str]);
    let _ = std::fs::remove_file(&path);
    created
}

fn task_create_highest(task_name: &str, exe: &str) -> bool {
    // 优先 XML 方式（带失败自动重启的看门狗设置）。
    if let Some(user) = current_user()
        && task_create_from_xml(task_name, &task_xml(exe, &user, RunLevel::Highest))
    {
        return true;
    }
    // 回退：老式命令行注册（无 RestartOnFailure，但仍是最高权限登录自启）。
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

    #[test]
    fn xml_escape_covers_all_special_characters() {
        assert_eq!(
            xml_escape(r#"A&B <"路径'>"#),
            "A&amp;B &lt;&quot;路径&apos;&gt;"
        );
        assert_eq!(xml_escape("C:\\普通路径\\app.exe"), "C:\\普通路径\\app.exe");
    }

    #[test]
    fn task_xml_contains_watchdog_and_run_level_settings() {
        let xml = task_xml(
            "C:\\Tools & Games\\bosskey-core.exe",
            "DESKTOP\\ivan",
            RunLevel::Highest,
        );
        assert!(xml.contains("<RestartOnFailure>"), "应包含失败自动重启");
        assert!(xml.contains("<Interval>PT1M</Interval>"));
        assert!(xml.contains("<Count>3</Count>"));
        assert!(
            xml.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"),
            "应取消默认 3 天运行时长限制"
        );
        assert!(xml.contains("<RunLevel>HighestAvailable</RunLevel>"));
        assert!(
            xml.contains("<Command>C:\\Tools &amp; Games\\bosskey-core.exe</Command>"),
            "路径中的特殊字符应被转义"
        );
        assert!(xml.contains("<UserId>DESKTOP\\ivan</UserId>"));
        assert!(xml.contains("<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"));
    }

    #[test]
    fn task_xml_file_is_utf16le_with_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("task.xml");
        write_task_xml(&path, "<Task>测试</Task>").unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..2], &[0xFF, 0xFE], "应带 UTF-16LE BOM");
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(String::from_utf16(&units).unwrap(), "<Task>测试</Task>");
    }

    #[test]
    fn xml_task_registers_and_deletes_via_schtasks() {
        // 用 LeastPrivilege 验证 XML 能被 schtasks 接受（HighestAvailable 需要管理员，
        // 在非提权测试环境会失败，那是权限问题而非 XML 问题）。
        let task_name = "BossKeyTest_XmlRegistration";
        let _guard = TaskCleanup(task_name);

        let exe = std::env::current_exe().unwrap();
        let user = current_user().expect("测试环境应能取到当前用户");
        let xml = task_xml(exe.to_str().unwrap(), &user, RunLevel::Least);

        assert!(
            task_create_from_xml(task_name, &xml),
            "生成的任务 XML 应能被 schtasks /Create /XML 接受"
        );
        assert!(task_exists(task_name), "注册后应能查询到任务");
    }
}
