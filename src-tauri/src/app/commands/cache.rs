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
    let config = state.config.lock().unwrap();

    // 1. utils の共通関数を使用して、この作業ファイル専用のキャッシュパスを取得
    let cache_root = utils::get_cache_root(config.use_same_dir_for_temp, &backup_dir, &work_file);

    if cache_root.exists() {
        // 2. 直接削除を試みる
        if let Err(_) = fs::remove_dir_all(&cache_root) {
            // 削除に失敗した場合（ファイルが他で開かれている等）、リネームして隔離
            // これにより、新しい展開処理が古いゴミと混ざるのを確実に防ぐ
            let timestamp = chrono::Local::now().format("%H%M%S");
            let old_cache = cache_root.with_extension(format!("old_{}", timestamp));

            if let Err(_) = fs::rename(&cache_root, &old_cache) {
                // リネームすら失敗する場合は、ユーザーに通知
                return Err("キャッシュをクリアできませんでした。エクスプローラー等でキャッシュフォルダを閉じてください。".to_string());
            }

            // リネームに成功した「古いゴミ」は、消せれば消す（失敗しても次回起動時に掃除される）
            let _ = fs::remove_dir_all(&old_cache);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn prepare_archive_cache(
    app: AppHandle,
    archive_path: String,
    work_file: String,
    password: Option<String>,
) -> Result<String, String> {
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap();
    let archive_file = Path::new(&archive_path);

    if !archive_file.exists() {
        return Err("アーカイブファイルが見つかりません".to_string());
    }

    let backup_dir = archive_file
        .parent()
        .unwrap_or(Path::new(""))
        .to_string_lossy();
    let cache_root = utils::get_cache_root(config.use_same_dir_for_temp, &backup_dir, &work_file);
    fs::create_dir_all(&cache_root).map_err(|e| e.to_string())?;

    let f_name_lower = archive_path.to_lowercase();

    if f_name_lower.ends_with(".zip") {
        let file = File::open(archive_file).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

        for i in 0..archive.len() {
            let mut file = if let Some(ref p) = password {
                archive.by_index_decrypt(i, p.as_bytes())
            } else {
                archive.by_index(i)
            }
            .map_err(|e| format!("展開エラー: {}", e))?;

            if file.is_dir() {
                continue;
            }

            let raw_name = file.name();
            let sanitized_path_str = raw_name.replace('\u{F05C}', "/").replace('\\', "/");
            let rel_path = Path::new(&sanitized_path_str);

            // get_backup_list が認識できるように「base...」フォルダを特定して配置
            let components: Vec<_> = rel_path
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect();

            // パスの中から最初に見つかった "base" で始まるディレクトリ名を使用
            if let Some(base_dir_name) = components.iter().find(|c| c.starts_with("base")) {
                if let Some(file_name) = rel_path.file_name() {
                    let final_path = cache_root.join(base_dir_name.as_ref()).join(file_name);

                    if let Some(parent) = final_path.parent() {
                        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }

                    let mut outfile = File::create(&final_path).map_err(|e| e.to_string())?;
                    std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
                }
            }
        }
    } else if f_name_lower.contains(".tar.gz") || f_name_lower.ends_with(".tgz") {
        let tar_gz = File::open(archive_file).map_err(|e| e.to_string())?;
        let tar = flate2::read::GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        let entries = archive.entries().map_err(|e| e.to_string())?;

        for entry in entries {
            let mut entry = entry.map_err(|e| e.to_string())?;
            if !entry.header().entry_type().is_file() {
                continue;
            }

            let path_owned = entry
                .path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let sanitized_path_str = path_owned.replace('\u{F05C}', "/").replace('\\', "/");
            let rel_path = Path::new(&sanitized_path_str);

            let components: Vec<_> = rel_path
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect();

            if let Some(base_dir_name) = components.iter().find(|c| c.starts_with("base")) {
                if let Some(file_name) = rel_path.file_name() {
                    let final_path = cache_root.join(base_dir_name.as_ref()).join(file_name);

                    if let Some(parent) = final_path.parent() {
                        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                    entry.unpack(&final_path).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(cache_root.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn rebuild_archive_caches(
    app: AppHandle,
    work_file: String,
    backup_dir: String,
) -> Result<(), String> {
    // 1. まず古いキャッシュを一掃（混ざり防止）
    clear_all_caches(app.clone(), backup_dir.clone(), work_file.clone())?;

    let root = if backup_dir.is_empty() {
        utils::default_backup_dir(&work_file)
    } else {
        PathBuf::from(&backup_dir)
    };

    if !root.exists() {
        return Ok(());
    }

    // ワークファイルのステム名（拡張子なし）を取得して、関連アーカイブか判定する材料にする
    let file_stem = Path::new(&work_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    // 2. アーカイブを探して、正当なものだけ展開
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let f_name_lower = file_name.to_lowercase();

            // 判定条件:
            // ① 名前が "base" から始まる (世代アーカイブ)
            // ② かつ、拡張子が zip / tar.gz / tgz である
            let is_archive_ext = f_name_lower.ends_with(".zip")
                || f_name_lower.ends_with(".tar.gz")
                || f_name_lower.ends_with(".tgz");

            if f_name_lower.starts_with("base") && is_archive_ext {
                // 条件に合致するものだけを、専用サブフォルダへ展開
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
