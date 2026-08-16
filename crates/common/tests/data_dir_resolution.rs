//! 数据目录定位：安装版用 `%APPDATA%\ZoneDeck`，便携版用程序目录，写不进去才回退。

use std::path::Path;

use zonedeck_common::paths::{self, CONFIG_FILE_NAME, DataDirKind, INSTALLED_MARKER};

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

#[test]
fn a_portable_copy_keeps_its_data_next_to_the_exe() {
    let program = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let user_dir = user.path().join("ZoneDeck");

    let located = paths::resolve_data_dir(program.path(), &user_dir, false, true);

    assert_eq!(located.dir, program.path(), "拷走整个文件夹就带走了设置");
    assert_eq!(located.kind, DataDirKind::Portable);
    assert!(!user_dir.exists(), "不该无谓地在用户目录下留空文件夹");
}

#[test]
fn an_installed_copy_uses_the_user_dir() {
    let program = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let user_dir = user.path().join("ZoneDeck");

    // 装在 Program Files 下的核心提权后写得进程序目录，但仍须用用户目录：
    // 普通权限的配置程序写不进去，两边不能各读一份。
    let located = paths::resolve_data_dir(program.path(), &user_dir, true, true);

    assert_eq!(located.dir, user_dir);
    assert_eq!(located.kind, DataDirKind::Installed);
    assert!(user_dir.is_dir(), "目录须就地创建，否则首次保存仍会失败");
}

#[test]
fn an_unwritable_portable_copy_falls_back_and_says_so() {
    let program = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let user_dir = user.path().join("ZoneDeck");

    let located = paths::resolve_data_dir(program.path(), &user_dir, false, false);

    assert_eq!(located.dir, user_dir, "写不进去也得让程序能用");
    assert_eq!(
        located.kind,
        DataDirKind::PortableFallback,
        "界面要据此提示用户这是权限问题，不能与正常的便携版混为一谈"
    );
    assert_eq!(
        located.program_dir,
        program.path(),
        "提示里要点名是哪个目录"
    );
}

#[test]
fn the_marker_file_identifies_an_installed_copy() {
    let program = tempfile::tempdir().unwrap();
    assert!(
        !paths::is_installed(program.path()),
        "压缩包解压出来的是便携版"
    );

    write(&program.path().join(INSTALLED_MARKER), "installed\n");
    assert!(paths::is_installed(program.path()));
}

#[test]
fn the_uninstaller_identifies_an_installed_copy_without_the_marker() {
    let program = tempfile::tempdir().unwrap();
    // 序号随重复安装递增，不能只认 unins000。
    write(&program.path().join("unins001.exe"), "");

    assert!(
        paths::is_installed(program.path()),
        "标记文件被误删也不能把数据写回 Program Files"
    );
}

#[test]
fn a_lookalike_file_is_not_mistaken_for_an_uninstaller() {
    let program = tempfile::tempdir().unwrap();
    write(&program.path().join("uninstall-notes.txt"), "");
    write(&program.path().join("unins000.exe.bak"), "");

    assert!(
        !paths::is_installed(program.path()),
        "便携版被误判成安装版的话，设置就不跟着文件夹走了"
    );
}

#[test]
fn an_installed_copy_takes_over_a_config_left_in_the_program_dir() {
    let program = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let user_dir = user.path().join("ZoneDeck");
    let old = program.path().join(CONFIG_FILE_NAME);
    write(&old, r#"{"hotkey": {"hide_hotkey": "Ctrl+Shift+B"}}"#);

    paths::resolve_data_dir(program.path(), &user_dir, true, false);

    let migrated = std::fs::read_to_string(user_dir.join(CONFIG_FILE_NAME)).unwrap();
    assert!(
        migrated.contains("Ctrl+Shift+B"),
        "换位置不能让用户的设置凭空消失: {migrated}"
    );
    assert!(!old.exists(), "搬走后旧位置不留副本");
}

#[test]
fn an_undeletable_original_is_left_where_it_is() {
    let program = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let user_dir = user.path().join("ZoneDeck");
    let old = program.path().join(CONFIG_FILE_NAME);
    write(&old, r#"{"version": "old"}"#);
    // 打开着的文件在 Windows 上删不掉，等价于 Program Files 下没有写权限的情形。
    let hold = std::fs::File::open(&old).unwrap();

    let located = paths::resolve_data_dir(program.path(), &user_dir, true, false);

    drop(hold);
    assert_eq!(located.dir, user_dir, "删不掉旧文件不影响迁移结果");
    let migrated = std::fs::read_to_string(user_dir.join(CONFIG_FILE_NAME)).unwrap();
    assert!(migrated.contains("old"), "内容仍须搬过去");
}

#[test]
fn a_config_already_in_the_user_dir_is_not_overwritten() {
    let program = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let user_dir = user.path().join("ZoneDeck");
    write(&user_dir.join(CONFIG_FILE_NAME), r#"{"version": "user"}"#);
    let old = program.path().join(CONFIG_FILE_NAME);
    write(&old, r#"{"version": "program"}"#);

    paths::resolve_data_dir(program.path(), &user_dir, true, false);

    let kept = std::fs::read_to_string(user_dir.join(CONFIG_FILE_NAME)).unwrap();
    assert!(
        kept.contains("user"),
        "迁移只补空缺：正在用的那份配置不得被旧位置的文件盖掉"
    );
    assert!(old.exists(), "没搬走的文件不得删除，那可能是用户还要的东西");
}

#[test]
fn migration_is_idempotent() {
    let program = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let user_dir = user.path().join("ZoneDeck");
    write(
        &program.path().join(CONFIG_FILE_NAME),
        r#"{"version": "old"}"#,
    );

    paths::resolve_data_dir(program.path(), &user_dir, true, false);
    write(&user_dir.join(CONFIG_FILE_NAME), r#"{"version": "new"}"#);
    paths::resolve_data_dir(program.path(), &user_dir, true, false);

    let kept = std::fs::read_to_string(user_dir.join(CONFIG_FILE_NAME)).unwrap();
    assert!(kept.contains("new"), "二次定位不得用旧文件盖掉新配置");
}

#[test]
fn resolving_into_the_same_dir_keeps_the_config() {
    // %APPDATA% 取不到时用户目录退回程序目录，此时迁移的源与目标是同一个文件。
    let program = tempfile::tempdir().unwrap();
    write(
        &program.path().join(CONFIG_FILE_NAME),
        r#"{"version": "same"}"#,
    );

    let located = paths::resolve_data_dir(program.path(), program.path(), true, false);

    assert_eq!(located.dir, program.path());
    let kept = std::fs::read_to_string(program.path().join(CONFIG_FILE_NAME)).unwrap();
    assert!(kept.contains("same"), "源与目标相同时不得把配置搬没了");
}

#[test]
fn writability_probe_reports_truth_and_leaves_nothing_behind() {
    let dir = tempfile::tempdir().unwrap();
    assert!(paths::dir_writable(dir.path()));
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "探针文件必须用后即删"
    );

    assert!(
        !paths::dir_writable(&dir.path().join("not_there")),
        "目录不存在即不可写"
    );
}
