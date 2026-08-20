use std::path::{Path, PathBuf};

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::core::PCWSTR;

use crate::util::to_wide_null;

const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const REG_VALUE_NAME: &str = "ZoneDeck Application";
pub const TASK_NAME: &str = "ZoneDeckAutostart";
/// 改名前注册的自启名称，仅用于清理旧残留。
pub const LEGACY_REG_VALUE_NAME: &str = "Boss Key Application";
pub const LEGACY_TASK_NAME: &str = "BossKeyAutostart";

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

    /// 针对指定 exe 路径构造；查询状态时须传核心 exe 路径。
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

    /// 开启自启：`admin=true` 注册最高权限计划任务，`false` 用普通权限。
    /// 计划任务失败时回退注册表启动项（始终普通权限）。
    pub fn enable(&self, admin: bool) -> Result<Method, AutostartError> {
        let exe = self
            .exe_path
            .to_str()
            .ok_or(AutostartError::NoExePath)?
            .to_string();

        let level = if admin {
            RunLevel::Highest
        } else {
            RunLevel::Least
        };
        if task_create(&self.task_name, &exe, level) {
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

/// 安装器在清理旧自启前写下的迁移标记（见 ZoneDeck.iss），核心凭此得知用户
/// 此前开着自启。
pub const MIGRATION_MARKER_SUBKEY: &str = r"Software\ZoneDeck";
pub const MIGRATION_MARKER_VALUE: &str = "MigrateAutostart";

/// [`migrate_legacy`] 的结果。
pub enum LegacyMigration {
    /// 未发现旧残留或迁移标记。
    NotNeeded,
    /// 新名称已注册（本次注册或此前已注册），旧残留与标记已清理。
    Done,
    /// 新名称注册失败；保留旧残留与标记，下次启动重试。
    Failed(AutostartError),
}

/// 自启的品牌迁移：发现旧名称残留（[`LEGACY_TASK_NAME`]、[`LEGACY_REG_VALUE_NAME`]）
/// 或安装器迁移标记时，先确保自启已在新名称下注册，成功后再清理旧残留与标记。
/// 注册失败时保留信号下次启动重试。
pub fn migrate_legacy(admin: bool) -> LegacyMigration {
    let legacy = LegacySignals {
        run_subkey: RUN_SUBKEY,
        run_value_name: LEGACY_REG_VALUE_NAME,
        task_name: LEGACY_TASK_NAME,
        marker_subkey: MIGRATION_MARKER_SUBKEY,
    };
    if !legacy.present() {
        return LegacyMigration::NotNeeded;
    }
    let auto = match Autostart::standard() {
        Ok(auto) => auto,
        Err(e) => return LegacyMigration::Failed(e),
    };
    if auto.status().is_none()
        && let Err(e) = auto.enable(admin)
    {
        return LegacyMigration::Failed(e);
    }
    legacy.clear();
    LegacyMigration::Done
}

/// 旧名称自启残留与安装器标记的位置，供测试注入。
struct LegacySignals {
    run_subkey: &'static str,
    run_value_name: &'static str,
    task_name: &'static str,
    marker_subkey: &'static str,
}

impl LegacySignals {
    fn present(&self) -> bool {
        registry_read(self.run_subkey, self.run_value_name).is_some()
            || registry_read(self.marker_subkey, MIGRATION_MARKER_VALUE).is_some()
            || task_exists(self.task_name)
    }

    /// 清除残留与标记；无权删除时记 warn 保留，待下次以管理员身份运行时再清。
    fn clear(&self) {
        registry_delete(self.run_subkey, self.run_value_name);
        registry_delete(self.marker_subkey, MIGRATION_MARKER_VALUE);
        if task_exists(self.task_name) && !task_delete(self.task_name) {
            crate::log_warn!(
                "旧计划任务 {} 删除失败（可能需要管理员权限），暂予保留",
                self.task_name
            );
        }
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
    Least,
}

impl RunLevel {
    fn as_xml(self) -> &'static str {
        match self {
            RunLevel::Highest => "HighestAvailable",
            RunLevel::Least => "LeastPrivilege",
        }
    }

    /// 命令行回退时 `schtasks /RL` 的取值。
    fn as_schtasks(self) -> &'static str {
        match self {
            RunLevel::Highest => "HIGHEST",
            RunLevel::Least => "LIMITED",
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
fn task_xml(exe: &str, user_id: &str, run_level: RunLevel) -> String {
    let exe = xml_escape(exe);
    let user = xml_escape(user_id);
    let level = run_level.as_xml();
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>ZoneDeck 开机自启（崩溃后自动重启）</Description>
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

/// 将 XML 以 UTF-16LE（带 BOM）写入文件，schtasks /XML 的规范编码。
fn write_task_xml(path: &Path, xml: &str) -> std::io::Result<()> {
    let mut bytes: Vec<u8> = vec![0xFF, 0xFE];
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(path, bytes)
}

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

fn task_create(task_name: &str, exe: &str, level: RunLevel) -> bool {
    // 优先 XML 方式，带失败自动重启。
    if let Some(user) = current_user()
        && task_create_from_xml(task_name, &task_xml(exe, &user, level))
    {
        return true;
    }
    // 回退到老式命令行注册。
    let tr = format!("\"{exe}\"");
    schtasks(&[
        "/Create",
        "/F",
        "/TN",
        task_name,
        "/TR",
        &tr,
        "/SC",
        "ONLOGON",
        "/RL",
        level.as_schtasks(),
    ])
}

fn task_delete(task_name: &str) -> bool {
    schtasks(&["/Delete", "/F", "/TN", task_name])
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SUBKEY: &str = r"Software\ZoneDeckTest\Autostart";

    #[test]
    fn legacy_signals_detect_and_clear_registry_and_marker() {
        let signals = LegacySignals {
            run_subkey: TEST_SUBKEY,
            run_value_name: "ZoneDeckTest_LegacyRunValue",
            task_name: "ZoneDeckTest_NoSuchLegacyTask_77",
            marker_subkey: r"Software\ZoneDeckTest\Marker",
        };
        assert!(!signals.present(), "初始不应报告残留");

        registry_write(
            TEST_SUBKEY,
            signals.run_value_name,
            "\"C:\\test\\legacy.exe\"",
        );
        assert!(signals.present(), "存在旧 Run 值即视为有残留");
        signals.clear();
        assert!(!signals.present(), "清理后不再报告残留");
        assert_eq!(registry_read(TEST_SUBKEY, signals.run_value_name), None);

        registry_write(signals.marker_subkey, MIGRATION_MARKER_VALUE, "1");
        assert!(signals.present(), "安装器迁移标记同样视为有残留");
        signals.clear();
        assert_eq!(
            registry_read(signals.marker_subkey, MIGRATION_MARKER_VALUE),
            None,
            "标记应被清除"
        );
    }

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
            registry_write(TEST_SUBKEY, name, "\"C:\\test\\zonedeck.exe\""),
            "写普通键应成功（不受启动项防护影响）"
        );
        assert_eq!(
            registry_read(TEST_SUBKEY, name).as_deref(),
            Some("\"C:\\test\\zonedeck.exe\"")
        );
        assert!(registry_delete(TEST_SUBKEY, name));
        assert!(registry_read(TEST_SUBKEY, name).is_none());
    }

    #[test]
    fn status_detects_registry_entry_with_and_without_quotes() {
        let name = "StatusQuotes";
        let _guard = RegCleanup(name);

        let auto = Autostart {
            task_name: "ZoneDeckTest_NoSuchTask".to_string(),
            run_subkey: TEST_SUBKEY.to_string(),
            reg_value_name: name.to_string(),
            exe_path: PathBuf::from("C:\\test\\zonedeck.exe"),
        };

        assert_eq!(auto.status(), None);

        registry_write(TEST_SUBKEY, name, "\"C:\\test\\zonedeck.exe\"");
        assert_eq!(auto.status(), Some(Method::Registry), "带引号路径应识别");

        registry_write(TEST_SUBKEY, name, "C:\\test\\zonedeck.exe");
        assert_eq!(
            auto.status(),
            Some(Method::Registry),
            "旧版不带引号路径应兼容识别"
        );
    }

    #[test]
    fn enable_then_disable_round_trip() {
        let reg_name = "EnableDisable";
        let task_name = "ZoneDeckTest_EnableDisableTask";
        let _guard1 = RegCleanup(reg_name);
        let _guard2 = TaskCleanup(task_name);

        let auto = Autostart {
            task_name: task_name.to_string(),
            run_subkey: TEST_SUBKEY.to_string(),
            reg_value_name: reg_name.to_string(),
            exe_path: std::env::current_exe().unwrap(),
        };

        let method = auto
            .enable(false)
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
        assert!(!task_exists("ZoneDeckTest_DefinitelyNotExists_42"));
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
            "C:\\Tools & Games\\zonedeck-core.exe",
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
            xml.contains("<Command>C:\\Tools &amp; Games\\zonedeck-core.exe</Command>"),
            "路径中的特殊字符应被转义"
        );
        assert!(xml.contains("<UserId>DESKTOP\\ivan</UserId>"));
        assert!(xml.contains("<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"));
    }

    #[test]
    fn task_xml_least_privilege_uses_least_run_level() {
        let xml = task_xml("C:\\app.exe", "DESKTOP\\ivan", RunLevel::Least);
        assert!(
            xml.contains("<RunLevel>LeastPrivilege</RunLevel>"),
            "普通权限应写 LeastPrivilege"
        );
        assert!(
            !xml.contains("HighestAvailable"),
            "普通权限不应出现最高权限标记"
        );
    }

    #[test]
    fn run_level_maps_to_schtasks_flags() {
        assert_eq!(RunLevel::Highest.as_schtasks(), "HIGHEST");
        assert_eq!(RunLevel::Least.as_schtasks(), "LIMITED");
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
    #[ignore = "需要管理员权限，非提权环境下会失败"]
    fn xml_task_registers_and_deletes_via_schtasks() {
        // 用 LeastPrivilege 验证 XML 能被 schtasks 接受；HighestAvailable 需要管理员。
        let task_name = "ZoneDeckTest_XmlRegistration";
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
