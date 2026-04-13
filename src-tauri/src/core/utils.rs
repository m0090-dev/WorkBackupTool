use chrono::Local;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tar::Archive;
use tar::Builder;
use zip::write::SimpleFileOptions;
use zip::ZipArchive;
use zip::ZipWriter;
use zip::{AesMode, CompressionMethod};

/// ファイルを安全に移動させる。
/// デバイスを跨ぐ移動（リネーム失敗）時は、コピー＆削除でフォールバックする。
pub fn move_file_safe<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> Result<(), String> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    if let Err(_) = fs::rename(src, dst) {
        // renameが失敗した場合（異なるファイルシステム間など）、コピーして元のファイルを消す
        fs::copy(src, dst).map_err(|e| format!("ファイルのコピーに失敗しました: {}", e))?;
        fs::remove_file(src).map_err(|e| format!("元ファイルの削除に失敗しました: {}", e))?;
    }

    Ok(())
}

/// ファイル名からタイムスタンプを抽出する (Go版のロジック通り)
pub fn extract_timestamp_from_backup(path: &str) -> Result<String, String> {
    let base = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();

    let parts: Vec<&str> = base.split('.').collect();

    // test.clip.20251231_150000.diff -> 20251231_150000
    if parts.len() >= 3 {
        Ok(parts[parts.len() - 2].to_string())
    } else {
        Ok("No Timestamp".to_string())
    }
}

pub fn timestamped_name(original: &str) -> String {
    let path = Path::new(original);

    // 拡張子を除いたファイル名 (test.clip -> test)
    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();

    // 拡張子 (test.clip -> clip)
    let extension = path
        .extension()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();

    // 現在時刻をフォーマット
    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();

    // 拡張子がある場合とない場合で結合を分ける
    if extension.is_empty() {
        format!("{}_{}", file_stem, ts)
    } else {
        format!("{}_{}.{}", file_stem, ts, extension)
    }
}

pub fn auto_output_path(work_file: &str) -> String {
    let path = Path::new(work_file);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();
    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();

    let new_filename = if extension.is_empty() {
        format!("{}_restored_{}", file_stem, ts)
    } else {
        format!("{}_restored_{}.{}", file_stem, ts, extension)
    };

    dir.join(new_filename).to_string_lossy().into_owned()
}

/// デフォルトのバックアップディレクトリを返す
pub fn default_backup_dir(work_file: &str) -> PathBuf {
    let path = Path::new(work_file);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));

    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();

    // wbt_backup_ファイル名 フォルダ
    dir.join(format!("wbt_backup_{}", file_stem))
}

/// 単純なファイルコピーを行う (Go版の CopyFile 相当)
/// 親ディレクトリの作成、ストリームコピー、ディスク同期(Sync)を網羅
pub fn copy_file(src: &str, dst: &str) -> Result<(), String> {
    let src_path = Path::new(src);
    let dst_path = Path::new(dst);

    // 1. 出力先の親ディレクトリを MkdirAll (0755)
    if let Some(parent) = dst_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("ディレクトリ作成失敗: {}", e))?;
        }
    }

    // 2. 入力ファイルを開く (os.Open)
    let mut reader =
        File::open(src_path).map_err(|e| format!("入力ファイルが開けません {}: {}", src, e))?;

    // 3. 出力ファイルを作成 (os.Create)
    let mut writer = File::create(dst_path)
        .map_err(|e| format!("出力ファイルが作成できません {}: {}", dst, e))?;

    // 4. 内容をコピー (io.Copy)
    io::copy(&mut reader, &mut writer)
        .map_err(|e| format!("コピー中にエラーが発生しました: {}", e))?;

    // 5. ディスクに書き込みを確定させる (out.Sync)
    writer
        .sync_all()
        .map_err(|e| format!("ディスク同期に失敗しました: {}", e))?;

    Ok(())
}

/// Readerの内容をターゲットファイルに書き出す (Goの saveToWorkFile 相当)
/// Rustでは io::Read トレイトを持つものを引数に取ります
pub fn save_to_work_file<R: Read>(mut reader: R, target_file: &str) -> Result<(), String> {
    // 1. ファイルの作成
    let mut out = File::create(target_file)
        .map_err(|e| format!("Failed to create file {}: {}", target_file, e))?;

    // 2. データのコピー (io.Copy 相当)
    io::copy(&mut reader, &mut out).map_err(|e| format!("Failed to copy data: {}", e))?;

    // 3. ディスクへの書き込み確定 (Sync 相当)
    out.sync_all()
        .map_err(|e| format!("Failed to sync file: {}", e))?;

    Ok(())
}

pub fn get_cache_root(use_same_dir: bool, backup_dir: &str, work_file: &str) -> PathBuf {
    if use_same_dir {
        // 1. バックアップディレクトリと同じ場所（.wbt_cache）
        if backup_dir.is_empty() {
            default_backup_dir(work_file).join(".wbt_cache")
        } else {
            Path::new(backup_dir).join(".wbt_cache")
        }
    } else {
        // 2. OSのTempディレクトリを使う場合（衝突回避のためハッシュ付与）
        let mut s = DefaultHasher::new();
        work_file.hash(&mut s);
        let hash_val = format!("{:x}", s.finish());

        let file_stem = Path::new(work_file)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();

        // 例: temp/wbt_cache_myart_a1b2c3d4
        std::env::temp_dir().join(format!("wbt_cache_{}_{}", file_stem, &hash_val[..8]))
    }
}

/// ファイルのサイズを取得する
pub fn get_file_size(path: &str) -> Result<i64, String> {
    if path.is_empty() {
        return Err("path is empty".to_string());
    }
    let p = Path::new(path);
    let metadata = fs::metadata(p).map_err(|e| e.to_string())?;

    if metadata.is_dir() {
        return Err("path is a directory".to_string());
    }
    Ok(metadata.len() as i64)
}

/// テキストファイルを読み込む
pub fn read_text_file(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Ok("".to_string());
    }
    fs::read_to_string(p).map_err(|e| format!("Failed to read file: {}", e))
}

/// テキストファイルを書き込む (親ディレクトリがなければ作成)
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
