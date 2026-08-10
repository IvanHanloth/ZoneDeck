//! 品牌改名迁移：`%APPDATA%\BossKey` 自动迁往 `%APPDATA%\ZoneDeck`。
//! 新目录已存在则不动；旧目录被占用重命名失败时退回复制关键文件。

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use zonedeck_common::paths::{self, CONFIG_FILE_NAME, LEGACY_USER_DIR_NAME, USER_DIR_NAME};

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn renames_legacy_dir_when_new_dir_missing() {
    let appdata = TempDir::new().unwrap();
    let legacy = appdata.path().join(LEGACY_USER_DIR_NAME);
    let new = appdata.path().join(USER_DIR_NAME);
    write(&legacy.join(CONFIG_FILE_NAME), "{\"v\":1}");
    write(&legacy.join("logs").join("BossKey-2026-08-01.log"), "line");

    paths::migrate_legacy_user_dir(&new);

    assert!(!legacy.exists(), "旧目录应整体搬走");
    assert_eq!(
        fs::read_to_string(new.join(CONFIG_FILE_NAME)).unwrap(),
        "{\"v\":1}"
    );
    assert!(new.join("logs").join("BossKey-2026-08-01.log").exists());
}

#[test]
fn keeps_existing_new_dir_untouched() {
    let appdata = TempDir::new().unwrap();
    let legacy = appdata.path().join(LEGACY_USER_DIR_NAME);
    let new = appdata.path().join(USER_DIR_NAME);
    write(&legacy.join(CONFIG_FILE_NAME), "old");
    write(&new.join(CONFIG_FILE_NAME), "current");

    paths::migrate_legacy_user_dir(&new);

    assert_eq!(
        fs::read_to_string(new.join(CONFIG_FILE_NAME)).unwrap(),
        "current"
    );
    assert!(legacy.exists(), "新目录在用时不碰旧目录");
}

#[test]
fn does_nothing_without_legacy_dir() {
    let appdata = TempDir::new().unwrap();
    let new = appdata.path().join(USER_DIR_NAME);

    paths::migrate_legacy_user_dir(&new);

    assert!(!new.exists(), "无旧数据时不应创建新目录");
}

#[test]
fn copies_key_files_when_rename_is_blocked() {
    let appdata = TempDir::new().unwrap();
    let legacy = appdata.path().join(LEGACY_USER_DIR_NAME);
    let new = appdata.path().join(USER_DIR_NAME);
    write(&legacy.join(CONFIG_FILE_NAME), "{\"v\":2}");
    write(&legacy.join("recovery.json"), "[]");

    // 以仅共享读的方式打开文件，让整个目录无法重命名，迫使走复制回退。
    let _hold = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .share_mode(0x1) // FILE_SHARE_READ
            .open(legacy.join(CONFIG_FILE_NAME))
            .unwrap()
    };
    paths::migrate_legacy_user_dir(&new);

    assert_eq!(
        fs::read_to_string(new.join(CONFIG_FILE_NAME)).unwrap(),
        "{\"v\":2}"
    );
    assert_eq!(fs::read_to_string(new.join("recovery.json")).unwrap(), "[]");
    assert!(legacy.exists(), "重命名失败时旧目录留在原处");
}
