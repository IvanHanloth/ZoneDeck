use bosskey_common::{Config, ProcessRule, WindowInfo, WindowRule};

#[test]
fn save_then_load_round_trips_through_a_real_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut cfg = Config::default();
    cfg.setting.freeze_after_hide = true;
    cfg.setting.auto_hide_time = 12;
    cfg.hotkey.hide_hotkey = "Ctrl+Shift+B".to_string();
    cfg.window_rules
        .push(WindowRule::from_window(&WindowInfo::new(
            "微信",
            555,
            "WeChat.exe",
            2020,
            "C:\\WeChat.exe",
        )));
    cfg.process_rules
        .push(ProcessRule::from_regex(r".*\\game\.exe$"));

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
fn load_reporting_surfaces_the_parse_error_behind_a_corrupt_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt.json");
    std::fs::write(&path, "{ not valid json at all ").unwrap();

    let (loaded, parse_error) = Config::load_reporting(&path).unwrap();
    assert_eq!(loaded, Config::default(), "损坏文件仍应回退默认值");
    assert!(
        parse_error.is_some(),
        "回退默认值这一事实必须能被调用方记录，否则用户规则丢失后无迹可循"
    );
}

#[test]
fn load_reporting_is_quiet_for_healthy_and_missing_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let (_, parse_error) = Config::load_reporting(&path).unwrap();
    assert_eq!(parse_error, None, "文件不存在不是解析失败");

    Config::default().save(&path).unwrap();
    let (_, parse_error) = Config::load_reporting(&path).unwrap();
    assert_eq!(parse_error, None, "正常文件不应报告解析失败");
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
    assert!(value.get("window_rules").is_some());
    assert!(value.get("process_rules").is_some());
    assert!(value.get("notifications").is_some());
}
