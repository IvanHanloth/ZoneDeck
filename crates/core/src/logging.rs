//! 分级文件日志 + panic 钩子。
//!
//! 按天切割（`ZoneDeck-YYYY-MM-DD.log`）、按天保留，按用户所选的
//! [输出等级](zonedeck_common::config::LOG_LEVELS)过滤。
//!
//! 写入前统一脱敏（见 [`redact_user_dir`]）；调用方不得把窗口标题一类的内容交给日志。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

use windows::Win32::System::SystemInformation::GetLocalTime;
use zonedeck_common::config;

pub const LOG_DIR_NAME: &str = "logs";
/// 日志文件名前缀；面向用户，用品牌大小写。
const LOG_FILE_PREFIX: &str = "ZoneDeck-";
/// 改名（Boss Key → ZoneDeck）前的日志文件前缀，仅用于识别旧文件。
const LEGACY_LOG_FILE_PREFIX: &str = "BossKey-";
const LOG_FILE_SUFFIX: &str = ".log";
/// 用户目录在日志中的替代写法。
const USER_DIR_PLACEHOLDER: &str = "%USERPROFILE%";

/// 日志等级，按严重程度递增排序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

    /// 配置文件里的写法。
    pub fn as_config_str(self) -> &'static str {
        match self {
            Level::Debug => config::LOG_LEVEL_DEBUG,
            Level::Info => config::LOG_LEVEL_INFO,
            Level::Warn => config::LOG_LEVEL_WARN,
            Level::Error => config::LOG_LEVEL_ERROR,
        }
    }

    /// 解析配置里的等级取值，无法识别时回落默认等级。
    pub fn from_config(value: &str) -> Self {
        match config::normalize_log_level(value).as_str() {
            config::LOG_LEVEL_DEBUG => Level::Debug,
            config::LOG_LEVEL_INFO => Level::Info,
            config::LOG_LEVEL_ERROR => Level::Error,
            _ => Level::Warn,
        }
    }
}

/// 会话标记的类型：每次运行各一条，不受输出等级过滤。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marker {
    Start,
    Exit,
}

impl Marker {
    fn as_str(self) -> &'static str {
        match self {
            Marker::Start => "START",
            Marker::Exit => "EXIT",
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
        self.write(level.as_str(), message);
    }

    /// 写一条带标签的记录，落盘前统一脱敏。
    fn write(&self, tag: &str, message: &str) {
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
            tag,
            &redact_user_dir(message, user_dir()),
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

/// 当天日志文件名：`ZoneDeck-YYYY-MM-DD.log`。
fn log_file_name(year: u16, month: u16, day: u16) -> String {
    format!("{LOG_FILE_PREFIX}{year:04}-{month:02}-{day:02}{LOG_FILE_SUFFIX}")
}

/// 从日志文件名解析出年月日；不符合命名规则时返回 `None`。
/// 兼容改名前的旧前缀，保留天数清理与会话回溯因此继续覆盖旧文件。
fn parse_log_date(name: &str) -> Option<(i64, u32, u32)> {
    let date = name
        .strip_prefix(LOG_FILE_PREFIX)
        .or_else(|| name.strip_prefix(LEGACY_LOG_FILE_PREFIX))?
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

fn format_entry(timestamp: &str, tag: &str, message: &str) -> String {
    format!("{timestamp} [{tag}] {message}\n")
}

/// 会话起始行的标志串，形如 `[START]`。
fn start_mark() -> String {
    format!("[{}]", Marker::Start.as_str())
}

/// 摘录里省略中间部分时插入的说明。
const OMISSION_MARK: &str = "……（超出上报长度，已省略中间 {n} 行）";

/// 最近一次会话在该文件中的部分：最后一个 `[START]` 行到文件末尾。
/// 没有会话标记（如本次运行前的旧格式日志）时返回 `None`。
fn session_excerpt(content: &str) -> Option<&str> {
    let mark = start_mark();
    let mut start = None;
    let mut offset = 0;
    for line in content.split_inclusive('\n') {
        if line.contains(&mark) {
            start = Some(offset);
        }
        offset += line.len();
    }
    Some(&content[start?..])
}

/// 把摘录压到 `max_bytes` 以内：保留首行（会话标记，含版本与数据目录）与末尾若干行，
/// 中间以 [`OMISSION_MARK`] 说明省略了多少行。`max_bytes` 为 0 时不做限制。
fn fit_within(excerpt: &str, max_bytes: usize) -> String {
    if max_bytes == 0 || excerpt.len() <= max_bytes {
        return excerpt.to_string();
    }
    let lines: Vec<&str> = excerpt.lines().collect();
    let head = lines.first().copied().unwrap_or_default();
    // 预留首行与省略说明的位置，其余预算留给末尾。
    let reserved = head.len() + OMISSION_MARK.len() + 2;
    let mut tail: Vec<&str> = Vec::new();
    let mut used = 0;
    for line in lines.iter().skip(1).rev() {
        let cost = line.len() + 1;
        if reserved + used + cost > max_bytes {
            break;
        }
        used += cost;
        tail.push(line);
    }
    tail.reverse();
    let omitted = lines.len().saturating_sub(1 + tail.len());
    let mut out = String::with_capacity(max_bytes);
    out.push_str(head);
    out.push('\n');
    if omitted > 0 {
        out.push_str(&OMISSION_MARK.replace("{n}", &omitted.to_string()));
        out.push('\n');
    }
    out.push_str(&tail.join("\n"));
    out
}

/// 按日期从新到旧列出日志文件路径。
fn log_files_newest_first(dir: &std::path::Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(i64, String, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            let (y, m, d) = parse_log_date(&name)?;
            Some((days_from_civil(y, m, d), name, e.path()))
        })
        .collect();
    // 同日期时按文件名降序：改名当天迁来的旧前缀文件（BossKey- < ZoneDeck-）
    // 必须排在新文件之后，否则升级当天的会话摘录会取到旧版本的日志。
    files.sort_by(|a, b| (b.0, &b.1).cmp(&(a.0, &a.1)));
    files.into_iter().map(|(_, _, path)| path).collect()
}

/// 跨零点的会话最多回溯几个日志文件。
const SESSION_LOOKBACK_FILES: usize = 2;

/// 最近一次运行的日志：从该次运行的 `[START]` 起到目前为止，压到 `max_bytes` 以内。
/// 会话跨零点时向前回溯至多 [`SESSION_LOOKBACK_FILES`] 个文件。
pub fn latest_session(dir: &std::path::Path, max_bytes: usize) -> String {
    let files = log_files_newest_first(dir);
    let mut parts: Vec<String> = Vec::new();
    for path in files.iter().take(SESSION_LOOKBACK_FILES) {
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        if let Some(excerpt) = session_excerpt(&content) {
            parts.push(excerpt.to_string());
            return fit_within(&join_parts(parts), max_bytes);
        }
        // 文件里没有起始标记：整个文件都属于更早开始的那次运行。
        parts.push(content);
    }
    fit_within(&join_parts(parts), max_bytes)
}

/// 把片段按从旧到新拼接；入参为从新到旧。
fn join_parts(mut parts: Vec<String>) -> String {
    parts.reverse();
    parts
        .iter()
        .map(|p| p.trim_end_matches('\n'))
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 当前用户目录；取不到（或为空）时不做替换。
fn user_dir() -> &'static str {
    static USER_DIR: OnceLock<String> = OnceLock::new();
    USER_DIR.get_or_init(|| std::env::var("USERPROFILE").unwrap_or_default())
}

/// 把消息里的用户目录换成 [`USER_DIR_PLACEHOLDER`]，避免日志上传时带出用户名。
/// 大小写按 ASCII 规则忽略（Windows 路径的大小写差异只出现在 ASCII 部分）。
fn redact_user_dir(message: &str, user_dir: &str) -> String {
    if user_dir.is_empty() || message.len() < user_dir.len() {
        return message.to_string();
    }
    let bytes = message.as_bytes();
    let needle = user_dir.as_bytes();
    let mut out = String::with_capacity(message.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes.len() - i >= needle.len()
            && bytes[i..i + needle.len()].eq_ignore_ascii_case(needle)
        {
            out.push_str(USER_DIR_PLACEHOLDER);
            i += needle.len();
            continue;
        }
        // 按字符推进，命中位置必定落在字符边界上。
        let ch = message[i..].chars().next().unwrap_or('\u{fffd}');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
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
/// 当前输出等级，存 [`Level`] 的判别序号；默认与配置默认值一致。
static LEVEL: AtomicU8 = AtomicU8::new(Level::Warn as u8);

/// 初始化全局日志；`retention_days == 0` 表示关闭日志。
pub fn init(dir: PathBuf, retention_days: u32, level: Level) {
    set_level(level);
    if retention_days == 0 {
        return;
    }
    let logger = Logger::new(dir, retention_days);
    logger.cleanup();
    let _ = GLOBAL.set(logger);
}

/// 调整输出等级；配置热重载时调用。
pub fn set_level(level: Level) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

pub fn level() -> Level {
    match LEVEL.load(Ordering::Relaxed) {
        v if v == Level::Debug as u8 => Level::Debug,
        v if v == Level::Info as u8 => Level::Info,
        v if v == Level::Error as u8 => Level::Error,
        _ => Level::Warn,
    }
}

/// 该级别是否应输出：低于当前输出等级的一律丢弃。
fn should_emit(level: Level) -> bool {
    level >= self::level()
}

/// 会话起始标记：不受输出等级影响，每次运行一条。
pub fn session_start(message: &str) {
    if let Some(logger) = GLOBAL.get() {
        logger.write(Marker::Start.as_str(), message);
    }
}

/// 会话结束标记：不受输出等级影响。日志末尾没有它即上次未正常退出。
pub fn session_exit(message: &str) {
    if let Some(logger) = GLOBAL.get() {
        logger.write(Marker::Exit.as_str(), message);
    }
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

/// 供 [`crate::log_warn!`] 使用：warn 并附上报错位置。
pub fn warn_at(file: &str, line: u32, message: &str) {
    log(Level::Warn, &format!("{message} ({file}:{line})"));
}

/// 供 [`crate::log_error!`] 使用：error 并附上报错位置。
pub fn error_at(file: &str, line: u32, message: &str) {
    log(Level::Error, &format!("{message} ({file}:{line})"));
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

/// warn 级日志并自动附上调用处的 `文件:行号`。
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logging::warn_at(file!(), line!(), &format!($($arg)*))
    };
}

/// error 级日志并自动附上调用处的 `文件:行号`。
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logging::error_at(file!(), line!(), &format!($($arg)*))
    };
}

/// 格式化 panic 信息为单条日志。
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
        let entry = format_entry("2026-07-13 10:00:00", Level::Warn.as_str(), "热键注册失败");
        assert_eq!(entry, "2026-07-13 10:00:00 [WARN] 热键注册失败\n");
    }

    #[test]
    fn log_file_name_uses_brand_prefix_and_date() {
        assert_eq!(log_file_name(2026, 7, 4), "ZoneDeck-2026-07-04.log");
    }

    #[test]
    fn same_day_new_prefix_sorts_before_legacy() {
        let dir = temp_dir();
        std::fs::write(dir.join("BossKey-2026-08-10.log"), "旧版会话").unwrap();
        std::fs::write(dir.join("ZoneDeck-2026-08-10.log"), "新版会话").unwrap();
        std::fs::write(dir.join("ZoneDeck-2026-08-09.log"), "前一天").unwrap();
        let files = log_files_newest_first(&dir);
        let names: Vec<_> = files
            .iter()
            .filter_map(|p| p.file_name()?.to_str().map(str::to_string))
            .collect();
        assert_eq!(
            names,
            [
                "ZoneDeck-2026-08-10.log",
                "BossKey-2026-08-10.log",
                "ZoneDeck-2026-08-09.log"
            ],
            "同日期时新前缀在前，升级当天的会话摘录才不会取到旧版本日志"
        );
    }

    #[test]
    fn parse_log_date_round_trips_and_rejects_others() {
        assert_eq!(
            parse_log_date("ZoneDeck-2026-07-04.log"),
            Some((2026, 7, 4))
        );
        assert_eq!(
            parse_log_date("BossKey-2026-07-04.log"),
            Some((2026, 7, 4)),
            "改名前的旧日志同样纳入清理与回溯"
        );
        assert_eq!(parse_log_date("recovery.json"), None);
        assert_eq!(parse_log_date("ZoneDeck-2026-07.log"), None);
        assert_eq!(
            parse_log_date("ZoneDeck-2026-13-04.log"),
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
            !is_expired("ZoneDeck-2026-07-14.log", today, 7),
            "今天不应过期"
        );
        assert!(
            !is_expired("ZoneDeck-2026-07-08.log", today, 7),
            "6 天前不应过期"
        );
        assert!(
            is_expired("ZoneDeck-2026-07-07.log", today, 7),
            "7 天前应过期"
        );
        assert!(is_expired("ZoneDeck-2026-06-01.log", today, 7));
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
        fs::write(dir.join("ZoneDeck-2000-01-01.log"), b"old").unwrap();
        let now = unsafe { GetLocalTime() };
        let recent = log_file_name(now.wYear, now.wMonth, now.wDay);
        fs::write(dir.join(&recent), b"recent").unwrap();
        fs::write(dir.join("keep.txt"), b"not a log").unwrap();

        logger.cleanup();

        assert!(
            !dir.join("ZoneDeck-2000-01-01.log").exists(),
            "过期日志应删除"
        );
        assert!(dir.join(&recent).exists(), "当天日志应保留");
        assert!(dir.join("keep.txt").exists(), "非日志文件应保留");
    }

    #[test]
    fn level_orders_by_severity() {
        assert!(Level::Debug < Level::Info);
        assert!(Level::Info < Level::Warn);
        assert!(Level::Warn < Level::Error);
    }

    #[test]
    fn level_parses_config_values() {
        assert_eq!(Level::from_config("debug"), Level::Debug);
        assert_eq!(Level::from_config("INFO"), Level::Info);
        assert_eq!(Level::from_config("warning"), Level::Warn);
        assert_eq!(Level::from_config("error"), Level::Error);
        assert_eq!(
            Level::from_config("胡说"),
            Level::Warn,
            "未知取值回落默认等级"
        );
        assert_eq!(
            Level::from_config(zonedeck_common::config::DEFAULT_LOG_LEVEL),
            Level::Warn,
            "默认等级即「仅记录警告及以上」"
        );
    }

    /// 修改全局等级的测试串行执行，避免相互干扰。
    fn with_level<T>(level: Level, body: impl FnOnce() -> T) -> T {
        static SERIAL: Mutex<()> = Mutex::new(());
        let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let previous = self::level();
        set_level(level);
        let out = body();
        set_level(previous);
        out
    }

    #[test]
    fn default_level_drops_info_and_debug() {
        with_level(Level::Warn, || {
            assert!(!should_emit(Level::Debug));
            assert!(!should_emit(Level::Info), "默认等级下 INFO 不记录");
            assert!(should_emit(Level::Warn));
            assert!(should_emit(Level::Error));
        });
    }

    #[test]
    fn lowering_level_lets_details_through() {
        with_level(Level::Debug, || {
            assert!(should_emit(Level::Debug));
            assert!(should_emit(Level::Info));
        });
        with_level(Level::Error, || {
            assert!(!should_emit(Level::Warn), "只记错误时连警告都丢弃");
            assert!(should_emit(Level::Error));
        });
    }

    #[test]
    fn init_with_zero_retention_disables_logging() {
        let dir = temp_dir();
        init(dir.join("logs"), 0, Level::Warn);
        assert!(!dir.join("logs").exists(), "关闭日志时不应创建目录");
    }

    #[test]
    fn session_markers_bypass_level_filter() {
        let dir = temp_dir();
        let logger = Logger::new(dir.clone(), 7);
        logger.write(Marker::Start.as_str(), "核心启动 9.9.9");
        logger.write(Marker::Exit.as_str(), "核心正常退出");
        let logs: Vec<_> = fs::read_dir(&dir).unwrap().flatten().collect();
        let content = fs::read_to_string(logs[0].path()).unwrap();
        assert!(content.contains("[START] 核心启动 9.9.9"));
        assert!(content.contains("[EXIT] 核心正常退出"));
    }

    #[test]
    fn session_excerpt_starts_at_the_last_start_marker() {
        let content = "\
2026-07-29 10:00:00 [START] 核心启动 3.0.0
2026-07-29 10:00:01 [WARN] 上一次运行的警告
2026-07-29 10:05:00 [EXIT] 核心正常退出
2026-07-29 11:00:00 [START] 核心启动 3.1.0
2026-07-29 11:00:02 [ERROR] 本次运行的错误
";
        let excerpt = session_excerpt(content).expect("应找到会话起始标记");
        assert!(excerpt.starts_with("2026-07-29 11:00:00 [START] 核心启动 3.1.0"));
        assert!(excerpt.contains("本次运行的错误"));
        assert!(!excerpt.contains("上一次运行的警告"), "不应带上更早的运行");
        assert_eq!(session_excerpt("没有任何标记的旧日志\n"), None);
    }

    #[test]
    fn latest_session_spans_midnight_and_skips_older_runs() {
        let dir = temp_dir();
        fs::write(
            dir.join("ZoneDeck-2026-07-28.log"),
            "2026-07-28 08:00:00 [START] 更早的一次运行\n\
             2026-07-28 09:00:00 [EXIT] 核心正常退出\n\
             2026-07-28 23:59:00 [START] 跨零点这次运行\n\
             2026-07-28 23:59:30 [WARN] 零点前的警告\n",
        )
        .unwrap();
        // 当天文件里没有起始标记：本次运行是昨晚开始的。
        fs::write(
            dir.join("ZoneDeck-2026-07-29.log"),
            "2026-07-29 00:00:10 [ERROR] 零点后的错误\n",
        )
        .unwrap();
        fs::write(dir.join("keep.txt"), "不是日志").unwrap();

        let session = latest_session(&dir, 0);
        assert!(session.starts_with("2026-07-28 23:59:00 [START] 跨零点这次运行"));
        assert!(session.contains("零点前的警告"));
        assert!(session.contains("零点后的错误"), "应续上当天的记录");
        assert!(!session.contains("更早的一次运行"));
    }

    #[test]
    fn latest_session_falls_back_when_no_marker_exists() {
        let dir = temp_dir();
        fs::write(
            dir.join("ZoneDeck-2026-07-29.log"),
            "2026-07-29 08:00:00 [WARN] 旧格式日志，没有会话标记\n",
        )
        .unwrap();
        let session = latest_session(&dir, 0);
        assert!(
            session.contains("旧格式日志"),
            "取不到会话标记时仍应上报已有内容"
        );
        assert_eq!(latest_session(&temp_dir(), 0), "", "没有日志文件时为空");
    }

    #[test]
    fn oversized_session_keeps_the_marker_and_the_tail() {
        let mut content = String::from("2026-07-29 10:00:00 [START] 核心启动 3.1.0\n");
        for i in 0..200 {
            content.push_str(&format!("2026-07-29 10:00:{i:02} [DEBUG] 第 {i} 行流水\n"));
        }
        content.push_str("2026-07-29 10:30:00 [ERROR] 最后的错误\n");

        let fitted = fit_within(&content, 600);
        assert!(fitted.len() <= 600, "应压到预算以内: {}", fitted.len());
        assert!(
            fitted.starts_with("2026-07-29 10:00:00 [START] 核心启动 3.1.0"),
            "首行的版本信息须保留"
        );
        assert!(fitted.contains("最后的错误"), "末尾的现场须保留");
        assert!(fitted.contains("已省略中间"), "省略须写明");
        assert_eq!(fit_within("短内容", 600), "短内容", "不超预算时原样返回");
    }

    #[test]
    fn user_dir_is_redacted_before_writing() {
        assert_eq!(
            redact_user_dir(
                r"配置: C:\Users\张三\AppData\Roaming\ZoneDeck",
                r"C:\Users\张三"
            ),
            r"配置: %USERPROFILE%\AppData\Roaming\ZoneDeck"
        );
        assert_eq!(
            redact_user_dir(r"c:\users\Ivan\logs", r"C:\Users\Ivan"),
            r"%USERPROFILE%\logs",
            "路径大小写差异不应漏掉"
        );
        assert_eq!(
            redact_user_dir("无关内容", r"C:\Users\Ivan"),
            "无关内容",
            "不含用户目录的消息原样保留"
        );
        assert_eq!(redact_user_dir("短", ""), "短", "取不到用户目录时不替换");
    }

    #[test]
    fn logger_redacts_user_dir_on_disk() {
        let dir = temp_dir();
        let logger = Logger::new(dir.clone(), 7);
        // 脱敏取真实环境变量，此处验证写盘路径上已调用脱敏。
        let profile = user_dir().to_string();
        if profile.is_empty() {
            return;
        }
        logger.log(Level::Error, &format!("读取失败: {profile}\\config.json"));
        let logs: Vec<_> = fs::read_dir(&dir).unwrap().flatten().collect();
        let content = fs::read_to_string(logs[0].path()).unwrap();
        assert!(
            !content.contains(&profile),
            "日志不应落下用户目录: {content}"
        );
        assert!(content.contains("%USERPROFILE%\\config.json"));
    }

    #[test]
    fn warn_at_appends_source_location() {
        let dir = temp_dir();
        let logger = Logger::new(dir.clone(), 7);
        // 直接验证格式化结果（warn_at 走全局 logger，测试里用本地实例复刻其格式）。
        logger.log(
            Level::Warn,
            &format!("{} ({}:{})", "落盘失败", "agent.rs", 42),
        );
        let logs: Vec<_> = fs::read_dir(&dir).unwrap().flatten().collect();
        let content = fs::read_to_string(logs[0].path()).unwrap();
        assert!(content.contains("[WARN] 落盘失败 (agent.rs:42)"));
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
