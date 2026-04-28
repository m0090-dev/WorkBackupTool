use chrono::Local;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// ファイルを安全に移動させる。
pub fn move_file_safe<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> Result<(), String> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    if let Err(_) = fs::rename(src, dst) {
        fs::copy(src, dst).map_err(|e| format!("ファイルのコピーに失敗しました: {}", e))?;
        fs::remove_file(src).map_err(|e| format!("元ファイルの削除に失敗しました: {}", e))?;
    }

    Ok(())
}

/// ファイル名からタイムスタンプを抽出する
pub fn extract_timestamp_from_backup(path: &str) -> Result<String, String> {
    let base = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();

    let parts: Vec<&str> = base.split('.').collect();

    if parts.len() >= 3 {
        Ok(parts[parts.len() - 2].to_string())
    } else {
        Ok("No Timestamp".to_string())
    }
}

/// タイムスタンプ付きの名前を生成する
/// ファイル: `stem_YYYYMMDD_HHMMSS.ext`
/// フォルダ: `name_YYYYMMDD_HHMMSS`
pub fn timestamped_name(original: &str) -> String {
    let path = Path::new(original);
    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();

    if path.is_dir() || path.extension().is_none() {
        // フォルダまたは拡張子なし
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();
        format!("{}_{}", name, ts)
    } else {
        let file_stem = path
            .file_stem()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();
        let extension = path
            .extension()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();
        format!("{}_{}.{}", file_stem, ts, extension)
    }
}

/// 復元時の出力パスを自動生成する
/// フォルダの場合は `<name>_restored_<ts>/`、ファイルは従来通り
pub fn auto_output_path(work_path: &str) -> String {
    let path = Path::new(work_path);
    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();

    if path.is_dir() {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();
        dir.join(format!("{}_restored_{}", name, ts))
            .to_string_lossy()
            .into_owned()
    } else {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let file_stem = path
            .file_stem()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();
        let extension = path
            .extension()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();

        let new_filename = if extension.is_empty() {
            format!("{}_restored_{}", file_stem, ts)
        } else {
            format!("{}_restored_{}.{}", file_stem, ts, extension)
        };
        dir.join(new_filename).to_string_lossy().into_owned()
    }
}

/// デフォルトのバックアップディレクトリを返す
/// ファイル: `wbt_backup_<stem>/`
/// フォルダ: `wbt_backup_<foldername>/`
pub fn default_backup_dir(work_path: &str) -> PathBuf {
    let path = Path::new(work_path);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));

    let entry_name = if path.is_dir() {
        // フォルダの場合はフォルダ名そのまま
        path.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    };

    dir.join(format!("wbt_backup_{}", entry_name))
}

/// 単純なファイルコピーを行う
pub fn copy_file(src: &str, dst: &str) -> Result<(), String> {
    let src_path = Path::new(src);
    let dst_path = Path::new(dst);

    if let Some(parent) = dst_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("ディレクトリ作成失敗: {}", e))?;
        }
    }

    let mut reader =
        File::open(src_path).map_err(|e| format!("入力ファイルが開けません {}: {}", src, e))?;

    let mut writer = File::create(dst_path)
        .map_err(|e| format!("出力ファイルが作成できません {}: {}", dst, e))?;

    io::copy(&mut reader, &mut writer)
        .map_err(|e| format!("コピー中にエラーが発生しました: {}", e))?;

    writer
        .sync_all()
        .map_err(|e| format!("ディスク同期に失敗しました: {}", e))?;

    Ok(())
}

/// Readerの内容をターゲットファイルに書き出す
pub fn save_to_work_file<R: Read>(mut reader: R, target_file: &str) -> Result<(), String> {
    let mut out = File::create(target_file)
        .map_err(|e| format!("Failed to create file {}: {}", target_file, e))?;
    io::copy(&mut reader, &mut out).map_err(|e| format!("Failed to copy data: {}", e))?;
    out.sync_all()
        .map_err(|e| format!("Failed to sync file: {}", e))?;
    Ok(())
}

pub fn get_cache_root(use_same_dir: bool, backup_dir: &str, work_path: &str) -> PathBuf {
    if use_same_dir {
        if backup_dir.is_empty() {
            default_backup_dir(work_path).join(".wbt_cache")
        } else {
            Path::new(backup_dir).join(".wbt_cache")
        }
    } else {
        let mut s = DefaultHasher::new();
        work_path.hash(&mut s);
        let hash_val = format!("{:x}", s.finish());

        let path = Path::new(work_path);
        let entry_name = if path.is_dir() {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        } else {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        };

        std::env::temp_dir().join(format!("wbt_cache_{}_{}", entry_name, &hash_val[..8]))
    }
}

/// ファイルまたはフォルダのサイズを返す
/// フォルダの場合は配下のファイルサイズを合算
pub fn get_file_size(path: &str) -> Result<i64, String> {
    if path.is_empty() {
        return Err("path is empty".to_string());
    }
    let p = Path::new(path);
    let metadata = fs::metadata(p).map_err(|e| e.to_string())?;

    if metadata.is_dir() {
        let mut total = 0i64;
        for entry in walkdir::WalkDir::new(p)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            total += entry.metadata().map_err(|e| e.to_string())?.len() as i64;
        }
        Ok(total)
    } else {
        Ok(metadata.len() as i64)
    }
}

/// テキストファイルを読み込む
pub fn read_text_file(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Ok("".to_string());
    }
    fs::read_to_string(p).map_err(|e| format!("Failed to read file: {}", e))
}

/// テキストファイルを書き込む
pub fn write_text_file(path: &str, content: &str) -> Result<(), String> {
    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }
    fs::write(p, content).map_err(|e| format!("Failed to write text file: {}", e))
}

/// ディレクトリが存在するかチェック
pub fn dir_exists(path: &str) -> bool {
    Path::new(path).is_dir()
}

/// ファイルが存在するかチェック
pub fn file_exists(path: &str) -> bool {
    Path::new(path).is_file()
}
