//! 分级文件日志 + panic 钩子。
//!
//! 按天切割（`BossKey-YYYY-MM-DD.log`）、按天保留、分级输出（release 丢弃 DEBUG）。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use windows::Win32::System::SystemInformation::GetLocalTime;

/// 日志目录名（位于 exe 同目录下）。
pub const LOG_DIR_NAME: &str = "logs";
/// 日志文件名前缀（面向用户，使用品牌大小写）。
const LOG_FILE_PREFIX: &str = "BossKey-";
const LOG_FILE_SUFFIX: &str = ".log";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

pub struct Logger {
    dir: PathBuf,
    retention_days: u32,
    lock: Mutex<()>,
}

impl Logger {
    pub fn new(dir: PathBuf, retention_days: u32) -> Self {
        Self {
            dir,
            retention_days,
            lock: Mutex::new(()),
        }
    }

    pub fn log(&self, level: Level, message: &str) {
        let now = unsafe { GetLocalTime() };
        let entry = format_entry(
            &format_timestamp(
                now.wYear,
                now.wMonth,
                now.wDay,
                now.wHour,
                now.wMinute,
                now.wSecond,
            ),
            level,
            message,
        );
        let path = self
            .dir
            .join(log_file_name(now.wYear, now.wMonth, now.wDay));
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let _ = fs::create_dir_all(&self.dir);
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = file.write_all(entry.as_bytes());
        }
    }

    /// 启动时清理超过保留天数的旧日志文件；`retention_days == 0` 时不清理。
    pub fn cleanup(&self) {
        if self.retention_days == 0 {
            return;
        }
        let now = unsafe { GetLocalTime() };
        let today = days_from_civil(now.wYear as i64, now.wMonth as u32, now.wDay as u32);
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if is_expired(name, today, self.retention_days) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

/// 当天日志文件名：`BossKey-YYYY-MM-DD.log`。
fn log_file_name(year: u16, month: u16, day: u16) -> String {
    format!("{LOG_FILE_PREFIX}{year:04}-{month:02}-{day:02}{LOG_FILE_SUFFIX}")
}

/// 从日志文件名解析出年月日；不符合命名规则时返回 `None`。
fn parse_log_date(name: &str) -> Option<(i64, u32, u32)> {
    let date = name
        .strip_prefix(LOG_FILE_PREFIX)?
        .strip_suffix(LOG_FILE_SUFFIX)?;
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

/// 将公历年月日换算为连续日序号（1970-01-01 为 0）。Howard Hinnant 算法。
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as i64 + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// 某个日志文件是否已过期（早于 `today` 达 `retention_days` 天或以上）。
fn is_expired(name: &str, today: i64, retention_days: u32) -> bool {
    let Some((y, m, d)) = parse_log_date(name) else {
        return false; // 非日志文件不动
    };
    let age = today - days_from_civil(y, m, d);
    age >= retention_days as i64
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

static GLOBAL: OnceLock<Logger> = OnceLock::new();

/// 初始化全局日志；`retention_days == 0` 表示关闭日志。
pub fn init(dir: PathBuf, retention_days: u32) {
    if retention_days == 0 {
        return;
    }
    let logger = Logger::new(dir, retention_days);
    logger.cleanup();
    let _ = GLOBAL.set(logger);
}

/// 该级别是否应输出：生产（release）构建丢弃 DEBUG。
fn should_emit(level: Level) -> bool {
    level != Level::Debug || cfg!(debug_assertions)
}

pub fn debug(message: &str) {
    log(Level::Debug, message);
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
    if !should_emit(level) {
        return;
    }
    if matches!(level, Level::Warn | Level::Error) {
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
    use std::path::Path;

    fn temp_dir() -> PathBuf {
        tempfile::tempdir().expect("创建临时目录失败").keep()
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
    fn log_file_name_uses_brand_prefix_and_date() {
        assert_eq!(log_file_name(2026, 7, 4), "BossKey-2026-07-04.log");
    }

    #[test]
    fn parse_log_date_round_trips_and_rejects_others() {
        assert_eq!(parse_log_date("BossKey-2026-07-04.log"), Some((2026, 7, 4)));
        assert_eq!(parse_log_date("recovery.json"), None);
        assert_eq!(parse_log_date("BossKey-2026-07.log"), None);
        assert_eq!(
            parse_log_date("BossKey-2026-13-04.log"),
            None,
            "非法月份应拒绝"
        );
        assert_eq!(parse_log_date("other-2026-07-04.log"), None);
    }

    #[test]
    fn days_from_civil_matches_known_epoch() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1970, 1, 2), 1);
        assert_eq!(days_from_civil(2000, 1, 1), 10957);
    }

    #[test]
    fn expiry_respects_retention_window() {
        let today = days_from_civil(2026, 7, 14);
        assert!(
            !is_expired("BossKey-2026-07-14.log", today, 7),
            "今天不应过期"
        );
        assert!(
            !is_expired("BossKey-2026-07-08.log", today, 7),
            "6 天前不应过期"
        );
        assert!(
            is_expired("BossKey-2026-07-07.log", today, 7),
            "7 天前应过期"
        );
        assert!(is_expired("BossKey-2026-06-01.log", today, 7));
        assert!(
            !is_expired("config.json", today, 7),
            "非日志文件不应被判过期"
        );
    }

    #[test]
    fn logger_writes_into_todays_dated_file() {
        let dir = temp_dir();
        let logger = Logger::new(dir.clone(), 7);
        logger.log(Level::Info, "第一条");
        logger.log(Level::Error, "第二条");

        let logs: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| parse_log_date(e.file_name().to_str().unwrap()).is_some())
            .collect();
        assert_eq!(logs.len(), 1, "应写入单个当天日志文件");
        let content = fs::read_to_string(logs[0].path()).unwrap();
        assert!(content.contains("[INFO] 第一条"));
        assert!(content.contains("[ERROR] 第二条"));
    }

    #[test]
    fn cleanup_removes_only_expired_logs() {
        let dir = temp_dir();
        let logger = Logger::new(dir.clone(), 7);
        fs::write(dir.join("BossKey-2000-01-01.log"), b"old").unwrap();
        let now = unsafe { GetLocalTime() };
        let recent = log_file_name(now.wYear, now.wMonth, now.wDay);
        fs::write(dir.join(&recent), b"recent").unwrap();
        fs::write(dir.join("keep.txt"), b"not a log").unwrap();

        logger.cleanup();

        assert!(
            !dir.join("BossKey-2000-01-01.log").exists(),
            "过期日志应删除"
        );
        assert!(dir.join(&recent).exists(), "当天日志应保留");
        assert!(dir.join("keep.txt").exists(), "非日志文件应保留");
    }

    #[test]
    fn debug_is_only_emitted_in_dev_builds() {
        assert!(should_emit(Level::Info));
        assert!(should_emit(Level::Warn));
        assert_eq!(should_emit(Level::Debug), cfg!(debug_assertions));
    }

    #[test]
    fn init_with_zero_retention_disables_logging() {
        let dir = temp_dir();
        init(dir.join("logs"), 0);
        assert!(!dir.join("logs").exists(), "关闭日志时不应创建目录");
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
    fn rotated_files_stay_within_directory() {
        let dir = temp_dir();
        let logger = Logger::new(dir.clone(), 7);
        logger.log(Level::Debug, "调试信息");
        let any = fs::read_dir(&dir).unwrap().flatten().next();
        assert!(any.is_some(), "应至少写出一个文件");
        assert!(
            any.unwrap().path().starts_with(Path::new(&dir)),
            "日志文件应位于日志目录内"
        );
    }
}
