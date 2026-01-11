use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
//use crate::app::types::R;
use crate::app::types::BackupGenInfo;

/// 最新の baseN_... フォルダを特定する
pub fn get_latest_generation(root: &Path) -> Result<Option<BackupGenInfo>, String> {
    if !root.exists() {
        return Ok(None);
    }

    let entries = fs::read_dir(root).map_err(|e| e.to_string())?;
    // base_連番_タイムスタンプ にマッチする正規表現
    let re = Regex::new(r"^base(\d+)_").unwrap();

    let mut latest_idx = -1;
    let mut latest_dir_name = String::new();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(caps) = re.captures(&name) {
                // indexを取得
                let idx = caps
                    .get(1)
                    .map_or(-1, |m| m.as_str().parse::<i32>().unwrap_or(-1));
                if idx > latest_idx {
                    latest_idx = idx;
                    latest_dir_name = name;
                }
            }
        }
    }

    if latest_idx == -1 {
        Ok(None)
    } else {
        Ok(Some(BackupGenInfo {
            dir_path: root.join(latest_dir_name),
            base_idx: latest_idx,
        }))
    }
}

/// 最新の世代フォルダを取得（なければ作成）
pub fn resolve_generation_dir(root: &Path, work_file: &str) -> Result<(PathBuf, i32), String> {
    match get_latest_generation(root)? {
        Some(info) => Ok((info.dir_path, info.base_idx)),
        None => {
            // 世代が一つもない場合は、インデックス 1 で新規作成
            let new_path = create_new_generation(root, 1, work_file)?;
            Ok((new_path, 1))
        }
    }
}

/// 新しい世代フォルダを作成し、.base をコピーする
pub fn create_new_generation(root: &Path, idx: i32, work_file: &str) -> Result<PathBuf, String> {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let new_dir_name = format!("base{}_{}", idx, ts);
    let new_dir_path = root.join(new_dir_name);

    // フォルダ作成 (mkdir -p)
    fs::create_dir_all(&new_dir_path).map_err(|e| e.to_string())?;

    // .base ファイルのコピー先パス
    let file_name = Path::new(work_file)
        .file_name()
        .ok_or_else(|| format!("Invalid work file name"))?
        .to_string_lossy();

    let base_path = new_dir_path.join(format!("{}.base", file_name));

    // 実ファイルのコピー (CopyFile相当)
    fs::copy(work_file, &base_path).map_err(|e| e.to_string())?;

    Ok(new_dir_path)
}

/// 新しい世代に切り替えるべきか判定する (ShouldRotate相当)
pub fn should_rotate(base_path: &Path, diff_path: &Path, threshold: f64) -> bool {
    let base_size = fs::metadata(base_path).map(|m| m.len()).unwrap_or(0);
    let diff_size = fs::metadata(diff_path).map(|m| m.len()).unwrap_or(0);

    if base_size == 0 {
        return false;
    }

    (diff_size as f64) > (base_size as f64) * threshold
}
