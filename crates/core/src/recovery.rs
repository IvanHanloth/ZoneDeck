//! 崩溃恢复：隐藏状态落盘，核心异常退出后下次启动自动找回窗口。
//!
//! 写入在隐藏动作执行前发生（意图先行），走 tmp + rename 原子替换；
//! 快照带开机时刻与进程创建时刻，跨重启后失效的快照在加载侧被丢弃。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::hide::Target;
use crate::log_warn;

/// 恢复文件名（与 config.json 同目录）。
pub const RECOVERY_FILE_NAME: &str = "recovery.json";

/// 当前快照格式版本。旧版（无该字段，缺省 0）不含身份信息，加载时按不可信丢弃。
pub const SCHEMA_CURRENT: u32 = 1;

/// 判定「同一次开机」的容差（GetTickCount64 精度约 10–16ms，另留时钟漂移余量）。
const BOOT_TOLERANCE_MS: i64 = 5_000;

/// 带创建时刻的进程记录（PID 会被系统回收复用，须配合创建时刻标识进程）。
/// `created_at` 为 0 表示记录时查不到，恢复时不做身份校验。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcRecord {
    pub pid: u32,
    #[serde(default)]
    pub created_at: i64,
}

impl ProcRecord {
    pub fn bare(pid: u32) -> Self {
        Self { pid, created_at: 0 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// 快照格式版本；由 [`save`] 盖章，加载侧与 [`SCHEMA_CURRENT`] 不符即丢弃。
    #[serde(default)]
    pub schema: u32,
    /// 写入时刻推算出的本次开机时刻（Unix 毫秒）；由 [`save`] 盖章。
    #[serde(default)]
    pub boot_time_ms: i64,
    pub hidden: Vec<Target>,
    pub frozen: Vec<ProcRecord>,
    pub muted: Vec<ProcRecord>,
    pub enhanced: bool,
}

impl Snapshot {
    /// 没有任何需要恢复的内容。
    pub fn is_empty(&self) -> bool {
        self.hidden.is_empty() && self.frozen.is_empty() && self.muted.is_empty()
    }

    /// 快照是否可用于恢复：格式为当前版本，且写入时与现在处于同一次开机。
    pub fn is_restorable(&self, now_boot_time_ms: i64) -> bool {
        self.schema == SCHEMA_CURRENT && same_boot(self.boot_time_ms, now_boot_time_ms)
    }
}

/// 由「当前时间 − 开机以来的毫秒数」推算本次开机时刻（Unix 毫秒）。
pub fn current_boot_time_ms() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let uptime = unsafe { windows::Win32::System::SystemInformation::GetTickCount64() } as i64;
    now - uptime
}

/// 两个推算出的开机时刻是否指同一次开机（容差内）。
pub fn same_boot(a: i64, b: i64) -> bool {
    (a - b).abs() <= BOOT_TOLERANCE_MS
}

/// 保存快照：盖上版本与开机时刻，写入 tmp 后 rename 原子替换。
/// 快照为空时等价于清除。
pub fn save(path: &Path, snapshot: &Snapshot) -> std::io::Result<()> {
    if snapshot.is_empty() {
        clear(path);
        return Ok(());
    }
    let mut stamped = snapshot.clone();
    stamped.schema = SCHEMA_CURRENT;
    stamped.boot_time_ms = current_boot_time_ms();
    let json = serde_json::to_string_pretty(&stamped)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = tmp_path(path);
    std::fs::write(&tmp, json)?;
    // Windows 下 rename 走 MOVEFILE_REPLACE_EXISTING，同目录替换是原子的。
    std::fs::rename(&tmp, path)
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

/// 读取快照。文件不存在或内容为空时返回 None；损坏时改名保留现场并 warn。
pub fn load(path: &Path) -> Option<Snapshot> {
    let content = std::fs::read_to_string(path).ok()?;
    let snapshot: Snapshot = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => {
            let corrupt = corrupt_path(path);
            let _ = std::fs::rename(path, &corrupt);
            log_warn!(
                "恢复文件解析失败，本次不恢复；原文件已改名为 {}: {e}",
                corrupt.display()
            );
            return None;
        }
    };
    if snapshot.is_empty() {
        None
    } else {
        Some(snapshot)
    }
}

fn corrupt_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".corrupt");
    path.with_file_name(name)
}

/// 删除恢复文件（不存在时静默成功）。
pub fn clear(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_file() -> PathBuf {
        tempfile::tempdir()
            .expect("创建临时目录失败")
            .keep()
            .join(RECOVERY_FILE_NAME)
    }

    fn sample() -> Snapshot {
        Snapshot {
            hidden: vec![Target::bare(10, 100), Target::bare(20, 200)],
            frozen: vec![ProcRecord {
                pid: 100,
                created_at: 111,
            }],
            muted: vec![ProcRecord::bare(200)],
            enhanced: true,
            ..Default::default()
        }
    }

    #[test]
    fn save_load_round_trip_and_stamps_identity() {
        let path = temp_file();
        save(&path, &sample()).expect("保存快照应成功");
        let loaded = load(&path).expect("应能读回");
        assert_eq!(loaded.hidden, sample().hidden);
        assert_eq!(loaded.frozen, sample().frozen);
        assert_eq!(loaded.muted, sample().muted);
        assert_eq!(loaded.schema, SCHEMA_CURRENT, "保存时应盖上版本");
        assert!(
            same_boot(loaded.boot_time_ms, current_boot_time_ms()),
            "保存时应盖上本次开机时刻"
        );
        assert!(loaded.is_restorable(current_boot_time_ms()));
        assert!(!path.with_file_name("recovery.json.tmp").exists());
    }

    #[test]
    fn load_missing_file_returns_none() {
        assert_eq!(
            load(Path::new("Z:\\definitely\\missing\\recovery.json")),
            None
        );
    }

    #[test]
    fn load_corrupt_file_renames_it_for_inspection() {
        let path = temp_file();
        std::fs::write(&path, "{ not valid json !!").unwrap();
        assert_eq!(load(&path), None, "损坏文件应按无快照处理而非 panic");
        assert!(!path.exists(), "损坏文件不应留在原名");
        assert!(
            corrupt_path(&path).exists(),
            "损坏文件应改名保留现场供排查"
        );
    }

    #[test]
    fn snapshot_from_previous_boot_is_not_restorable() {
        let now = current_boot_time_ms();
        let mut snapshot = sample();
        snapshot.schema = SCHEMA_CURRENT;
        snapshot.boot_time_ms = now - 3_600_000; // 一小时前的「开机」
        assert!(!snapshot.is_restorable(now), "跨重启的快照不可恢复");
        snapshot.boot_time_ms = now - 1_000; // 容差内
        assert!(snapshot.is_restorable(now));
    }

    #[test]
    fn legacy_snapshot_without_schema_is_not_restorable() {
        // 旧版快照（frozen/muted 是裸 PID 数组）应能解析但不可恢复。
        let json = r#"{"hidden":[{"hwnd":10,"pid":100}],"frozen":[],"muted":[],"enhanced":false}"#;
        let snapshot: Snapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snapshot.schema, 0);
        assert!(!snapshot.is_restorable(current_boot_time_ms()));
    }

    #[test]
    fn empty_snapshot_is_treated_as_absent() {
        let path = temp_file();
        save(&path, &sample()).unwrap();
        save(&path, &Snapshot::default()).unwrap();
        assert!(!path.exists(), "空快照应删除恢复文件");
        assert_eq!(load(&path), None);
    }

    #[test]
    fn clear_removes_file_and_tolerates_missing() {
        let path = temp_file();
        save(&path, &sample()).unwrap();
        clear(&path);
        assert!(!path.exists());
        clear(&path);
    }

    #[test]
    fn same_boot_respects_tolerance() {
        assert!(same_boot(1_000_000, 1_000_000));
        assert!(same_boot(1_000_000, 1_000_000 + BOOT_TOLERANCE_MS));
        assert!(!same_boot(1_000_000, 1_000_000 + BOOT_TOLERANCE_MS + 1));
        assert!(!same_boot(1_000_000, 995_000 - 1));
    }
}
