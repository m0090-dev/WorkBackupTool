use crate::core::utils::*;
use chrono::Local;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::fs::File;
use std::hash::Hasher;
use std::io::{self};
use std::path::{Path, PathBuf};
use tar::Archive;
use tar::Builder;
use zip::write::SimpleFileOptions;
use zip::ZipArchive;
use zip::ZipWriter;
use zip::{AesMode, CompressionMethod};

pub fn zip_backup_file(src: &str, backup_dir: &Path, password: &str) -> Result<(), String> {
    // 1. 保存先の決定 (既存ロジック維持)
    let stem = Path::new(src)
        .file_stem()
        .ok_or("Invalid source path")?
        .to_string_lossy();
    let zip_filename = timestamped_name(&format!("{}.zip", stem));
    let zip_path = backup_dir.join(zip_filename);

    let file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);

    // 2. オプション構築 (パスワードとAES暗号化を追加)
    // password引数を使用してAES256モードで暗号化を設定します
    let mut options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    if !password.is_empty() {
        options = options.with_aes_encryption(AesMode::Aes256, password);
    }

    // 3. アーカイブ内にファイルエントリー作成
    let file_name = Path::new(src)
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy();
    zip.start_file(file_name.to_string(), options)
        .map_err(|e| e.to_string())?;

    // 4. 内容のコピー
    let mut f = File::open(src).map_err(|e| e.to_string())?;
    io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;

    // 5. 書き込み確定
    zip.finish().map_err(|e| e.to_string())?;

    Ok(())
}

pub fn tar_backup_file(src: &str, backup_dir: &Path) -> Result<(), String> {
    let stem = Path::new(src).file_stem().unwrap().to_string_lossy();
    let tar_filename = timestamped_name(&format!("{}.tar.gz", stem));
    let tar_path = backup_dir.join(tar_filename);

    let file = File::create(&tar_path).map_err(|e| e.to_string())?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    let mut f = File::open(src).map_err(|e| e.to_string())?;

    // 修正ポイント: file_name を String に変換することで AsRef<Path> を満たすようにする
    let file_name = Path::new(src)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned(); // ここで String (owned data) に変換

    // tar.append_file は &String なら AsRef<Path> として受け取れるようになります
    tar.append_file(&file_name, &mut f)
        .map_err(|e| e.to_string())?;

    tar.finish().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn restore_archive(archive_path: &str, work_file: &str) -> Result<(), String> {
    let path = Path::new(archive_path);
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

    if file_name.ends_with(".zip") {
        let file = File::open(archive_path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

        if archive.len() > 0 {
            let mut file_in_zip = archive.by_index(0).map_err(|e| e.to_string())?;
            // 既存の utils 関数を呼び出し
            save_to_work_file(&mut file_in_zip, work_file)?;
            return Ok(());
        }
    } else if file_name.ends_with(".tar.gz") {
        let file = File::open(archive_path).map_err(|e| e.to_string())?;
        let tar_gz = GzDecoder::new(file);
        let mut archive = Archive::new(tar_gz);

        if let Some(Ok(mut entry)) = archive.entries().map_err(|e| e.to_string())?.next() {
            // 既存の utils 関数を呼び出し
            save_to_work_file(&mut entry, work_file)?;
            return Ok(());
        }
    }

    Err(format!(
        "サポートされていない形式、またはアーカイブが空です"
    ))
}

/// フォルダをZIP圧縮する内部関数
pub fn compress_dir_zip(src_dir: &Path, dst_file: &Path, password: &str) -> Result<(), String> {
    let file = File::create(dst_file).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);

    // パスワードがある場合のみAESを有効化
    let mut options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    if !password.is_empty() {
        options = options.with_aes_encryption(zip::AesMode::Aes256, password);
    }

    let walk = walkdir::WalkDir::new(src_dir);
    for entry in walk.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        // パス計算（src_dirの親からの相対にすると展開時にフォルダごと戻る）
        let name = path
            .strip_prefix(src_dir.parent().unwrap())
            .map_err(|e| e.to_string())?;
        let name_str = name.to_string_lossy().to_string();

        if path.is_file() {
            zip.start_file(name_str, options)
                .map_err(|e| e.to_string())?;
            let mut f = File::open(path).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
        } else if !name.as_os_str().is_empty() {
            zip.add_directory(name_str, options)
                .map_err(|e| e.to_string())?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
/// フォルダをTAR.GZ圧縮する内部関数
pub fn compress_dir_tar(src_dir: &Path, dst_file: &Path) -> Result<(), String> {
    let file = File::create(dst_file).map_err(|e| e.to_string())?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    // src_dir の終端（フォルダ名）をアーカイブ内のルートにする
    let folder_name = src_dir.file_name().ok_or("Invalid folder name")?;
    tar.append_dir_all(folder_name, src_dir)
        .map_err(|e| format!("TAR追加失敗: {}", e))?;

    tar.finish().map_err(|e| e.to_string())?;
    Ok(())
}
pub fn execute_archive_backup(
    src: &str,
    backup_dir_opt: Option<PathBuf>,
    format: &str,
    password: &str,
) -> Result<String, String> {
    // 1. バックアップ先ディレクトリの決定 (Noneならデフォルトを使用)
    let target_dir = match backup_dir_opt {
        Some(dir) => dir,
        None => default_backup_dir(src),
    };

    // 2. ディレクトリの存在保証
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("アーカイブ先ディレクトリの作成に失敗しました: {}", e))?;
    }

    // 3. フォーマットに応じて圧縮実行
    if format == "zip" {
        zip_backup_file(src, &target_dir, password)?;
    } else {
        tar_backup_file(src, &target_dir)?;
    }

    Ok("Archive created successfully".to_string())
}

pub fn execute_generation_archive(
    target_n: u32,
    format: &str,
    work_file: &str,
    backup_dir: &str,
    password: &str,
) -> Result<(), String> {
    // 1. パス解決
    let backup_path = if backup_dir.is_empty() {
        default_backup_dir(work_file)
    } else {
        PathBuf::from(backup_dir)
    };

    // 2. 対象フォルダの特定
    let prefix = format!("base{}_", target_n);
    let entries = fs::read_dir(&backup_path).map_err(|e| e.to_string())?;

    let src_path = entries
        .flatten()
        .find(|e| {
            e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && e.file_name().to_string_lossy().starts_with(&prefix)
        })
        .map(|e| e.path())
        .ok_or_else(|| format!("世代 {} のフォルダが見つかりません", target_n))?;

    // 3. 出力パス決定
    let folder_name = src_path.file_name().unwrap().to_string_lossy();
    let ext = if format == "tar" { "tar.gz" } else { "zip" };
    let dst_path = backup_path.join(format!("{}.{}", folder_name, ext));

    // 4. 圧縮実行
    if format == "tar" {
        compress_dir_tar(&src_path, &dst_path)?;
    } else {
        compress_dir_zip(&src_path, &dst_path, password)?;
    }

    // 5. 後始末 (安全確認付き)
    if dst_path.exists() && fs::metadata(&dst_path).map(|m| m.len()).unwrap_or(0) > 0 {
        fs::remove_dir_all(&src_path).map_err(|e| format!("フォルダ削除失敗: {}", e))?;
    } else {
        return Err("アーカイブ作成不完全のため削除を中止しました".to_string());
    }

    Ok(())
}

/// キャッシュディレクトリを一掃する
/// 削除できない場合はリネームして隔離し、新しい展開を妨げないようにする
pub fn clear_cache_directory(cache_root: &Path) -> Result<(), String> {
    if cache_root.exists() {
        if fs::remove_dir_all(cache_root).is_err() {
            let timestamp = Local::now().format("%H%M%S");
            let old_cache = cache_root.with_extension(format!("old_{}", timestamp));

            // 削除失敗時はリネームしてパスを空ける
            fs::rename(cache_root, &old_cache).map_err(|_| {
                "キャッシュをクリアできませんでした。フォルダを閉じているか確認してください。"
            })?;
            // リネームしたゴミは消せれば消す（失敗しても次回以降に掃除される）
            let _ = fs::remove_dir_all(&old_cache);
        }
    }
    Ok(())
}

/// アーカイブ（ZIP/TAR.GZ）をキャッシュルートへ展開する
pub fn extract_to_cache(
    archive_path: &str,
    cache_root: &Path,
    password: Option<String>,
) -> Result<String, String> {
    let archive_file = Path::new(archive_path);
    if !archive_file.exists() {
        return Err("アーカイブファイルが見つかりません".to_string());
    }

    fs::create_dir_all(cache_root).map_err(|e| e.to_string())?;
    let f_name_lower = archive_path.to_lowercase();

    if f_name_lower.ends_with(".zip") {
        extract_zip(archive_file, cache_root, password)?;
    } else if f_name_lower.contains(".tar.gz") || f_name_lower.ends_with(".tgz") {
        extract_targz(archive_file, cache_root)?;
    }

    Ok(cache_root.to_string_lossy().to_string())
}

fn extract_zip(
    archive_file: &Path,
    cache_root: &Path,
    password: Option<String>,
) -> Result<(), String> {
    let file = File::open(archive_file).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..zip.len() {
        let mut file = if let Some(ref p) = password {
            zip.by_index_decrypt(i, p.as_bytes())
        } else {
            zip.by_index(i)
        }
        .map_err(|e| format!("展開エラー: {}", e))?;

        if file.is_dir() {
            continue;
        }

        // パス正規化とbaseフォルダ特定
        let sanitized = file.name().replace('\u{F05C}', "/").replace('\\', "/");
        let rel_path = Path::new(&sanitized);

        if let Some(base_dir) = find_base_component(rel_path) {
            if let Some(fname) = rel_path.file_name() {
                let dest = cache_root.join(base_dir).join(fname);
                if let Some(p) = dest.parent() {
                    fs::create_dir_all(p).ok();
                }
                let mut outfile = File::create(&dest).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

fn extract_targz(archive_file: &Path, cache_root: &Path) -> Result<(), String> {
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
        let sanitized = path_owned.replace('\u{F05C}', "/").replace('\\', "/");
        let rel_path = Path::new(&sanitized);

        if let Some(base_dir) = find_base_component(rel_path) {
            if let Some(fname) = rel_path.file_name() {
                let dest = cache_root.join(base_dir).join(fname);
                if let Some(p) = dest.parent() {
                    fs::create_dir_all(p).ok();
                }
                entry.unpack(&dest).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

/// パスコンポーネントから "base" で始まるディレクトリ名を抽出する
fn find_base_component(path: &Path) -> Option<String> {
    path.components().find_map(|c| {
        let s = c.as_os_str().to_string_lossy();
        if s.starts_with("base") {
            Some(s.into_owned())
        } else {
            None
        }
    })
}
