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
use crate::core::ext::hdiff_common::*;
use crate::core::types::BackupItem;
use crate::core::types::*;
use crate::core::{backup::auto_generation, utils};
use flate2::read::GzDecoder;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use tar::Archive;
use zip::ZipArchive;

#[tauri::command]
pub fn get_backup_list(
    app: AppHandle,
    work_file: String,
    backup_dir: String,
) -> Result<Vec<BackupItem>, String> {
    let mut list = Vec::new();
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap();
    let strict = config.strict_file_name_match;
    // --- 1. ルートディレクトリの決定 ---
    let root = if backup_dir.is_empty() {
        utils::default_backup_dir(&work_file)
    } else {
        PathBuf::from(&backup_dir)
    };

    if !root.exists() {
        return Ok(list);
    }

    // キャッシュルートの決定
    let cache_root = utils::get_cache_root(config.use_same_dir_for_temp, &backup_dir, &work_file);

    // ファイル名（拡張子なし）を取得
    let file_path_obj = Path::new(&work_file);
    let base_name_only = file_path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let file_path_ext: String = match file_path_obj.extension().and_then(|s| s.to_str()) {
        Some(ext) => format!(".{}", ext),
        None => String::new(),
    };
    let mut valid_exts = vec![
        ".diff".to_string(),
        ".zip".to_string(),
        ".tar.gz".to_string(),
        ".tar".to_string(),
        ".gz".to_string(),
    ];
    if !file_path_ext.is_empty() {
        valid_exts.push(file_path_ext.to_lowercase());
    }
    // 拡張子判定ヘルパー
    let is_valid_ext = |name: &str| -> bool {
        let n = name.to_lowercase();
        valid_exts
            .iter()
            .any(|ext| n.ends_with(&ext.to_lowercase()))
    };

    // --- 1. ルート直下のアーカイブをスキャン ---
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let f_name_lower = file_name.to_lowercase();
            let base_lower = base_name_only.to_lowercase();
            if (!strict || f_name_lower.contains(&base_lower)) && is_valid_ext(file_name) {
                if let Ok(metadata) = fs::metadata(&path) {
                    // 通常のルート直下ファイルはアーカイブフラグ false
                    list.push(create_backup_item(file_name, &path, &metadata, 0, false));
                }
            }
        }
    }

    // --- 2. すべての世代フォルダ(base*)をスキャン ---
    // 通常のroot直下と、アーカイブ展開済みのcache_rootの両方を走査対象にする
    let scan_roots = vec![
        (&root, false), // (パス, is_archived_flag)
        (&cache_root, true),
    ];

    for (current_root, is_archived_flag) in scan_roots {
        if !current_root.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(current_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                if dir_name.starts_with("base") {
                    let gen_idx: i32 = dir_name
                        .strip_prefix("base")
                        .and_then(|s| s.split('_').next())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);

                    if let Ok(gen_entries) = fs::read_dir(&path) {
                        for gen_entry in gen_entries.flatten() {
                            let gen_path = gen_entry.path();
                            if gen_path.is_dir() {
                                continue;
                            }
                            let f_name =
                                gen_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                            // 除外条件
                            if gen_path.is_dir() || f_name.ends_with(".base") {
                                continue;
                            }
                            let f_name_lower = f_name.to_lowercase();
                            let base_lower = base_name_only.to_lowercase();
                            if (!strict || f_name_lower.contains(&base_lower))
                                && is_valid_ext(f_name)
                            {
                                if let Ok(metadata) = fs::metadata(&gen_path) {
                                    let pure_name = gen_path
                                        .file_name()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or(f_name);
                                    list.push(create_backup_item(
                                        pure_name,
                                        &gen_path,
                                        &metadata,
                                        gen_idx,
                                        is_archived_flag, // ここで判定値を注入
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(list)
}

// ヘルパー関数: 拡張子チェック
fn is_valid_backup_ext(name: &str, exts: &[&str]) -> bool {
    exts.iter().any(|&ext| name.ends_with(ext))
}

// ヘルパー関数: アイテム生成 (日付フォーマット含む)
fn create_backup_item(
    name: &str,
    path: &Path,
    meta: &fs::Metadata,
    gen: i32,
    is_archived: bool,
) -> BackupItem {
    let modified: DateTime<Local> = meta
        .modified()
        .unwrap_or_else(|_| std::time::SystemTime::now())
        .into();
    BackupItem {
        file_name: name.to_string(),
        file_path: path.to_string_lossy().into_owned(),
        timestamp: modified.format("%Y-%m-%d %H:%M:%S").to_string(),
        file_size: meta.len() as i64,
        generation: gen,
        is_archived: is_archived,
    }
}

#[tauri::command]
pub fn get_generation_folders(
    work_file: String,
    backup_dir: String,
) -> Result<Vec<BackupItem>, String> {
    let root = if backup_dir.is_empty() {
        utils::default_backup_dir(&work_file)
    } else {
        PathBuf::from(&backup_dir)
    };

    if !root.exists() {
        return Ok(Vec::new());
    }

    // 既存の get_latest_generation を使って「最新」を特定しておく
    // (最新のフォルダは現在進行系で使っているのでアーカイブ対象から外すため)
    let latest_info = auto_generation::get_latest_generation(&root)?;
    let latest_path = latest_info.map(|i| i.dir_path);

    let mut list = Vec::new();
    let entries = fs::read_dir(&root).map_err(|e| e.to_string())?;
    let re = regex::Regex::new(r"^base(\d+)_").unwrap();

    for entry in entries.flatten() {
        if entry.file_type().map_or(false, |t| t.is_dir()) {
            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();

            if let Some(caps) = re.captures(&name) {
                // 最新のフォルダ（進行中の世代）はリストから除外する
                if let Some(ref lp) = latest_path {
                    if &path == lp {
                        continue;
                    }
                }

                let gen_idx = caps[1].parse::<i32>().unwrap_or(0);
                let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
                let modified: chrono::DateTime<chrono::Local> = metadata
                    .modified()
                    .map(|t| t.into())
                    .unwrap_or_else(|_| chrono::Local::now());

                // 既存の BackupItem 構造体を流用（JS側で扱いやすいため）
                list.push(BackupItem {
                    file_name: name,
                    file_path: path.to_string_lossy().to_string(),
                    timestamp: modified.format("%Y/%m/%d %H:%M").to_string(),
                    file_size: 0, // フォルダサイズ計算は重いので0でOK
                    generation: gen_idx,
                    is_archived: false,
                });
            }
        }
    }

    // 世代が古い順に並べて表示したい
    list.sort_by(|a, b| a.generation.cmp(&b.generation));
    Ok(list)
}
