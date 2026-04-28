// 標準ライブラリ
use std::fs;
use std::path::PathBuf;

// 外部クレート
use tauri::{AppHandle, Manager, State, WebviewWindow, Window};

// Tauriプラグイン

// 内部モジュール (自作)
use crate::app::state::AppState;
use crate::app::utils;
use crate::core::types::*;
use std::collections::HashMap;
use serde_json;

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    Ok(cfg.clone())
}

#[tauri::command]
pub async fn update_config_value(
    state: tauri::State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();

    match key.as_str() {
        // usize用
        "startupCacheLimit" => {
            cfg.startup_cache_limit = value.as_u64().unwrap_or(0) as usize;
        }
        // f64用 (閾値 0.0 ~ 1.0)
        "autoBaseGenerationThreshold" => {
            cfg.auto_base_generation_threshold = value.as_f64().unwrap_or(0.6);
        }
        "hdiffStrictHashCheck" => {
            cfg.hdiff_strict_hash_check = value.as_bool().unwrap_or(false);
        }
        "strictFileNameMatch" => {
            cfg.strict_file_name_match = value.as_bool().unwrap_or(true);
        }
        _ => return Err(format!("Unknown numeric config key: {}", key)),
    }

    drop(cfg);
    state.save()
}

#[tauri::command]
pub fn set_always_on_top(
    window: Window,
    state: State<'_, AppState>,
    flag: bool,
) -> Result<(), String> {
    // 1. ウィンドウの設定変更
    #[cfg(desktop)]
    {
        window.set_always_on_top(flag).map_err(|e| e.to_string())?;
    }
    // 2. 設定の保存
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.always_on_top = flag;
    }
    state.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_restore_previous_state(state: State<'_, AppState>) -> bool {
    state.config.lock().unwrap().restore_previous_state
}

#[tauri::command]
pub fn get_auto_base_generation_threshold(state: State<'_, AppState>) -> f64 {
    state.config.lock().unwrap().auto_base_generation_threshold
}

#[tauri::command]
pub fn get_rebuild_cache_on_startup(state: State<'_, AppState>) -> bool {
    state.config.lock().unwrap().rebuild_cache_on_startup
}
#[tauri::command]
pub fn get_show_memo_after_backup(state: State<'_, AppState>) -> bool {
    state.config.lock().unwrap().show_memo_after_backup
}

#[tauri::command]
pub fn get_startup_cache_limit(state: State<'_, AppState>) -> usize {
    state.config.lock().unwrap().startup_cache_limit
}

/// 特定のキーに対応する翻訳テキストを返す (Goの GetLanguageText 相当)
/// Rust内部のメニュー構築などで使用する場合、AppStateを引数に取る形で実装

#[tauri::command]
pub fn get_language_text(state: State<'_, AppState>, key: &str) -> Result<String, String> {
    state.translate(&key)
}

/// 現在の言語設定に基づいた辞書をまるごと返す (Goの GetI18N 相当)
#[tauri::command]
pub fn get_i18n(state: State<'_, AppState>) -> Result<HashMap<String, String>, String> {
    let lang = {
        let cfg = state.config.lock().unwrap();
        if cfg.language.is_empty() {
            "ja".to_string()
        } else {
            cfg.language.clone()
        }
    };

    Ok(state.i18n.get(&lang).cloned().unwrap_or_default())
}

/// 言語を切り替えて保存する (Goの SetLanguage 相当)
#[tauri::command]
pub fn set_language(state: State<'_, AppState>, lang: String) -> Result<(), String> {
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.language = lang;
    }
    // 前に作った state.save() を呼び出す
    state.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_config_dir(app: AppHandle) -> String {
    // Tauriの組み込み機能で設定ディレクトリを取得
    // 取得に失敗した場合はフォールバックとして "./config" を返す
    let config_dir: PathBuf = match app.path().app_config_dir() {
        Ok(path) => path,
        Err(_) => return "./config".to_string(),
    };

    // フォルダが存在しない場合は作成 (MkdirAll 相当)
    if !config_dir.exists() {
        let _ = fs::create_dir_all(&config_dir);
    }

    // JS側には文字列として返す
    config_dir.to_string_lossy().into_owned()
}

// コマンド用ラッパー
#[tauri::command]
pub async fn toggle_compact_mode(window: WebviewWindow, is_compact: bool) -> Result<(), String> {
    utils::apply_compact_mode(&window, is_compact).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_window_visibility(app: AppHandle, show: bool) -> Result<(), String> {
    utils::apply_window_visibility(app, show)
}

/// タブごとの session.json フィールドを更新する。
/// 現状は hdiffIgnoreList のみ対応。必要に応じてフィールドを追加してください。
///
/// - `session_path`: JS 側で GetConfigDir() + "/session.json" を渡す
/// - `tab_id`:       更新対象タブの id (number)
/// - `key`:          更新するフィールド名 (camelCase)
/// - `value`:        新しい値 (JSON Value)
#[tauri::command]
pub async fn update_session_tab_value(
    session_path: String,
    tab_id: u64,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    // --- session.json を読み込む ---
    let raw = fs::read_to_string(&session_path)
        .unwrap_or_else(|_| r#"{"tabs":[],"recentFiles":[]}"#.to_string());
    let mut session: SessionData =
        serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    // --- 対象タブを検索 ---
    let tab = session
        .tabs
        .iter_mut()
        .find(|t| t.id == tab_id)
        .ok_or_else(|| format!("Tab {} not found", tab_id))?;

    // --- フィールド更新 ---
    match key.as_str() {
        "hdiffIgnoreList" => {
            tab.hdiff_ignore_list = value
                .as_array()
                .ok_or("hdiffIgnoreList must be an array")?
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        _ => return Err(format!("Unknown session tab key: {}", key)),
    }

    // --- 書き戻す ---
    let out = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    fs::write(&session_path, out).map_err(|e| e.to_string())?;
    Ok(())
}
