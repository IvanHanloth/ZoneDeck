//! 能效统计：累计冻结 / 效率模式 / 内存释放量落盘，供配置界面展示。
//!
//! 只记核心自动施加的能效控制，文件由核心独占写入，读方（配置程序）只读不写。
//! 存的全是原始累计量（次数、进程·秒、字节），折算成电能与碳排放由界面负责。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::log_warn;

/// 统计文件名（与 config.json 同目录）。
pub const STATS_FILE_NAME: &str = "stats.json";

/// 当前统计格式版本；与之不符的文件按空统计处理，不做迁移。
pub const SCHEMA_CURRENT: u32 = 1;

/// 落盘节流：一次隐藏会连着上报十几个进程，合并成一次写盘。
const SAVE_THROTTLE: Duration = Duration::from_secs(2);

/// 累计能效成绩。次数与字节是实测值，界面据此估算电能与碳排放。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PowerStats {
    #[serde(default)]
    pub schema: u32,
    /// 开始统计的时刻（Unix 秒）；0 表示还没记过任何一笔。
    #[serde(default)]
    pub since: i64,
    #[serde(default)]
    pub updated_at: i64,
    /// 累计冻结进程次数；同一进程反复冻结按多次计。
    #[serde(default)]
    pub freeze_count: u64,
    #[serde(default)]
    pub efficiency_count: u64,
    /// 冻结累计时长，单位「进程·秒」：三个进程各冻结 10 秒记 30。
    #[serde(default)]
    pub freeze_seconds: f64,
    #[serde(default)]
    pub efficiency_seconds: f64,
    /// 清空工作集实测释放的字节数之和。
    #[serde(default)]
    pub memory_freed_bytes: u64,
}

impl PowerStats {
    /// 一笔都没记过。
    pub fn is_empty(&self) -> bool {
        self.freeze_count == 0
            && self.efficiency_count == 0
            && self.memory_freed_bytes == 0
            && self.freeze_seconds == 0.0
            && self.efficiency_seconds == 0.0
    }
}

/// 带节流落盘的统计仓库。核心持有一份 [`Arc`]，副作用线程与代理线程共用。
pub struct PowerStatsStore {
    path: PathBuf,
    inner: Mutex<Inner>,
}

struct Inner {
    stats: PowerStats,
    /// 冻结中的进程 → 起始时刻，解冻时据此结算时长。
    freeze_started: HashMap<u32, Instant>,
    eco_started: HashMap<u32, Instant>,
    dirty: bool,
    /// None 表示本次运行还没落过盘，下一笔改动即写。
    last_save: Option<Instant>,
}

impl PowerStatsStore {
    /// 从文件恢复累计量；读不出来就从零开始。
    pub fn load(path: PathBuf) -> Arc<Self> {
        let stats = read(&path).unwrap_or_default();
        Arc::new(Self {
            path,
            inner: Mutex::new(Inner {
                stats,
                freeze_started: HashMap::new(),
                eco_started: HashMap::new(),
                dirty: false,
                last_save: None,
            }),
        })
    }

    pub fn on_suspend(&self, pid: u32) {
        self.update(|inner| {
            inner.stats.freeze_count += 1;
            inner.freeze_started.insert(pid, Instant::now());
            true
        });
    }

    pub fn on_resume(&self, pid: u32) {
        self.update(|inner| match inner.freeze_started.remove(&pid) {
            Some(started) => {
                inner.stats.freeze_seconds += started.elapsed().as_secs_f64();
                true
            }
            // 崩溃恢复只会调解冻，没有配对的起始记录，此时没有时长可结算。
            None => false,
        });
    }

    pub fn on_efficiency_on(&self, pid: u32) {
        self.update(|inner| {
            inner.stats.efficiency_count += 1;
            inner.eco_started.insert(pid, Instant::now());
            true
        });
    }

    pub fn on_efficiency_off(&self, pid: u32) {
        self.update(|inner| match inner.eco_started.remove(&pid) {
            Some(started) => {
                inner.stats.efficiency_seconds += started.elapsed().as_secs_f64();
                true
            }
            None => false,
        });
    }

    /// 记一次工作集清空实际释放的字节数。
    pub fn on_trim(&self, freed_bytes: u64) {
        self.update(|inner| {
            if freed_bytes == 0 {
                return false;
            }
            inner.stats.memory_freed_bytes += freed_bytes;
            true
        });
    }

    /// 清零并立即落盘。重置后仍在冻结中的进程不再有起始记录，解冻时不补计时长。
    pub fn reset(&self) {
        let now = unix_now();
        let mut inner = self.lock();
        inner.stats = PowerStats {
            schema: SCHEMA_CURRENT,
            since: now,
            updated_at: now,
            ..Default::default()
        };
        inner.freeze_started.clear();
        inner.eco_started.clear();
        self.write(&mut inner);
    }

    /// 无视节流立即落盘。除退出前，副作用队列排空静置后也会调一次
    /// （见 [`crate::effects_worker`]）：只靠 [`SAVE_THROTTLE`] 的话，
    /// 一批操作末尾那几笔要一直等到下次有改动才写得进去。
    pub fn flush(&self) {
        let mut inner = self.lock();
        if inner.dirty {
            self.write(&mut inner);
        }
    }

    pub fn snapshot(&self) -> PowerStats {
        self.lock().stats.clone()
    }

    /// 加锁执行一笔改动；闭包返回 false 表示什么也没改，不触发落盘。
    fn update(&self, f: impl FnOnce(&mut Inner) -> bool) {
        let mut inner = self.lock();
        if !f(&mut inner) {
            return;
        }
        let now = unix_now();
        if inner.stats.since == 0 {
            inner.stats.since = now;
        }
        inner.stats.updated_at = now;
        inner.dirty = true;
        if inner.last_save.is_none_or(|t| t.elapsed() >= SAVE_THROTTLE) {
            self.write(&mut inner);
        }
    }

    fn write(&self, inner: &mut Inner) {
        inner.stats.schema = SCHEMA_CURRENT;
        if let Err(e) = write_file(&self.path, &inner.stats) {
            log_warn!(
                "写入能效统计失败，本次的统计数据不会保留: {} — {e}",
                self.path.display()
            );
        }
        inner.dirty = false;
        inner.last_save = Some(Instant::now());
    }

    /// 统计失败不该拖垮冻结，锁中毒后照常沿用里面的数据。
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// 读取统计文件。不存在、为空或版本不符时返回 None；损坏时改名保留现场并 warn。
pub fn read(path: &Path) -> Option<PowerStats> {
    let content = std::fs::read_to_string(path).ok()?;
    let stats: PowerStats = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => {
            let corrupt = corrupt_path(path);
            let kept = match std::fs::rename(path, &corrupt) {
                Ok(()) => format!("原文件已改名为 {}", corrupt.display()),
                Err(rename_err) => format!(
                    "原文件改名保留失败（{rename_err}），仍在 {}",
                    path.display()
                ),
            };
            log_warn!("能效统计文件解析失败，累计数据从零重新开始；{kept}: {e}");
            return None;
        }
    };
    (stats.schema == SCHEMA_CURRENT).then_some(stats)
}

/// 读取统计文件，读不出来就给全零；供只读方（配置程序）使用。
pub fn read_or_default(path: &Path) -> PowerStats {
    read(path).unwrap_or_default()
}

/// 删除统计文件；不存在时静默成功。核心未运行时的重置走这条路。
pub fn clear(path: &Path) {
    if let Err(e) = std::fs::remove_file(path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        log_warn!(
            "删除能效统计文件失败，重置可能未生效: {} — {e}",
            path.display()
        );
    }
}

/// 写入 tmp 后 rename 原子替换，读方不会撞见写到一半的文件。
fn write_file(path: &Path, stats: &PowerStats) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(stats)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = suffixed(path, ".tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

fn corrupt_path(path: &Path) -> PathBuf {
    suffixed(path, ".corrupt")
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file() -> PathBuf {
        tempfile::tempdir()
            .expect("创建临时目录失败")
            .keep()
            .join(STATS_FILE_NAME)
    }

    #[test]
    fn counts_and_bytes_accumulate_and_round_trip() {
        let path = temp_file();
        let store = PowerStatsStore::load(path.clone());
        store.on_suspend(100);
        store.on_suspend(200);
        store.on_efficiency_on(300);
        store.on_trim(4096);
        store.on_trim(1024);
        store.flush();

        let saved = read(&path).expect("应能读回");
        assert_eq!(saved.freeze_count, 2);
        assert_eq!(saved.efficiency_count, 1);
        assert_eq!(saved.memory_freed_bytes, 5120);
        assert_eq!(saved.schema, SCHEMA_CURRENT, "保存时应盖上版本");
        assert!(saved.since > 0, "第一笔记录时应盖上起始时刻");
        assert!(!path.with_file_name("stats.json.tmp").exists());
    }

    #[test]
    fn load_resumes_from_previous_run() {
        let path = temp_file();
        let store = PowerStatsStore::load(path.clone());
        store.on_suspend(100);
        store.flush();
        let since = store.snapshot().since;

        let reopened = PowerStatsStore::load(path);
        reopened.on_suspend(200);
        let stats = reopened.snapshot();
        assert_eq!(stats.freeze_count, 2, "重启后应在旧累计量上继续加");
        assert_eq!(stats.since, since, "起始时刻不该被重启刷新");
    }

    #[test]
    fn durations_settle_on_the_paired_call() {
        let store = PowerStatsStore::load(temp_file());
        store.on_suspend(100);
        store.on_efficiency_on(100);
        std::thread::sleep(Duration::from_millis(20));
        store.on_resume(100);
        store.on_efficiency_off(100);

        let stats = store.snapshot();
        assert!(stats.freeze_seconds > 0.0, "解冻时应结算冻结时长");
        assert!(stats.efficiency_seconds > 0.0, "撤销时应结算效率模式时长");
    }

    #[test]
    fn resume_without_a_matching_suspend_is_ignored() {
        // 崩溃恢复只会调解冻，不得因此倒扣计数或凭空加时长。
        let store = PowerStatsStore::load(temp_file());
        store.on_resume(100);
        store.on_efficiency_off(100);

        let stats = store.snapshot();
        assert_eq!(stats.freeze_count, 0);
        assert_eq!(stats.freeze_seconds, 0.0);
        assert_eq!(stats.efficiency_seconds, 0.0);
        assert!(stats.is_empty());
    }

    #[test]
    fn trimming_nothing_is_not_recorded() {
        let store = PowerStatsStore::load(temp_file());
        store.on_trim(0);
        assert!(store.snapshot().is_empty(), "没释放出内存就不该记一笔");
    }

    #[test]
    fn reset_clears_counters_and_pending_sessions() {
        let path = temp_file();
        let store = PowerStatsStore::load(path.clone());
        store.on_suspend(100);
        store.reset();

        assert!(store.snapshot().is_empty());
        assert!(store.snapshot().since > 0, "重置后应重新起算");
        assert!(read(&path).expect("重置应立即落盘").is_empty());

        // 重置前就冻着的进程，解冻时不该把跨越重置的时长补记进来。
        store.on_resume(100);
        assert_eq!(store.snapshot().freeze_seconds, 0.0);
    }

    #[test]
    fn load_missing_file_starts_from_zero() {
        assert_eq!(read(Path::new("Z:\\definitely\\missing\\stats.json")), None);
        let store = PowerStatsStore::load(PathBuf::from("Z:\\definitely\\missing\\stats.json"));
        assert!(store.snapshot().is_empty());
    }

    #[test]
    fn load_corrupt_file_renames_it_for_inspection() {
        let path = temp_file();
        std::fs::write(&path, "{ not valid json !!").unwrap();
        assert_eq!(read(&path), None, "损坏文件应按无统计处理而非 panic");
        assert!(!path.exists(), "损坏文件不应留在原名");
        assert!(corrupt_path(&path).exists(), "损坏文件应改名保留现场供排查");
    }

    #[test]
    fn file_from_another_schema_is_discarded() {
        let path = temp_file();
        std::fs::write(&path, r#"{"schema":999,"freeze_count":42}"#).unwrap();
        assert_eq!(read(&path), None, "版本不符的文件不做迁移，从零重新统计");
    }

    #[test]
    fn clear_removes_file_and_tolerates_missing() {
        let path = temp_file();
        PowerStatsStore::load(path.clone()).reset();
        assert!(path.exists());
        clear(&path);
        assert!(!path.exists());
        clear(&path);
    }
}
