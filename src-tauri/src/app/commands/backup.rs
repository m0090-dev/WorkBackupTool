// 標準ライブラリ
use std::fs;
use std::path::PathBuf;

// 外部クレート
use tauri::AppHandle;

// 内部モジュール (自作)
use crate::app::hdiff::*;
use crate::app::state::AppState;
use crate::core::backup::workflow;
use crate::core::{backup::archive, utils};
use tauri::Manager;

#[tauri::command]
pub async fn backup_or_diff(
    app: AppHandle,
    work_file: String,
    custom_dir: String,
    algo: String,
    compress: String,
) -> Result<String, String> {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    // 1. ディレクトリ解決
    let initial_path = if custom_dir.is_empty() {
        utils::default_backup_dir(&work_file)
    } else {
        PathBuf::from(custom_dir.trim_end_matches(|c| c == '/' || c == '\\'))
    };
    let target = workflow::resolve_backup_target(initial_path, &work_file)?;

    if !target.target_dir.exists() {
        fs::create_dir_all(&target.target_dir).map_err(|e| e.to_string())?;
    }
    let mut final_path_str = String::new();
    // 2. フェーズ1: 最初の作成
    if let Some((base, work, temp)) = workflow::prepare_initial_plan(&work_file, &target, &ts)? {
        crate::app::hdiff::create_hdiff(
            app.clone(),
            &base.to_string_lossy(),
            &work.to_string_lossy(),
            &temp.to_string_lossy(),
            &compress,
        )
        .await?;

        // 3. 判定用の閾値取得
        let threshold = {
            let state = app.state::<AppState>();
            let cfg = state.config.lock().unwrap();
            if cfg.auto_base_generation_threshold <= 0.0 {
                0.8
            } else {
                cfg.auto_base_generation_threshold
            }
        };
        let (path_str, next_plan) =
            workflow::finalize_or_next_plan(&work_file, temp, &target, threshold, &algo, &ts)?;

        final_path_str = path_str;
        // 4. フェーズ2: 判定と後始末（世代交代が必要なら次を実行）
        if let Some((new_base, new_work, final_dest)) = next_plan {
            crate::app::hdiff::create_hdiff(
                app,
                &new_base.to_string_lossy(),
                &new_work.to_string_lossy(),
                &final_dest.to_string_lossy(),
                &compress,
            )
            .await?;
        }
    }

    Ok(final_path_str)
}

#[tauri::command]
pub async fn apply_multi_diff(
    app: AppHandle,
    work_file: String,
    diff_paths: Vec<String>,
) -> Result<(), String> {
    let hdiff_strict_hash_check = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().unwrap();
        cfg.hdiff_strict_hash_check
    };

    for dp in diff_paths {
        let algo = workflow::detect_diff_algo(&dp);

        let result = match algo {
            workflow::DiffAlgo::HDiff => {
                apply_hdiff_wrapper(app.clone(), &work_file, &dp, hdiff_strict_hash_check).await
            }
            workflow::DiffAlgo::BsDiff => {
                return Err("`bsdiff` is not supported currently.".into());
            }
            _ => Err("Unknown format".into()),
        };

        if let Err(e) = result {
            return Err(format!("復元失敗 ({}): {}", dp, e));
        }
    }
    Ok(())
}

/// ファイルをそのままコピーしてバックアップする (Go版の CopyBackupFile 相当)
#[tauri::command]
pub fn copy_backup_file(src: String, backup_dir: String) -> Result<String, String> {
    // 1. 引数の加工 (app層の仕事)
    let dir_opt = if backup_dir.is_empty() {
        None
    } else {
        Some(PathBuf::from(backup_dir))
    };

    // 2. 実行 (ロジックはすべてcoreへ)
    workflow::execute_copy_backup(&src, dir_opt)
}

#[tauri::command]
pub async fn archive_backup_file(
    src: String,
    backup_dir: String,
    format: String,
    password: String,
) -> Result<String, String> {
    // 1. 引数の正規化
    let dir_opt = if backup_dir.is_empty() {
        None
    } else {
        Some(PathBuf::from(backup_dir))
    };

    // 2. coreのワークフローを呼び出す
    crate::core::backup::archive::execute_archive_backup(&src, dir_opt, &format, &password)
}

#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    path: String,
    work_file: String,
) -> Result<(), String> {
    let lower_path = path.to_lowercase();

    // 1. 差分パッチの場合のみSidecar（async/app層）が必要
    if lower_path.ends_with(".diff") {
        return apply_multi_diff(app, work_file, vec![path]).await;
    }

    // 出力パスの自動生成
    let restored_path = utils::auto_output_path(&work_file);

    if lower_path.ends_with(".zip") || lower_path.ends_with(".tar.gz") {
        // 2 & 3. アーカイブ展開 (core::backup::archive に丸投げ)
        archive::restore_archive(&path, &restored_path)
    } else {
        // 4. フルコピー (core::utils)
        utils::copy_file(&path, &restored_path)?;
        Ok(())
    }
}

#[tauri::command]
pub async fn archive_generation(
    target_n: u32,
    format: String,
    work_file: String,
    backup_dir: String,
    password: Option<String>,
) -> Result<(), String> {
    let pwd = password.unwrap_or_default();

    // core 側のワークフローに丸投げ
    crate::core::backup::archive::execute_generation_archive(
        target_n,
        &format,
        &work_file,
        &backup_dir,
        &pwd,
    )
}
