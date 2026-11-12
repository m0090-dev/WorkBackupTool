//! アーカイブ(ZIP / TAR.GZ)の作成・復元・展開。
//!
//! - 作成: ファイル単体 / フォルダ。ZIP はパスワード(AES-256)付きも作れる。
//!   途中で失敗したときは、作りかけのアーカイブを残さない(履歴に壊れたものが出ないように)。
//! - 復元: ファイル単体のアーカイブはファイルとして、フォルダのアーカイブはフォルダとして書き出す。
//!   パスワード付き ZIP は、パスワードが無い/違うときに専用のエラー(`ERR_PASSWORD_*`)を返す。
//! - 世代の圧縮: 世代フォルダ(baseN_...)を1つのアーカイブにまとめ、検証してから元のフォルダを消す。
//! - キャッシュ展開: 圧縮済みの世代を、フォルダ構造を保ったまま一時フォルダに展開する。
//!
//! アーカイブ内のパスは、「..」や絶対パスを含むものを拒否する(展開先の外へ書き出されないように)。

use crate::core::utils::*;
use chrono::Local;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};
use tar::{Archive, Builder};
use walkdir::WalkDir;
use zip::result::ZipError;
use zip::write::{FileOptions, SimpleFileOptions};
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter};

/// パスワード付きの ZIP に、パスワードが渡されなかった(フロントはこの文字列を見てパスワードを尋ねる)
pub const ERR_PASSWORD_REQUIRED: &str = "ARCHIVE_PASSWORD_REQUIRED";
/// パスワードが違う
pub const ERR_PASSWORD_INVALID: &str = "ARCHIVE_PASSWORD_INVALID";

// ---------------------------------------------------------------- 共通の部品

fn now_ts() -> String {
    Local::now().format("%Y%m%d_%H%M%S").to_string()
}

/// `<base>.<ext>` を、既に存在すれば `<base>_2.<ext>` ... にして返す。
/// ext は `tar.gz` のように「.」を含んでもよい(拡張子を壊さない)。
fn unique_archive_path(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{}.{}", base, ext));
    if !first.exists() {
        return first;
    }
    for n in 2..10000 {
        let candidate = dir.join(format!("{}_{}.{}", base, n, ext));
        if !candidate.exists() {
            return candidate;
        }
    }
    first
}

/// アーカイブ内のパスを、安全な相対パスにする。
/// 「..」・絶対パス・ドライブ指定を含むものは None(展開先の外へ書き出されるのを防ぐ)。
fn safe_relative_path(name: &str) -> Option<PathBuf> {
    // 一部のツールが作る「\」の代用文字(U+F05C)も、区切りとして扱う
    let normalized = name.replace('\u{F05C}', "/").replace('\\', "/");
    let mut out = PathBuf::new();
    for comp in Path::new(&normalized).components() {
        match comp {
            Component::Normal(c) => out.push(c),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

/// アーカイブ内の名前(区切りは常に「/」)
fn entry_name(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn zip_file_options(password: &str, size: u64) -> FileOptions<'_, ()> {
    let mut options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    // 4GiB 以上のファイルは、ZIP64 の指定が要る
    if size >= 0xFFFF_FFFF {
        options = options.large_file(true);
    }
    if !password.is_empty() {
        options = options.with_aes_encryption(AesMode::Aes256, password);
    }
    options
}

/// 処理が失敗したら、作りかけのファイルを消す
fn with_cleanup<T>(dst: &Path, work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let result = work();
    if result.is_err() {
        let _ = fs::remove_file(dst);
    }
    result
}

fn remove_path(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

fn finish_targz(tar: Builder<GzEncoder<File>>) -> Result<(), String> {
    // Builder を閉じ、gzip の終端まで確実に書き切る(drop 任せだと失敗に気づけない)
    let enc = tar.into_inner().map_err(|e| e.to_string())?;
    let file = enc.finish().map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- 作成

/// ファイル単体をZIP圧縮する
pub fn zip_backup_file(src: &str, backup_dir: &Path, password: &str) -> Result<String, String> {
    let src_path = Path::new(src);
    let stem = src_path
        .file_stem()
        .ok_or("Invalid source path")?
        .to_string_lossy();
    let file_name = src_path
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy()
        .into_owned();
    let size = fs::metadata(src_path).map_err(|e| e.to_string())?.len();
    let zip_path = unique_archive_path(backup_dir, &format!("{}_{}", stem, now_ts()), "zip");

    with_cleanup(&zip_path, || {
        let file = File::create(&zip_path).map_err(|e| e.to_string())?;
        let mut zip = ZipWriter::new(file);
        zip.start_file(file_name, zip_file_options(password, size))
            .map_err(|e| e.to_string())?;
        let mut f = File::open(src_path).map_err(|e| e.to_string())?;
        io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
        zip.finish().map_err(|e| e.to_string())?;
        Ok(())
    })?;

    Ok(zip_path.to_string_lossy().into_owned())
}

/// ファイル単体をTAR.GZ圧縮する
pub fn tar_backup_file(src: &str, backup_dir: &Path) -> Result<String, String> {
    let src_path = Path::new(src);
    let stem = src_path
        .file_stem()
        .ok_or("Invalid source path")?
        .to_string_lossy();
    let file_name = src_path
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy()
        .into_owned();
    let tar_path = unique_archive_path(backup_dir, &format!("{}_{}", stem, now_ts()), "tar.gz");

    with_cleanup(&tar_path, || {
        let file = File::create(&tar_path).map_err(|e| e.to_string())?;
        let mut tar = Builder::new(GzEncoder::new(file, Compression::default()));
        let mut f = File::open(src_path).map_err(|e| e.to_string())?;
        tar.append_file(&file_name, &mut f)
            .map_err(|e| e.to_string())?;
        finish_targz(tar)
    })?;

    Ok(tar_path.to_string_lossy().into_owned())
}

/// フォルダをZIP圧縮する(中身は `<フォルダ名>/...` の形で入る)
pub fn compress_dir_zip(src_dir: &Path, dst_file: &Path, password: &str) -> Result<(), String> {
    let parent = src_dir.parent().unwrap_or_else(|| Path::new(""));
    let file =
        File::create(dst_file).map_err(|e| format!("アーカイブを作成できません: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let dir_options = SimpleFileOptions::default().unix_permissions(0o755);

    for entry in WalkDir::new(src_dir).sort_by_file_name() {
        // 読めないものを黙って飛ばすと、欠けたバックアップになるので、エラーにする
        let entry = entry.map_err(|e| format!("フォルダの走査に失敗しました: {}", e))?;
        let path = entry.path();
        if path == dst_file {
            continue;
        }
        let rel = path.strip_prefix(parent).map_err(|e| e.to_string())?;
        let name = entry_name(rel);
        if name.is_empty() {
            continue;
        }

        if entry.file_type().is_dir() {
            zip.add_directory(name, dir_options)
                .map_err(|e| e.to_string())?;
        } else if entry.file_type().is_file() {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            zip.start_file(name, zip_file_options(password, size))
                .map_err(|e| e.to_string())?;
            let mut f = File::open(path)
                .map_err(|e| format!("ファイルを開けません {}: {}", path.display(), e))?;
            io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

/// フォルダをTAR.GZ圧縮する(中身は `<フォルダ名>/...` の形で入る)
pub fn compress_dir_tar(src_dir: &Path, dst_file: &Path) -> Result<(), String> {
    let parent = src_dir.parent().unwrap_or_else(|| Path::new(""));
    let file =
        File::create(dst_file).map_err(|e| format!("アーカイブを作成できません: {}", e))?;
    let mut tar = Builder::new(GzEncoder::new(file, Compression::default()));

    for entry in WalkDir::new(src_dir).sort_by_file_name() {
        let entry = entry.map_err(|e| format!("フォルダの走査に失敗しました: {}", e))?;
        let path = entry.path();
        if path == dst_file {
            continue;
        }
        let rel = path.strip_prefix(parent).map_err(|e| e.to_string())?;
        let name = entry_name(rel);
        if name.is_empty() {
            continue;
        }

        if entry.file_type().is_dir() {
            tar.append_dir(&name, path)
                .map_err(|e| format!("TAR追加失敗: {}", e))?;
        } else if entry.file_type().is_file() {
            tar.append_path_with_name(path, &name)
                .map_err(|e| format!("TAR追加失敗 {}: {}", path.display(), e))?;
        }
    }
    finish_targz(tar)
}

/// バックアップを実行する（ファイル・フォルダ両対応）
pub fn execute_archive_backup(
    src: &str,
    backup_dir_opt: Option<PathBuf>,
    format: &str,
    password: &str,
) -> Result<String, String> {
    let src_path = Path::new(src);
    if !src_path.exists() {
        return Err(format!("バックアップ元が見つかりません: {}", src));
    }

    let target_dir = match backup_dir_opt {
        Some(dir) => dir,
        None => default_backup_dir(src),
    };

    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("アーカイブ先ディレクトリの作成に失敗しました: {}", e))?;
    }

    if src_path.is_dir() {
        // フォルダの場合: compress_dir_zip / compress_dir_tar を使う
        let folder_name = src_path
            .file_name()
            .ok_or("Invalid folder name")?
            .to_string_lossy();
        let base = format!("{}_{}", folder_name, now_ts());
        let is_zip = format == "zip";
        let dst_path = unique_archive_path(&target_dir, &base, if is_zip { "zip" } else { "tar.gz" });

        with_cleanup(&dst_path, || {
            if is_zip {
                compress_dir_zip(src_path, &dst_path, password)
            } else {
                compress_dir_tar(src_path, &dst_path)
            }
        })?;
        Ok(dst_path.to_string_lossy().into_owned())
    } else if format == "zip" {
        zip_backup_file(src, &target_dir, password)
    } else {
        tar_backup_file(src, &target_dir)
    }
}

// ---------------------------------------------------------------- 復元

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveKind {
    /// ファイル単体のアーカイブ(中身は1ファイルだけ)
    File,
    /// フォルダのアーカイブ(`<フォルダ名>/...`)
    Folder,
}

struct ExtractPlan {
    kind: ArchiveKind,
    /// フォルダのアーカイブで、全エントリが共有する最上位のフォルダ名(復元先では取り除く)
    root: Option<OsString>,
}

impl ExtractPlan {
    /// アーカイブ内の相対パスから、展開先のパスを決める
    fn dest(&self, out: &Path, rel: &Path) -> Option<PathBuf> {
        match (&self.kind, &self.root) {
            (ArchiveKind::File, _) => Some(out.to_path_buf()),
            (ArchiveKind::Folder, Some(root)) => {
                let rest = rel.strip_prefix(root).ok()?;
                if rest.as_os_str().is_empty() {
                    Some(out.to_path_buf())
                } else {
                    Some(out.join(rest))
                }
            }
            (ArchiveKind::Folder, None) => Some(out.join(rel)),
        }
    }
}

/// 中身(相対パス, フォルダか)から、ファイル/フォルダのどちらのアーカイブかと、展開の仕方を決める
fn plan_extraction(entries: &[(PathBuf, bool)]) -> Result<ExtractPlan, String> {
    if entries.is_empty() {
        return Err("サポートされていない形式、またはアーカイブが空です".to_string());
    }
    if entries.len() == 1 && !entries[0].1 && entries[0].0.components().count() == 1 {
        return Ok(ExtractPlan {
            kind: ArchiveKind::File,
            root: None,
        });
    }

    let first: Option<OsString> = entries[0]
        .0
        .components()
        .next()
        .map(|c| c.as_os_str().to_os_string());
    let root = first.filter(|f| {
        entries
            .iter()
            .all(|(p, _)| p.components().next().map(|c| c.as_os_str()) == Some(f.as_os_str()))
    });
    Ok(ExtractPlan {
        kind: ArchiveKind::Folder,
        root,
    })
}

fn map_zip_error(e: ZipError) -> String {
    match e {
        ZipError::InvalidPassword => ERR_PASSWORD_INVALID.to_string(),
        ZipError::UnsupportedArchive(msg) if msg == ZipError::PASSWORD_REQUIRED => {
            ERR_PASSWORD_REQUIRED.to_string()
        }
        other => format!("展開エラー: {}", other),
    }
}

/// ZIP の1エントリを dest に書き出す。パスワード付きのエントリは、パスワードで開く。
fn extract_zip_entry(
    zip: &mut ZipArchive<File>,
    index: usize,
    password: Option<&str>,
    dest: &Path,
) -> Result<(), String> {
    // パスワード付きかどうかは、まずパスワードなしで開いてみて確かめる
    let encrypted = matches!(
        zip.by_index(index),
        Err(ZipError::UnsupportedArchive(msg)) if msg == ZipError::PASSWORD_REQUIRED
    );

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut entry = if encrypted {
        let pw = password.ok_or_else(|| ERR_PASSWORD_REQUIRED.to_string())?;
        zip.by_index_decrypt(index, pw.as_bytes())
            .map_err(map_zip_error)?
    } else {
        zip.by_index(index).map_err(map_zip_error)?
    };

    let mut out = File::create(dest)
        .map_err(|e| format!("出力ファイルを作成できません {}: {}", dest.display(), e))?;
    io::copy(&mut entry, &mut out).map_err(|e| {
        if encrypted {
            // パスワードが違うと、最後のチェックサムの確認で失敗する
            ERR_PASSWORD_INVALID.to_string()
        } else {
            format!("展開エラー: {}", e)
        }
    })?;
    out.sync_all().map_err(|e| e.to_string())
}

fn restore_zip(archive_file: &Path, out: &Path, password: Option<&str>) -> Result<(), String> {
    let file = File::open(archive_file).map_err(|e| format!("アーカイブを開けません: {}", e))?;
    let mut zip = ZipArchive::new(file).map_err(|e| format!("ZIPを開けません: {}", e))?;

    // 1. 中身を調べる(安全でないパスは、展開を始める前に拒否する)
    let mut entries: Vec<(PathBuf, bool)> = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let entry = zip.by_index_raw(i).map_err(|e| format!("展開エラー: {}", e))?;
        let name = entry.name().to_string();
        let rel = safe_relative_path(&name)
            .ok_or_else(|| format!("安全でないパスが含まれています: {}", name))?;
        entries.push((rel, entry.is_dir()));
    }
    let plan = plan_extraction(&entries)?;

    // 2. 展開する(失敗したら、今回作った書き出し先は消す)
    let existed = out.exists();
    let result = (|| -> Result<(), String> {
        if plan.kind == ArchiveKind::Folder {
            fs::create_dir_all(out).map_err(|e| e.to_string())?;
        }
        for (i, (rel, is_dir)) in entries.iter().enumerate() {
            let Some(dest) = plan.dest(out, rel) else {
                continue;
            };
            if *is_dir {
                fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
            } else {
                extract_zip_entry(&mut zip, i, password, &dest)?;
            }
        }
        Ok(())
    })();
    if result.is_err() && !existed {
        remove_path(out);
    }
    result
}

/// TAR.GZ の中身(相対パス, フォルダか)を調べる。ファイルとフォルダ以外(リンクなど)は無視する
fn list_targz(archive_file: &Path) -> Result<Vec<(PathBuf, bool)>, String> {
    let file = File::open(archive_file).map_err(|e| format!("アーカイブを開けません: {}", e))?;
    let mut archive = Archive::new(GzDecoder::new(file));
    let mut entries = Vec::new();
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| format!("展開エラー: {}", e))?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            continue;
        }
        let name = entry
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|e| e.to_string())?;
        let rel = safe_relative_path(&name)
            .ok_or_else(|| format!("安全でないパスが含まれています: {}", name))?;
        entries.push((rel, kind.is_dir()));
    }
    Ok(entries)
}

fn restore_targz(archive_file: &Path, out: &Path) -> Result<(), String> {
    let entries = list_targz(archive_file)?;
    let plan = plan_extraction(&entries)?;

    let existed = out.exists();
    let result = (|| -> Result<(), String> {
        if plan.kind == ArchiveKind::Folder {
            fs::create_dir_all(out).map_err(|e| e.to_string())?;
        }
        let file = File::open(archive_file).map_err(|e| e.to_string())?;
        let mut archive = Archive::new(GzDecoder::new(file));
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| format!("展開エラー: {}", e))?;
            let kind = entry.header().entry_type();
            if !(kind.is_file() || kind.is_dir()) {
                continue;
            }
            let name = entry
                .path()
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|e| e.to_string())?;
            let Some(rel) = safe_relative_path(&name) else {
                continue;
            };
            let Some(dest) = plan.dest(out, &rel) else {
                continue;
            };
            if kind.is_dir() {
                fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut f = File::create(&dest).map_err(|e| {
                    format!("出力ファイルを作成できません {}: {}", dest.display(), e)
                })?;
                io::copy(&mut entry, &mut f).map_err(|e| format!("展開エラー: {}", e))?;
                f.sync_all().map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })();
    if result.is_err() && !existed {
        remove_path(out);
    }
    result
}

/// アーカイブを復元する(パスワードなし)。
/// ファイル単体のアーカイブは out_path のファイルへ、フォルダのアーカイブは out_path のフォルダへ書き出す。
pub fn restore_archive(archive_path: &str, work_path: &str) -> Result<(), String> {
    restore_archive_with_password(archive_path, work_path, None)
}

/// アーカイブを復元する(パスワード付き ZIP に対応)。
/// パスワードが無い/違うときは、`ERR_PASSWORD_REQUIRED` / `ERR_PASSWORD_INVALID` を返す。
pub fn restore_archive_with_password(
    archive_path: &str,
    out_path: &str,
    password: Option<&str>,
) -> Result<(), String> {
    let lower = archive_path.to_lowercase();
    let archive_file = Path::new(archive_path);
    let out = Path::new(out_path);
    // 空のパスワードは「なし」として扱う
    let password = password.filter(|p| !p.is_empty());

    if lower.ends_with(".zip") {
        restore_zip(archive_file, out, password)
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        restore_targz(archive_file, out)
    } else {
        Err("サポートされていない形式、またはアーカイブが空です".to_string())
    }
}

// ---------------------------------------------------------------- 世代の圧縮

/// 作ったアーカイブを最後まで読み直し、ファイル数と整合性を確かめる(元のフォルダを消す前の確認)
fn verify_archive(archive_file: &Path, password: &str, expected_files: usize) -> Result<(), String> {
    let lower = archive_file.to_string_lossy().to_lowercase();
    let mut count = 0usize;

    if lower.ends_with(".zip") || lower.ends_with(".zip.partial") {
        let file = File::open(archive_file).map_err(|e| e.to_string())?;
        let mut zip = ZipArchive::new(file).map_err(|e| e.to_string())?;
        for i in 0..zip.len() {
            let mut entry = if password.is_empty() {
                zip.by_index(i)
            } else {
                zip.by_index_decrypt(i, password.as_bytes())
            }
            .map_err(|e| e.to_string())?;
            if entry.is_dir() {
                continue;
            }
            io::copy(&mut entry, &mut io::sink()).map_err(|e| e.to_string())?;
            count += 1;
        }
    } else {
        let file = File::open(archive_file).map_err(|e| e.to_string())?;
        let mut archive = Archive::new(GzDecoder::new(file));
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            io::copy(&mut entry, &mut io::sink()).map_err(|e| e.to_string())?;
            count += 1;
        }
    }

    if count != expected_files {
        return Err(format!(
            "ファイル数が一致しません(元: {} / アーカイブ: {})",
            expected_files, count
        ));
    }
    Ok(())
}

/// 世代フォルダ(baseN_...)を1つのアーカイブにまとめ、検証してから元のフォルダを削除する。
/// 途中で失敗したときは、元のフォルダはそのまま残す。
pub fn execute_generation_archive(
    target_n: u32,
    format: &str,
    work_file: &str,
    backup_dir: &str,
    password: &str,
) -> Result<(), String> {
    let backup_path = if backup_dir.is_empty() {
        default_backup_dir(work_file)
    } else {
        PathBuf::from(backup_dir)
    };

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

    let folder_name = src_path.file_name().unwrap().to_string_lossy().into_owned();
    let ext = if format == "tar" { "tar.gz" } else { "zip" };
    let dst_path = backup_path.join(format!("{}.{}", folder_name, ext));
    // 作成中は別名で書き、検証が済んでから本来の名前にする(作りかけが履歴に出ないように)
    let tmp_path = backup_path.join(format!("{}.{}.partial", folder_name, ext));

    let expected_files = WalkDir::new(&src_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    let built = with_cleanup(&tmp_path, || {
        if format == "tar" {
            compress_dir_tar(&src_path, &tmp_path)?;
        } else {
            compress_dir_zip(&src_path, &tmp_path, password)?;
        }
        verify_archive(&tmp_path, if format == "tar" { "" } else { password }, expected_files)
            .map_err(|e| {
                format!(
                    "アーカイブの検証に失敗したため、元のフォルダは削除しませんでした: {}",
                    e
                )
            })
    });
    built?;

    // 同名のアーカイブが残っていれば置き換える
    fs::rename(&tmp_path, &dst_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("アーカイブの保存に失敗しました: {}", e)
    })?;

    fs::remove_dir_all(&src_path).map_err(|e| format!("フォルダ削除失敗: {}", e))?;
    Ok(())
}

// ---------------------------------------------------------------- キャッシュ

/// キャッシュディレクトリを一掃する
pub fn clear_cache_directory(cache_root: &Path) -> Result<(), String> {
    if cache_root.exists() {
        if fs::remove_dir_all(cache_root).is_err() {
            let timestamp = Local::now().format("%H%M%S");
            let old_cache = cache_root.with_extension(format!("old_{}", timestamp));
            fs::rename(cache_root, &old_cache).map_err(|_| {
                "キャッシュをクリアできませんでした。フォルダを閉じているか確認してください。"
            })?;
            let _ = fs::remove_dir_all(&old_cache);
        }
    }
    Ok(())
}

/// アーカイブ内のパスから、キャッシュ内の展開先を決める。
/// `baseN_...` から先のフォルダ構造を保つ(base 側のフォルダの中身も、そのまま復元に使えるように)。
fn cache_dest(cache_root: &Path, name: &str) -> Option<PathBuf> {
    let rel = safe_relative_path(name)?;
    let comps: Vec<Component> = rel.components().collect();
    let pos = comps
        .iter()
        .position(|c| c.as_os_str().to_string_lossy().starts_with("base"))?;
    let tail: PathBuf = comps[pos..].iter().collect();
    Some(cache_root.join(tail))
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
        extract_zip(archive_file, cache_root, password.as_deref())?;
    } else if f_name_lower.contains(".tar.gz") || f_name_lower.ends_with(".tgz") {
        extract_targz(archive_file, cache_root)?;
    }

    Ok(cache_root.to_string_lossy().to_string())
}

fn extract_zip(
    archive_file: &Path,
    cache_root: &Path,
    password: Option<&str>,
) -> Result<(), String> {
    let file = File::open(archive_file).map_err(|e| e.to_string())?;
    let mut zip = ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..zip.len() {
        let (name, is_dir) = {
            let entry = zip.by_index_raw(i).map_err(|e| format!("展開エラー: {}", e))?;
            (entry.name().to_string(), entry.is_dir())
        };
        let Some(dest) = cache_dest(cache_root, &name) else {
            continue;
        };
        if is_dir {
            fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
        } else {
            extract_zip_entry(&mut zip, i, password, &dest)?;
        }
    }
    Ok(())
}

fn extract_targz(archive_file: &Path, cache_root: &Path) -> Result<(), String> {
    let file = File::open(archive_file).map_err(|e| e.to_string())?;
    let mut archive = Archive::new(GzDecoder::new(file));

    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            continue;
        }
        let name = entry
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(dest) = cache_dest(cache_root, &name) else {
            continue;
        };
        if kind.is_dir() {
            fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut f = File::create(&dest).map_err(|e| e.to_string())?;
            io::copy(&mut entry, &mut f).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
