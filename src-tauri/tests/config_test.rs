use app_lib::app::config;

#[test]
fn test_default_config_loading() {
    // 埋め込まれた AppConfig.json が正常にパースできるか
    let cfg = config::default_config();

    // 基本的なフィールドがデフォルト値を持っているか
    // (types.rs の AppConfig 定義に基づき確認)
    assert!(!cfg.language.is_empty());
}

#[test]
fn test_i18n_integrity() {
    let i18n = config::default_i18n();

    // JSONに合わせて "en" と "ja" をチェック
    assert!(i18n.contains_key("ja"), "i18n should contain ja");
    assert!(i18n.contains_key("en"), "i18n should contain en");

    let ja = i18n.get("ja").unwrap();
    assert!(!ja.is_empty());
    // 具体的なキー（例: "settings"）があるかチェック
    assert!(ja.contains_key("settings"));
}
