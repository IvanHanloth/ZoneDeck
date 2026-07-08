use bosskey_common::{Config, WindowInfo};

#[test]
fn save_then_load_round_trips_through_a_real_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut cfg = Config::default();
    cfg.setting.freeze_after_hide = true;
    cfg.setting.auto_hide_time = 12;
    cfg.hotkey.hide_hotkey = "Ctrl+Shift+B".to_string();
    cfg.hide_binding.push(WindowInfo::new(
        "微信",
        555,
        "WeChat.exe",
        2020,
        "C:\\WeChat.exe",
    ));

    cfg.save(&path).unwrap();
    assert!(path.exists());

    let loaded = Config::load(&path).unwrap();
    assert_eq!(cfg, loaded);
}

#[test]
fn loading_a_missing_file_returns_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does_not_exist.json");
    let loaded = Config::load(&path).unwrap();
    assert_eq!(loaded, Config::default());
}

#[test]
fn loading_a_corrupt_file_falls_back_to_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt.json");
    std::fs::write(&path, "{ not valid json at all ").unwrap();
    let loaded = Config::load(&path).unwrap();
    assert_eq!(loaded, Config::default());
}

#[test]
fn save_creates_missing_parent_directories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("deeper").join("config.json");
    Config::default().save(&path).unwrap();
    assert!(path.exists());
}

#[test]
fn written_file_is_readable_by_a_generic_json_parser() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    Config::default().save(&path).unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(value.get("setting").is_some());
    assert!(value.get("hotkey").is_some());
    assert!(value.get("hide_binding").is_some());
}
