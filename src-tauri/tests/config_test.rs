use serde_json::{json, Value};
use work_backup_tool::core::config::loader::default_config;
use work_backup_tool::core::config::update::apply_config_value;
use work_backup_tool::core::types::AppConfig;

fn to_value(cfg: &AppConfig) -> Value {
    serde_json::to_value(cfg).unwrap()
}

// =====================================================================
// 旧バージョンの設定ファイルでも、設定が初期化されないこと
// =====================================================================

#[test]
fn test_old_config_without_new_fields_keeps_user_values() {
    // 機能追加前の AppConfig.json(新しい項目なし)+ ユーザーが変更した値
    let old = r#"{
        "language": "en",
        "alwaysOnTop": true,
        "restorePreviousState": false,
        "autoBaseGenerationThreshold": 0.9,
        "trayMode": true,
        "useSameDirForTemp": true,
        "trayBackupMode": "archive",
        "rebuildCacheOnStartup": false,
        "startupCacheLimit": 12,
        "showMemoAfterBackup": false,
        "strictFileNameMatch": false,
        "hdiffStrictHashCheck": false
    }"#;
    let cfg: AppConfig = serde_json::from_str(old).expect("old config must still parse");

    // ユーザーの値はそのまま
    assert_eq!(cfg.language, "en");
    assert!(cfg.always_on_top);
    assert!(!cfg.restore_previous_state);
    assert_eq!(cfg.auto_base_generation_threshold, 0.9);
    assert_eq!(cfg.startup_cache_limit, 12);
    assert_eq!(cfg.tray_backup_mode, "archive");

    // 新しい項目は既定値
    assert!(!cfg.global_shortcut_enabled);
    assert!(!cfg.notify_enabled);
    assert_eq!(cfg.notify_style, "popup");
    assert_eq!(cfg.notify_duration_sec, 3);
    assert!(!cfg.reminder_enabled);
    assert_eq!(cfg.reminder_interval_min, 30);
    assert!(cfg.reminder_skip_unchanged);
    assert!(!cfg.open_after_restore);
}

#[test]
fn test_empty_object_equals_embedded_default() {
    // どの項目が欠けていても、組み込みの既定値(AppConfig.json)と同じになる
    let from_empty: AppConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(to_value(&from_empty), to_value(&default_config()));
}

#[test]
fn test_config_roundtrip() {
    let cfg = default_config();
    let text = serde_json::to_string_pretty(&cfg).unwrap();
    let back: AppConfig = serde_json::from_str(&text).unwrap();
    assert_eq!(to_value(&cfg), to_value(&back));
}

#[test]
fn test_embedded_default_has_documented_values() {
    let cfg = default_config();
    assert_eq!(cfg.language, "ja");
    assert_eq!(cfg.shortcut_run, "Ctrl+Alt+B");
    assert_eq!(cfg.shortcut_run_with_memo, "Ctrl+Alt+Shift+B");
    assert_eq!(cfg.shortcut_backup_mode, "diff");
    assert!(!cfg.global_shortcut_enabled);
}

// =====================================================================
// apply_config_value
// =====================================================================

#[test]
fn test_apply_bool_ok_and_wrong_type() {
    let mut cfg = default_config();
    apply_config_value(&mut cfg, "restorePreviousState", &json!(false)).unwrap();
    assert!(!cfg.restore_previous_state);
    apply_config_value(&mut cfg, "useSameDirForTemp", &json!(true)).unwrap();
    assert!(cfg.use_same_dir_for_temp);
    apply_config_value(&mut cfg, "rebuildCacheOnStartup", &json!(false)).unwrap();
    assert!(!cfg.rebuild_cache_on_startup);
    apply_config_value(&mut cfg, "showMemoAfterBackup", &json!(false)).unwrap();
    assert!(!cfg.show_memo_after_backup);

    // 文字列や数値は受け付けない(従来は黙って false になっていた)
    assert!(apply_config_value(&mut cfg, "restorePreviousState", &json!("true")).is_err());
    assert!(apply_config_value(&mut cfg, "notifyEnabled", &json!(1)).is_err());
}
#[test]
fn test_apply_threshold_range() {
    let mut cfg = default_config();
    apply_config_value(&mut cfg, "autoBaseGenerationThreshold", &json!(0.8)).unwrap();
    assert_eq!(cfg.auto_base_generation_threshold, 0.8);

    // 下限未満
    assert!(apply_config_value(&mut cfg, "autoBaseGenerationThreshold", &json!(0.05)).is_err());
    // 上限超過（1.5 は範囲内なので使わない）
    assert!(apply_config_value(&mut cfg, "autoBaseGenerationThreshold", &json!(3.5)).is_err());
    // 型違い
    assert!(apply_config_value(&mut cfg, "autoBaseGenerationThreshold", &json!("0.5")).is_err());

    // 失敗したときは値が変わらない
    assert_eq!(cfg.auto_base_generation_threshold, 0.8);
}

#[test]
fn test_apply_startup_cache_limit() {
    let mut cfg = default_config();
    apply_config_value(&mut cfg, "startupCacheLimit", &json!(0)).unwrap();
    assert_eq!(cfg.startup_cache_limit, 0);
    assert!(apply_config_value(&mut cfg, "startupCacheLimit", &json!(-1)).is_err());
}

#[test]
fn test_apply_notify_settings() {
    let mut cfg = default_config();
    apply_config_value(&mut cfg, "notifyStyle", &json!("both")).unwrap();
    assert_eq!(cfg.notify_style, "both");
    assert!(apply_config_value(&mut cfg, "notifyStyle", &json!("loud")).is_err());

    apply_config_value(&mut cfg, "notifyDurationSec", &json!(5)).unwrap();
    assert_eq!(cfg.notify_duration_sec, 5);
    assert!(apply_config_value(&mut cfg, "notifyDurationSec", &json!(0)).is_err());
    assert!(apply_config_value(&mut cfg, "notifyDurationSec", &json!(6)).is_err());
}

#[test]
fn test_apply_reminder_settings() {
    let mut cfg = default_config();
    for v in [15, 30, 60] {
        apply_config_value(&mut cfg, "reminderIntervalMin", &json!(v)).unwrap();
        assert_eq!(cfg.reminder_interval_min, v);
    }
    assert!(apply_config_value(&mut cfg, "reminderIntervalMin", &json!(45)).is_err());
    apply_config_value(&mut cfg, "reminderMethod", &json!("sound")).unwrap();
    assert_eq!(cfg.reminder_method, "sound");
    assert!(apply_config_value(&mut cfg, "reminderMethod", &json!("both")).is_err());
}

#[test]
fn test_apply_shortcut_settings() {
    let mut cfg = default_config();
    apply_config_value(&mut cfg, "shortcutRun", &json!("  Ctrl+Shift+F9 ")).unwrap();
    assert_eq!(cfg.shortcut_run, "Ctrl+Shift+F9"); // 前後の空白は取り除く
    apply_config_value(&mut cfg, "shortcutRunWithMemo", &json!("")).unwrap();
    assert_eq!(cfg.shortcut_run_with_memo, ""); // 空文字 = 未割り当て
    assert!(apply_config_value(&mut cfg, "shortcutRun", &json!("Ctrl+\nA")).is_err());
    assert!(apply_config_value(&mut cfg, "shortcutRun", &json!(["Ctrl+A"])).is_err());

    apply_config_value(&mut cfg, "shortcutBackupMode", &json!("archive")).unwrap();
    assert_eq!(cfg.shortcut_backup_mode, "archive");
    assert!(apply_config_value(&mut cfg, "shortcutBackupMode", &json!("full")).is_err());
}

#[test]
fn test_apply_unknown_key_is_error() {
    let mut cfg = default_config();
    assert!(apply_config_value(&mut cfg, "doesNotExist", &json!(true)).is_err());
    // メニューで切り替える項目は、この経路では変更しない
    assert!(apply_config_value(&mut cfg, "alwaysOnTop", &json!(true)).is_err());
}
