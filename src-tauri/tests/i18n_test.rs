use regex::Regex;
use serde_json::Value;
use std::collections::BTreeSet;
use work_backup_tool::core::config::assets::DEFAULT_I18N_JSON;

fn load() -> Value {
    serde_json::from_str(DEFAULT_I18N_JSON).expect("i18n.json is invalid JSON")
}

fn keys(v: &Value, lang: &str) -> BTreeSet<String> {
    v[lang]
        .as_object()
        .unwrap_or_else(|| panic!("i18n.json has no `{}` section", lang))
        .keys()
        .cloned()
        .collect()
}

// =====================================================================
// en / ja のキー・プレースホルダの整合
// =====================================================================

#[test]
fn test_i18n_en_ja_have_same_keys() {
    let v = load();
    let en = keys(&v, "en");
    let ja = keys(&v, "ja");
    let only_en: Vec<_> = en.difference(&ja).collect();
    let only_ja: Vec<_> = ja.difference(&en).collect();
    assert!(only_en.is_empty(), "keys only in en: {:?}", only_en);
    assert!(only_ja.is_empty(), "keys only in ja: {:?}", only_ja);
}

#[test]
fn test_i18n_no_empty_values() {
    let v = load();
    for lang in ["en", "ja"] {
        for (k, val) in v[lang].as_object().unwrap() {
            let s = val.as_str().unwrap_or_else(|| panic!("{}.{} is not a string", lang, k));
            assert!(!s.trim().is_empty(), "{}.{} is empty", lang, k);
        }
    }
}

#[test]
fn test_i18n_placeholders_match_between_languages() {
    // {format} {current} {total} {tag} のような差し込み用の文字列が両言語で一致するか
    let re = Regex::new(r"\{[A-Za-z]+\}").unwrap();
    let v = load();
    for (k, en_val) in v["en"].as_object().unwrap() {
        let ja_val = v["ja"][k].as_str().unwrap_or("");
        let mut en_ph: Vec<_> = re
            .find_iter(en_val.as_str().unwrap())
            .map(|m| m.as_str().to_string())
            .collect();
        let mut ja_ph: Vec<_> = re.find_iter(ja_val).map(|m| m.as_str().to_string()).collect();
        en_ph.sort();
        ja_ph.sort();
        assert_eq!(en_ph, ja_ph, "placeholder mismatch in key `{}`", k);
    }
}

// =====================================================================
// ネイティブメニュー / トレイメニューが使うキーが存在するか
// =====================================================================

#[test]
fn test_i18n_has_all_menu_keys() {
    let v = load();
    let menu_keys = [
        // メニューバー
        "menuFile",
        "settings",
        "menuHelp",
        "generationArchive",
        "quit",
        "alwaysOnTop",
        "compactMode",
        "trayMode",
        "restoreState",
        "rebuildCacheOnStartup",
        "useSameDirForTemp",
        "showMemoAfterBackup",
        "language",
        "english",
        "japanese",
        "advancedSettingsTitle",
        "about",
        "aboutText",
        "restartRequired",
        // トレイメニュー
        "showWindow",
        "backupMode",
        "modeFull",
        "modeArc",
        "modeDiff",
        "executeBtn",
        "workFileBtn",
        "backupDirBtn",
        "trayTitle",
    ];
    for lang in ["en", "ja"] {
        for k in menu_keys {
            assert!(
                v[lang].get(k).is_some(),
                "menu key `{}` is missing in `{}`",
                k,
                lang
            );
        }
    }
}
