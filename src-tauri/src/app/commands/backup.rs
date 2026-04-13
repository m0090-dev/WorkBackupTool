// 標準ライブラリ
use std::fs;
use std::path::{Path, PathBuf};

// 外部クレート
use tauri::AppHandle;

// Tauriプラグイン

// 内部モジュール (自作)
use crate::app::hdiff::*;
use crate::core::{backup::auto_generation, utils};
use flate2::read::GzDecoder;
use std::fs::File;
use tar::Archive;
use zip::ZipArchive;

#[tauri::command]
pub async fn backup_or_diff(
    app: AppHandle,
    work_file: String,
    custom_dir: String,
    algo: String,
    compress: String,
) -> Result<(), String> {
    use crate::app::state::AppState;
    use regex::Regex;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tauri::Manager;

    // --- 1. ディレクトリの決定 ---
    let initial_path = if custom_dir.is_empty() {
        utils::default_backup_dir(&work_file)
    } else {
        PathBuf::from(custom_dir.trim_end_matches(|c| c == '/' || c == '\\'))
    };

    let target_dir: PathBuf;
    let mut current_idx: i32 = 0;
    let project_root: PathBuf;

    let folder_name = initial_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    // --- 1a. 手動選択された世代フォルダか、親フォルダかの判定 ---
    if folder_name.starts_with("base") {
        target_dir = initial_path.clone();
        project_root = initial_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| initial_path.clone());

        let re_idx = Regex::new(r"base(\d+)").unwrap();
        if let Some(caps) = re_idx.captures(folder_name) {
            current_idx = caps[1].parse().unwrap_or(0);
        }
    } else {
        project_root = initial_path.clone();
        let (resolved_path, idx) =
            auto_generation::resolve_generation_dir(&project_root, &work_file)?;
        target_dir = resolved_path;
        current_idx = idx;
    }

    // フォルダの存在保証
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    }

    let file_name = Path::new(&work_file)
        .file_name()
        .ok_or("Invalid work file name")?
        .to_string_lossy();

    // --- 2. 既存の .base 同期と一時差分作成 ---
    // ここで一旦、現在の target_dir に対して core の準備ロジックを走らせる
    let plan = crate::core::ext::hdiff_common::prepare_hdiff_paths(&work_file, target_dir.clone())?;

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let temp_diff = std::env::temp_dir().join(format!("{}.{}.tmp", file_name, ts));

    // Sidecar実行 (一時ファイルへ)
    if let Some((base_path, work, _)) = plan {
        crate::app::hdiff::create_hdiff(
            app.clone(),
            &base_path,
            &work,
            &temp_diff.to_string_lossy(),
            &compress,
        )
        .await?;
    } else {
        // .baseが新規作成（コピー）されただけの場合はここで終了
        return Ok(());
    }

    // --- 3. サイズ・閾値判定 ---
    let work_size = fs::metadata(&work_file).map_err(|e| e.to_string())?.len();
    let diff_size = fs::metadata(&temp_diff).map_err(|e| e.to_string())?.len();

    let threshold = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().unwrap();
        if cfg.auto_base_generation_threshold <= 0.0 {
            0.8
        } else {
            cfg.auto_base_generation_threshold
        }
    };

    let mut should_next_gen = false;
    if work_size > 100 * 1024 && (diff_size as f64) > (work_size as f64) * threshold {
        should_next_gen = true;
    }

    if should_next_gen {
        // --- 4a. 【世代交代】 ---
        let _ = fs::remove_file(&temp_diff);

        let (new_gen_dir, _) = match auto_generation::get_latest_generation(&project_root)? {
            Some(info) if info.base_idx > current_idx => (info.dir_path, info.base_idx),
            _ => {
                let next_idx = current_idx + 1;
                let path =
                    auto_generation::create_new_generation(&project_root, next_idx, &work_file)?;
                (path, next_idx)
            }
        };

        // 新しい世代に対して再度準備と実行
        let new_plan =
            crate::core::ext::hdiff_common::prepare_hdiff_paths(&work_file, new_gen_dir.clone())?;
        if let Some((base, work, _)) = new_plan {
            let final_path = new_gen_dir.join(format!("{}.{}.{}.diff", file_name, ts, algo));
            crate::app::hdiff::create_hdiff(
                app,
                &base,
                &work,
                &final_path.to_string_lossy(),
                &compress,
            )
            .await?;
        }
        Ok(())
    } else {
        // --- 4b. 【維持】 一時ファイルを正規の場所に移動 ---
        let final_path = target_dir.join(format!("{}.{}.{}.diff", file_name, ts, algo));
        if let Err(e) = fs::rename(&temp_diff, &final_path) {
            // クロスデバイス（WSL等の制限）対応のコピーフォールバック
            fs::copy(&temp_diff, &final_path).map_err(|e| e.to_string())?;
            fs::remove_file(&temp_diff).ok();
        }
        Ok(())
    }
}

#[tauri::command]
pub async fn apply_multi_diff(
    app: AppHandle,
    work_file: String,
    diff_paths: Vec<String>,
) -> Result<(), String> {
    for dp in diff_paths {
        let diff_name = Path::new(&dp)
            .file_name()
            .ok_or("Invalid path")?
            .to_string_lossy();

        let result = if diff_name.contains(".bsdiff.") {
            return Err(String::from("`bsdiff` is not supported."));
        } else if diff_name.contains(".hdiff.") {
            apply_hdiff_wrapper(app.clone(), work_file.as_str(), dp.as_str()).await
        } else {
            // 古いファイルのリトライ戦略
            match apply_hdiff_wrapper(app.clone(), work_file.as_str(), dp.as_str()).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    // bsdiffリトライの枠だけ
                    Err(format!("recovery failed for old format: {}", e))
                }
            }
        };

        if let Err(e) = result {
            return Err(format!("復元失敗 ({}): {}", diff_name, e));
        }
    }
    Ok(())
}

/// ファイルをそのままコピーしてバックアップする (Go版の CopyBackupFile 相当)
#[tauri::command]
pub fn copy_backup_file(src: String, backup_dir: String) -> Result<String, String> {
    // 1. バックアップ先ディレクトリの決定
    // backup_dir が空ならソースファイルに基づいたデフォルトディレクトリを作成
    let target_dir = if backup_dir.is_empty() {
        utils::default_backup_dir(&src)
    } else {
        PathBuf::from(backup_dir)
    };

    // 2. ディレクトリの作成 (MkdirAll 0755 相当)
    // utils::copy_file 内部でも作成していますが、Go版の構造に合わせここで明示的に作成
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("バックアップ先フォルダの作成に失敗しました: {}", e))?;
    }

    // 3. タイムスタンプ付きファイル名の生成 (例: filename_20260111_120000.ext)
    let new_filename = utils::timestamped_name(&src);

    // 4. 保存先のフルパスを組み立て
    let dest_path = target_dir.join(new_filename);
    let dest_str = dest_path.to_string_lossy();

    // 5. utils::copy_file (Sync処理付き) を実行
    utils::copy_file(&src, &dest_str).map_err(|e| e.to_string())?;

    // 6. 成功したら保存先のパスを返す (JS側での表示用)
    Ok(dest_str.into_owned())
}

#[tauri::command]
pub async fn archive_backup_file(
    src: String,
    backup_dir: String,
    format: String,
    password: String,
) -> Result<String, String> {
    // 1. バックアップ先の決定
    let target_dir = if backup_dir.is_empty() {
        utils::default_backup_dir(&src)
    } else {
        std::path::PathBuf::from(backup_dir)
    };

    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    }

    // 2. フォーマットによる分岐
    if format == "zip" {
        utils::zip_backup_file(&src, &target_dir, &password).map_err(|e| e.to_string())?;
    } else {
        utils::tar_backup_file(&src, &target_dir).map_err(|e| e.to_string())?;
    }

    Ok("Archive created successfully".to_string())
}

#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    path: String,
    work_file: String,
) -> Result<(), String> {
    let lower_path = path.to_lowercase();

    // 1. 差分パッチ (.diff)
    if lower_path.ends_with(".diff") {
        return apply_multi_diff(app, work_file, vec![path]).await;
    }

    // 復元先のパスを「別名」として自動生成
    let restored_path = utils::auto_output_path(&work_file);

    // 2. ZIPアーカイブ
    if lower_path.ends_with(".zip") {
        let file = File::open(&path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
        if archive.len() > 0 {
            let mut file_in_zip = archive.by_index(0).map_err(|e| e.to_string())?;
            return utils::save_to_work_file(&mut file_in_zip, &restored_path);
        }
    }

    // 3. TARアーカイブ (.tar.gz)
    if lower_path.ends_with(".tar.gz") {
        let file = File::open(&path).map_err(|e| e.to_string())?;
        let tar_gz = GzDecoder::new(file);
        let mut archive = Archive::new(tar_gz);
        if let Some(Ok(mut entry)) = archive.entries().map_err(|e| e.to_string())?.next() {
            return utils::save_to_work_file(&mut entry, &restored_path);
        }
    }

    // 4. フルコピー (.clip / .psd 等)
    // 既存の utils::copy_file を使用
    utils::copy_file(&path, &restored_path)?;
    Ok(())
}

#[tauri::command]
pub async fn archive_generation(
    target_n: u32,
    format: String,
    work_file: String,
    backup_dir: String,
    password: Option<String>,
) -> Result<(), String> {
    let backup_path = if backup_dir.is_empty() {
        utils::default_backup_dir(&work_file)
    } else {
        PathBuf::from(&backup_dir)
    };
    let pwd = password.unwrap_or_default();

    // 1. 対象となる世代フォルダ (baseN_timestamp) を特定
    // backup_dir 直下を走査し、"baseN_" で始まるディレクトリを探す
    let entries = fs::read_dir(&backup_path)
        .map_err(|e| format!("バックアップディレクトリにアクセスできません: {}", e))?;

    let prefix = format!("base{}_", target_n);
    let mut target_folder_path = None;

    for entry in entries.flatten() {
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(&prefix) {
                    target_folder_path = Some(entry.path());
                    break;
                }
            }
        }
    }

    let src_path = target_folder_path.ok_or_else(|| {
        format!(
            "世代 {} のフォルダが見つかりません (接頭辞: {})",
            target_n, prefix
        )
    })?;

    // 2. 出力ファイル名の決定 (元フォルダ名に拡張子を付与)
    let folder_name = src_path.file_name().unwrap().to_string_lossy();
    let ext = if format == "tar" { "tar.gz" } else { "zip" };
    let archive_filename = format!("{}.{}", folder_name, ext);
    let dst_path = backup_path.join(&archive_filename);

    // 3. 圧縮実行
    if format == "tar" {
        utils::compress_dir_tar(&src_path, &dst_path)?;
    } else {
        utils::compress_dir_zip(&src_path, &dst_path, &pwd)?;
    }

    // 4. 安全策: 成功確認（ファイル存在とサイズ）後に元フォルダを削除
    if dst_path.exists() && fs::metadata(&dst_path).map(|m| m.len()).unwrap_or(0) > 0 {
        fs::remove_dir_all(&src_path).map_err(|e| format!("フォルダ削除に失敗しました: {}", e))?;
    } else {
        return Err(
            "アーカイブ作成に失敗した可能性があるため、元データの削除を中止しました".to_string(),
        );
    }

    Ok(())
}
