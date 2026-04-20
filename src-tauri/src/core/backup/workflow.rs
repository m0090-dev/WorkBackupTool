use crate::core::backup::auto_generation;
use crate::core::utils;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

pub struct BackupTargetInfo {
    pub target_dir: PathBuf,
    pub project_root: PathBuf,
    pub current_idx: i32,
}
pub enum DiffAlgo {
    HDiff,
    BsDiff, // 将来用
    Unknown,
}

pub fn resolve_backup_target(
    initial_path: PathBuf,
    work_file: &str,
) -> Result<BackupTargetInfo, String> {
    let folder_name = initial_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let mut current_idx = 0;
    let target_dir: PathBuf;
    let project_root: PathBuf;

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
            crate::core::backup::auto_generation::resolve_generation_dir(&project_root, work_file)?;
        target_dir = resolved_path;
        current_idx = idx;
    }

    Ok(BackupTargetInfo {
        target_dir,
        project_root,
        current_idx,
    })
}

pub fn should_transition_to_next_gen(work_size: u64, diff_size: u64, threshold: f64) -> bool {
    work_size > 100 * 1024 && (diff_size as f64) > (work_size as f64) * threshold
}

/// フェーズ1-2まで差分作成用の独自ワークフロー
/// フェーズ1: 最初の差分作成プラン（パスのセット）を計算する
pub fn prepare_initial_plan(
    work_file: &str,
    target: &BackupTargetInfo,
    ts: &str,
) -> Result<Option<(PathBuf, PathBuf, PathBuf)>, String> {
    let plan =
        crate::core::ext::hdiff_common::prepare_hdiff_paths(work_file, target.target_dir.clone())?;

    if let Some((base, work, _)) = plan {
        let file_name = Path::new(work_file).file_name().unwrap().to_string_lossy();
        let temp_diff = std::env::temp_dir().join(format!("{}.{}.tmp", file_name, ts));
        Ok(Some((base.into(), work.into(), temp_diff)))
    } else {
        Ok(None)
    }
}

/// フェーズ2: 一時ファイルの判定を行い、必要なら「次（世代交代）のプラン」を、不要なら「維持（移動）」を行う
pub fn finalize_or_next_plan(
    work_file: &str,
    temp_diff: PathBuf,
    target: &BackupTargetInfo,
    threshold: f64,
    algo: &str,
    ts: &str,
) -> Result<(String, Option<(PathBuf, PathBuf, PathBuf)>), String> {
    let work_size = fs::metadata(work_file).map_err(|e| e.to_string())?.len();
    let diff_size = fs::metadata(&temp_diff).map_err(|e| e.to_string())?.len();
    let file_name = Path::new(work_file).file_name().unwrap().to_string_lossy();

    if should_transition_to_next_gen(work_size, diff_size, threshold) {
        // --- 世代交代プランの作成 ---
        let _ = fs::remove_file(&temp_diff);

        let (new_gen_dir, _) = match auto_generation::get_latest_generation(&target.project_root)? {
            Some(info) if info.base_idx > target.current_idx => (info.dir_path, info.base_idx),
            _ => {
                let next_idx = target.current_idx + 1;
                let path = auto_generation::create_new_generation(
                    &target.project_root,
                    next_idx,
                    work_file,
                )?;
                (path, next_idx)
            }
        };

        let plan =
            crate::core::ext::hdiff_common::prepare_hdiff_paths(work_file, new_gen_dir.clone())?;
        if let Some((base, work, _)) = plan {
            let final_path = new_gen_dir.join(format!("{}.{}.{}.diff", file_name, ts, algo));
            let final_str = final_path.to_string_lossy().into_owned();

            return Ok((final_str, Some((base.into(), work.into(), final_path))));
        }
    } else {
        // --- 維持（一時ファイルを本番パスへ移動） ---
        let final_path = target
            .target_dir
            .join(format!("{}.{}.{}.diff", file_name, ts, algo));
        let final_str = final_path.to_string_lossy().into_owned();
        // クロスデバイス対応の移動ロジック
        utils::move_file_safe(&temp_diff, &final_path)?;
        return Ok((final_str, None));
    }

    Err("予期しないワークフローエラーが発生しました".to_string())
}

pub fn detect_diff_algo(path: &str) -> DiffAlgo {
    let name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if name.contains(".hdiff.") {
        DiffAlgo::HDiff
    } else if name.contains(".bsdiff.") {
        DiffAlgo::BsDiff
    } else {
        // 拡張子が含まれない古い形式などは、とりあえずHDiffで試す等の戦略
        DiffAlgo::Unknown
    }
}

/// シンプルなコピーバックアップのための準備と実行
/// 保存先のパスを返す
pub fn execute_copy_backup(src: &str, backup_dir: Option<PathBuf>) -> Result<String, String> {
    // 1. ターゲットディレクトリの決定
    let target_dir = match backup_dir {
        Some(dir) => dir,
        None => utils::default_backup_dir(src),
    };

    // 2. ディレクトリ作成
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("バックアップ先フォルダの作成に失敗しました: {}", e))?;
    }

    // 3. 命名規則 (core::utils にある想定)
    let new_filename = utils::timestamped_name(src);
    let dest_path = target_dir.join(new_filename);
    let dest_str = dest_path.to_string_lossy().into_owned();

    // 4. 実行 (core::utils::copy_file)
    utils::copy_file(src, &dest_str).map_err(|e| e.to_string())?;

    Ok(dest_str)
}
