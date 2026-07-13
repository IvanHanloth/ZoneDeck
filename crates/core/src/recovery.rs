//! 崩溃恢复：隐藏状态落盘，核心异常退出后下次启动自动找回窗口。
//!
//! 流程：
//! - 每次隐藏后，把「隐藏的窗口 + 冻结/静音的进程」写入恢复文件；
//! - 正常显示或正常退出时删除恢复文件；
//! - 启动时若恢复文件存在，说明上次是异常退出——恢复窗口、解冻、取消静音，
//!   避免窗口永远“找不回来”。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::hide::Target;

/// 恢复文件名（与 config.json 同目录）。
pub const RECOVERY_FILE_NAME: &str = "recovery.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub hidden: Vec<Target>,
    pub frozen: Vec<u32>,
    pub muted: Vec<u32>,
    pub enhanced: bool,
}

impl Snapshot {
    /// 没有任何需要恢复的内容。
    pub fn is_empty(&self) -> bool {
        self.hidden.is_empty() && self.frozen.is_empty() && self.muted.is_empty()
    }
}

/// 保存快照。快照为空时等价于清除（避免留下无意义的文件）。
pub fn save(path: &Path, snapshot: &Snapshot) -> std::io::Result<()> {
    if snapshot.is_empty() {
        clear(path);
        return Ok(());
    }
    let json = serde_json::to_string_pretty(snapshot)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// 读取快照。文件不存在、损坏或内容为空时返回 None。
pub fn load(path: &Path) -> Option<Snapshot> {
    let content = std::fs::read_to_string(path).ok()?;
    let snapshot: Snapshot = serde_json::from_str(&content).ok()?;
    if snapshot.is_empty() {
        None
    } else {
        Some(snapshot)
    }
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
            hidden: vec![Target { hwnd: 10, pid: 100 }, Target { hwnd: 20, pid: 200 }],
            frozen: vec![100],
            muted: vec![200],
            enhanced: true,
        }
    }

    #[test]
    fn save_load_round_trip() {
        let path = temp_file();
        let snapshot = sample();
        save(&path, &snapshot).expect("保存快照应成功");
        assert_eq!(load(&path), Some(snapshot));
    }

    #[test]
    fn load_missing_file_returns_none() {
        assert_eq!(
            load(Path::new("Z:\\definitely\\missing\\recovery.json")),
            None
        );
    }

    #[test]
    fn load_corrupt_file_returns_none() {
        let path = temp_file();
        std::fs::write(&path, "{ not valid json !!").unwrap();
        assert_eq!(load(&path), None, "损坏文件应按无快照处理而非 panic");
    }

    #[test]
    fn empty_snapshot_is_treated_as_absent() {
        let path = temp_file();
        // 先写入有效快照，再保存空快照——应把文件清掉。
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
        clear(&path); // 再次清除不应报错
    }
}
