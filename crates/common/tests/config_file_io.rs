use zonedeck_common::{Config, ProcessRule, WindowInfo, WindowRule};

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
    // load 无副作用：启动早期读日志参数等场景不得破坏现场，
    // 隔离备份只由 load_reporting 执行。
    assert!(path.exists(), "load 不得移走原文件");
    assert!(
        !dir.path().join("corrupt.json.bad").exists(),
        "load 不得产生备份"
    );
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
fn a_corrupt_file_is_backed_up_before_falling_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, "{ not valid json at all ").unwrap();

    let (loaded, parse_error) = Config::load_reporting(&path).unwrap();
    assert_eq!(loaded, Config::default());

    let backup = dir.path().join("config.json.bad");
    assert!(
        backup.exists(),
        "解析失败的原文件必须先备份，否则随后写入的默认配置会把用户规则永久覆写"
    );
    assert_eq!(
        std::fs::read_to_string(&backup).unwrap(),
        "{ not valid json at all ",
        "备份须保留原文件的完整内容"
    );
    assert!(!path.exists(), "备份即改名，原路径让位给随后写入的默认配置");

    let msg = parse_error.unwrap();
    assert!(
        msg.contains("config.json.bad"),
        "备份去向必须出现在报告里，日志才能指引用户找回: {msg}"
    );
}

#[test]
fn a_bad_numeric_field_keeps_user_rules_recoverable_from_backup() {
    // 单个数值字段类型不符（如浮点写进整数字段）会让整份配置解析失败并回退
    // 默认值；用户规则必须能从备份找回。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut cfg = Config::default();
    cfg.window_rules
        .push(WindowRule::from_window(&WindowInfo::new(
            "微信",
            555,
            "WeChat.exe",
            2020,
            "C:\\WeChat.exe",
        )));
    cfg.save(&path).unwrap();

    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    v["setting"]["auto_hide_time"] = serde_json::json!(5.5);
    std::fs::write(&path, serde_json::to_string(&v).unwrap()).unwrap();

    let (loaded, parse_error) = Config::load_reporting(&path).unwrap();
    assert_eq!(loaded, Config::default(), "整份解析失败时回退默认值");
    assert!(parse_error.is_some());

    let backup = std::fs::read_to_string(dir.path().join("config.json.bad")).unwrap();
    assert!(
        backup.contains("微信"),
        "用户规则必须留在备份里可供找回: {backup}"
    );
}

#[test]
fn a_new_corruption_replaces_the_stale_backup() {
    // 旧备份对应的配置早已被默认值取代，新损坏文件里才是最新的用户数据，
    // rename 直接顶替旧备份。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(dir.path().join("config.json.bad"), "stale backup").unwrap();
    std::fs::write(&path, "{ fresh corruption ").unwrap();

    Config::load_reporting(&path).unwrap();

    assert_eq!(
        std::fs::read_to_string(dir.path().join("config.json.bad")).unwrap(),
        "{ fresh corruption ",
        "新损坏文件必须顶替旧备份"
    );
}

#[test]
fn a_failed_backup_still_falls_back_and_reports() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, "{ not valid json at all ").unwrap();
    // 用同名目录占住备份位置：rename 到已存在目录必然失败，稳定触发备份失败分支。
    std::fs::create_dir(dir.path().join("config.json.bad")).unwrap();

    let (loaded, parse_error) = Config::load_reporting(&path).unwrap();
    assert_eq!(
        loaded,
        Config::default(),
        "备份失败不得阻断回退，核心必须能启动"
    );
    assert!(path.exists(), "备份失败时原文件留在原地");
    let msg = parse_error.unwrap();
    assert!(
        msg.contains("备份原文件失败"),
        "报告须注明备份未成功、数据仍有被覆盖的风险: {msg}"
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

    assert!(
        !dir.path().join("config.json.bad").exists(),
        "健康文件不得产生备份"
    );
    assert!(path.exists(), "健康文件不得被移走");
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
