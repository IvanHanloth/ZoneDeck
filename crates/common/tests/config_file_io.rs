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
fn save_leaves_no_temp_file_behind() {
    let dir = tempfile::tempdir().unwrap();
    Config::default()
        .save(&dir.path().join("config.json"))
        .unwrap();

    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["config.json".to_string()], "临时文件须已改名");
}

#[test]
fn a_failed_save_keeps_the_previous_file_intact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut cfg = Config::default();
    cfg.hotkey.hide_hotkey = "Ctrl+Shift+B".to_string();
    cfg.save(&path).unwrap();

    // 占住临时文件名（这里用目录），逼真地模拟写入中途失败（磁盘满、杀软拦截）。
    std::fs::create_dir(dir.path().join("config.json.tmp")).unwrap();
    let err = Config::default().save(&path).unwrap_err();

    let kept = Config::load(&path).unwrap();
    assert_eq!(
        kept.hotkey.hide_hotkey, "Ctrl+Shift+B",
        "写入失败不得把原配置截断，用户的规则不能因此丢光: {err}"
    );
}

#[test]
fn io_errors_name_the_path_they_failed_on() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("locked").join("config.json");
    // 父路径是个文件，创建目录必然失败。
    std::fs::write(dir.path().join("locked"), "occupied").unwrap();

    let err = Config::default().save(&path).unwrap_err().to_string();
    assert!(
        err.contains("locked"),
        "报错须带上实际路径，否则用户无从判断问题出在哪个目录: {err}"
    );
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
