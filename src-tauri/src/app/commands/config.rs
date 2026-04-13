// 標準ライブラリ
use std::fs;
use std::path::{Path, PathBuf};

// 外部クレート
use chrono::{DateTime, Local};
use tauri::{AppHandle, LogicalSize, Manager, Size, State, WebviewWindow, Window};

// Tauriプラグイン
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::ShellExt;

// 内部モジュール (自作)
use crate::app::hdiff::*;
use crate::app::state::AppState;
use crate::app::utils;
use crate::core::backup::auto_generation;
use crate::core::ext::hdiff_common::*;
use crate::core::types::BackupItem;
use crate::core::types::*;
use flate2::read::GzDecoder;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use tar::Archive;
use zip::ZipArchive;

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
