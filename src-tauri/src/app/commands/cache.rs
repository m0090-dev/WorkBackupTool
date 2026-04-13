// 標準ライブラリ
use std::fs;
use std::path::{Path, PathBuf};

// 外部クレート
use tauri::{AppHandle, Manager};

// Tauriプラグイン

// 内部モジュール (自作)
use crate::app::state::AppState;
use crate::core::utils;
use std::fs::File;

#[tauri::command]
pub fn clear_all_caches(
    app: AppHandle,
    backup_dir: String,
    work_file: String,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap();
    let cache_root = utils::get_cache_root(cfg.use_same_dir_for_temp, &backup_dir, &work_file);

    crate::core::backup::archive::clear_cache_directory(&cache_root)
}

#[tauri::command]
pub async fn prepare_archive_cache(
    app: AppHandle,
    archive_path: String,
    work_file: String,
    password: Option<String>,
) -> Result<String, String> {
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap();

    let archive_file = Path::new(&archive_path);
    let backup_dir = archive_file
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let cache_root = utils::get_cache_root(cfg.use_same_dir_for_temp, &backup_dir, &work_file);

    crate::core::backup::archive::extract_to_cache(&archive_path, &cache_root, password)
}

#[tauri::command]
pub async fn rebuild_archive_caches(
    app: AppHandle,
    work_file: String,
    backup_dir: String,
) -> Result<(), String> {
    // 1. キャッシュの一掃
    clear_all_caches(app.clone(), backup_dir.clone(), work_file.clone())?;

    let root = if backup_dir.is_empty() {
        utils::default_backup_dir(&work_file)
    } else {
        PathBuf::from(&backup_dir)
    };
    if !root.exists() {
        return Ok(());
    }

    // 2. アーカイブをスキャンして順次キャッシュ化
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            let is_archive =
                name.ends_with(".zip") || name.ends_with(".tar.gz") || name.ends_with(".tgz");

            if name.starts_with("base") && is_archive {
                let _ = prepare_archive_cache(
                    app.clone(),
                    path.to_string_lossy().to_string(),
                    work_file.clone(),
                    None,
                )
                .await;
            }
        }
    }
    Ok(())
}
