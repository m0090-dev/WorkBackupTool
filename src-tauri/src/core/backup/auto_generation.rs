use crate::core::types::BackupGenInfo;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

/// 最新の baseN_... フォルダを特定する
pub fn get_latest_generation(root: &Path) -> Result<Option<BackupGenInfo>, String> {
    if !root.exists() {
        return Ok(None);
    }

    let entries = fs::read_dir(root).map_err(|e| e.to_string())?;
    let re = Regex::new(r"^base(\d+)_").unwrap();

    let mut latest_idx = -1;
    let mut latest_dir_name: Option<String> = None;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(caps) = re.captures(&name) {
                if let Ok(idx) = caps[1].parse::<i32>() {
                    if idx > latest_idx
                        || (idx == latest_idx
                            && latest_dir_name.as_ref().map_or(true, |n| &name >= n))
                    {
                        latest_idx = idx;
                        latest_dir_name = Some(name);
                    }
                }
            }
        }
    }

    match latest_dir_name {
        Some(name) => Ok(Some(BackupGenInfo {
            dir_path: root.join(name),
            base_idx: latest_idx,
        })),
        None => Ok(None),
    }
}

/// 最新の世代フォルダを取得（なければ作成）
pub fn resolve_generation_dir(root: &Path, work_path: &str) -> Result<(PathBuf, i32), String> {
    match get_latest_generation(root)? {
        Some(info) => Ok((info.dir_path, info.base_idx)),
        None => {
            let new_path = create_new_generation(root, 1, work_path)?;
            Ok((new_path, 1))
        }
    }
}

/// 新しい世代フォルダを作成し、base スナップショットをコピーする
/// - ファイルの場合: `<name>.base` ファイルをコピー
/// - フォルダの場合: `<name>.base/` フォルダごとコピー（fs_extra 使用）
///
/// 例（フォルダ）:
///   my_project.base/
///   ├── page_01.clip
///   ├── page_02.clip
///   └── ...
pub fn create_new_generation(root: &Path, idx: i32, work_path: &str) -> Result<PathBuf, String> {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let new_dir_name = format!("base{}_{}", idx, ts);
    let new_dir_path = root.join(new_dir_name);

    fs::create_dir_all(&new_dir_path).map_err(|e| e.to_string())?;

    let src = Path::new(work_path);
    let entry_name = src
        .file_name()
        .ok_or_else(|| "Invalid work path name".to_string())?
        .to_string_lossy();

    if src.is_dir() {
        // フォルダの場合: <name>.base/ として丸ごとコピー
        let base_dir = new_dir_path.join(format!("{}.base", entry_name));
        fs_extra::dir::copy(
            src,
            &base_dir,
            &fs_extra::dir::CopyOptions {
                copy_inside: true,
                ..Default::default()
            },
        )
        .map_err(|e| format!("Failed to copy base folder: {}", e))?;
    } else {
        // ファイルの場合: <name>.base ファイルとしてコピー（従来どおり）
        let base_path = new_dir_path.join(format!("{}.base", entry_name));
        fs::copy(work_path, &base_path).map_err(|e| format!("Failed to copy base file: {}", e))?;
    }

    Ok(new_dir_path)
}

/// 新しい世代に切り替えるべきか判定する
pub fn should_rotate(base_path: &Path, diff_path: &Path, threshold: f64) -> bool {
    let base_size = fs::metadata(base_path).map(|m| m.len()).unwrap_or(0);
    let diff_size = fs::metadata(diff_path).map(|m| m.len()).unwrap_or(0);

    if base_size == 0 {
        return false;
    }

    (diff_size as f64) > (base_size as f64) * threshold
}
