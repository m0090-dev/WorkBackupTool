// 標準ライブラリ
use std::fs;
use std::path::{Path, PathBuf};

// 外部クレート
use chrono::{DateTime, Local};
use tauri::{AppHandle, Manager};

// Tauriプラグイン

// 内部モジュール (自作)
use crate::app::state::AppState;
use crate::core::types::BackupItem;
use crate::core::{backup::auto_generation, utils};

#[tauri::command]
pub fn get_backup_list(
    app: tauri::AppHandle,
    work_file: String,
    backup_dir: String,
) -> Result<Vec<BackupItem>, String> {
    // 1. AppStateから設定値を抜き出す
    let (strict, use_same_dir) = {
        let state = app.state::<crate::app::state::AppState>();
        let cfg = state.config.lock().unwrap();
        (cfg.strict_file_name_match, cfg.use_same_dir_for_temp)
    };

    // 2. core::backup::scanner の「scan_backups」を呼ぶ
    Ok(crate::core::backup::scanner::scan_backups(
        &work_file,
        &backup_dir,
        strict,
        use_same_dir,
    ))
}

#[tauri::command]
pub fn get_generation_folders(
    work_file: String,
    backup_dir: String,
) -> Result<Vec<BackupItem>, String> {
    crate::core::backup::scanner::scan_generation_folders(&work_file, &backup_dir)
}
