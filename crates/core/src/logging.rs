//! 崩溃日志：轻量文件日志 + panic 钩子。
//!
//! 设计目标：
//! - 常驻核心崩溃后能从日志文件（`bosskey.log`）定位原因；
//! - 单文件 + 大小上限轮转（超限时改名为 `.old`），不会无限膨胀；
//! - 不引入外部日志框架，保持核心二进制极小。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use windows::Win32::System::SystemInformation::GetLocalTime;

/// 默认日志大小上限：超过后当前日志改名为 `<name>.old`，重新开始写。
pub const DEFAULT_MAX_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

pub struct Logger {
    path: PathBuf,
    max_bytes: u64,
    lock: Mutex<()>,
}

impl Logger {
    pub fn new(path: PathBuf, max_bytes: u64) -> Self {
        Self {
            path,
            max_bytes,
            lock: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn log(&self, level: Level, message: &str) {
        let entry = format_entry(&local_timestamp(), level, message);
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        self.rotate_if_needed(entry.len() as u64);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = file.write_all(entry.as_bytes());
        }
    }

    /// 当前日志加上新条目会超过上限时，将其轮转为 `<文件名>.old`（覆盖旧备份）。
    fn rotate_if_needed(&self, incoming: u64) {
        let Ok(meta) = fs::metadata(&self.path) else {
            return;
        };
        if meta.len() + incoming <= self.max_bytes {
            return;
        }
        let backup = rotated_path(&self.path);
        let _ = fs::remove_file(&backup);
        let _ = fs::rename(&self.path, &backup);
    }
}

/// 轮转备份文件路径：`bosskey.log` → `bosskey.log.old`。
fn rotated_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".old");
    PathBuf::from(name)
}

fn format_entry(timestamp: &str, level: Level, message: &str) -> String {
    format!("{timestamp} [{}] {message}\n", level.as_str())
}

fn format_timestamp(
    year: u16,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
) -> String {
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

fn local_timestamp() -> String {
    let t = unsafe { GetLocalTime() };
    format_timestamp(t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
}

// ---- 全局日志入口 ----

static GLOBAL: OnceLock<Logger> = OnceLock::new();

/// 初始化全局日志（仅首次生效）。通常传 exe 同目录下的 `bosskey.log`。
pub fn init(path: PathBuf) {
    let _ = GLOBAL.set(Logger::new(path, DEFAULT_MAX_BYTES));
}

pub fn info(message: &str) {
    log(Level::Info, message);
}

pub fn warn(message: &str) {
    log(Level::Warn, message);
}

pub fn error(message: &str) {
    log(Level::Error, message);
}

fn log(level: Level, message: &str) {
    if level != Level::Info {
        eprintln!("[{}] {message}", level.as_str());
    }
    if let Some(logger) = GLOBAL.get() {
        logger.log(level, message);
    }
}

/// 格式化 panic 信息为单条日志（便于单元测试）。
fn format_panic(message: &str, location: Option<&str>) -> String {
    match location {
        Some(loc) => format!("程序发生崩溃 (panic): {message} @ {loc}"),
        None => format!("程序发生崩溃 (panic): {message}"),
    }
}

/// 安装 panic 钩子：崩溃信息写入全局日志后再走默认钩子。
/// release 下 `panic = "abort"`，钩子执行完后进程以非零码退出，
/// 由计划任务的失败重启（RestartOnFailure）负责拉活。
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "未知 panic 载荷".to_string()
        };
        let location = info.location().map(|l| l.to_string());
        error(&format_panic(&message, location.as_deref()));
        default_hook(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log(name: &str) -> PathBuf {
        tempfile::tempdir()
            .expect("创建临时目录失败")
            .keep()
            .join(name)
    }

    #[test]
    fn timestamp_is_zero_padded() {
        assert_eq!(format_timestamp(2026, 7, 3, 9, 5, 8), "2026-07-03 09:05:08");
    }

    #[test]
    fn entry_contains_level_and_message() {
        let entry = format_entry("2026-07-13 10:00:00", Level::Warn, "热键注册失败");
        assert_eq!(entry, "2026-07-13 10:00:00 [WARN] 热键注册失败\n");
    }

    #[test]
    fn logger_appends_entries_in_order() {
        let path = temp_log("bosskey.log");
        let logger = Logger::new(path.clone(), DEFAULT_MAX_BYTES);
        logger.log(Level::Info, "第一条");
        logger.log(Level::Error, "第二条");

        let content = fs::read_to_string(&path).expect("日志文件应存在");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("[INFO] 第一条"));
        assert!(lines[1].contains("[ERROR] 第二条"));
    }

    #[test]
    fn logger_rotates_to_old_file_when_over_limit() {
        let path = temp_log("bosskey.log");
        let logger = Logger::new(path.clone(), 64);
        logger.log(Level::Info, "旧日志内容 aaaaaaaaaaaaaaaaaaaaaaaa");
        logger.log(Level::Info, "新日志内容");

        let backup = rotated_path(&path);
        let old = fs::read_to_string(&backup).expect("轮转后应存在 .old 备份");
        let new = fs::read_to_string(&path).expect("轮转后应重新开始写主文件");
        assert!(old.contains("旧日志内容"));
        assert!(new.contains("新日志内容"));
        assert!(!new.contains("旧日志内容"));
    }

    #[test]
    fn rotated_path_appends_old_suffix() {
        assert_eq!(
            rotated_path(Path::new("C:\\app\\bosskey.log")),
            PathBuf::from("C:\\app\\bosskey.log.old")
        );
    }

    #[test]
    fn panic_message_includes_location_when_available() {
        assert_eq!(
            format_panic("index out of bounds", Some("agent.rs:42:1")),
            "程序发生崩溃 (panic): index out of bounds @ agent.rs:42:1"
        );
        assert_eq!(format_panic("boom", None), "程序发生崩溃 (panic): boom");
    }

    #[test]
    fn panic_hook_writes_crash_to_global_log() {
        // 全局 Logger 进程内只初始化一次，本测试是唯一使用全局入口的用例。
        let path = temp_log("bosskey.log");
        init(path.clone());
        install_panic_hook();

        let result = std::panic::catch_unwind(|| panic!("测试崩溃"));
        assert!(result.is_err());

        let content = fs::read_to_string(&path).expect("崩溃后日志应存在");
        assert!(content.contains("[ERROR] 程序发生崩溃 (panic): 测试崩溃"));
        assert!(content.contains("logging.rs"), "应记录崩溃位置");
    }
}
