use crate::core::types::DiffFileInfo;
use crate::core::utils;
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};
use fs_extra::dir;
/// hdiffz 用の引数リストを生成するロジック
pub fn build_hdiffz_args<'a>(
    old_file: &'a str,
    new_file: &'a str,
    diff_file: &'a str,
    compress_algo: &'a str,
) -> Vec<&'a str> {
    let mut args = vec!["-f", "-s"];

    match compress_algo {
        "zstd" => args.push("-c-zstd"),
        "lzma2" => args.push("-c-lzma2"),
        "lzma" => args.push("-c-lzma"),
        "zlib" => args.push("-c-zlib"),
        "ldef" => args.push("-c-ldef"),
        "pbzip2" => args.push("-c-pbzip2"),
        "bzip2" => args.push("-c-bzip2"),
        "none" => {}               // uncompress
        _ => args.push("-c-zstd"), // default
    };

    args.push(old_file);
    args.push(new_file);
    args.push(diff_file);
    args
}

/// hpatchz 用の引数リストを生成するロジック
pub fn build_hpatchz_args<'a>(
    base_full: &'a str,
    diff_file: &'a str,
    out_path: &'a str,
) -> Vec<&'a str> {
    vec!["-f", "-s", base_full, diff_file, out_path]
}

// 1. GetHdiffList の移植
pub fn get_hdiff_list(
    work_file: &str,
    custom_dir: Option<String>,
) -> Result<Vec<DiffFileInfo>, String> {
    // custom_dir がなければデフォルトパスを取得
    let target_dir = match custom_dir {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => utils::default_backup_dir(work_file),
    };

    if !target_dir.exists() {
        return Ok(vec![]);
    }

    let mut list = Vec::new();
    for entry in fs::read_dir(target_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        let file_name = entry.file_name().to_string_lossy().into_owned();

        // ディレクトリではなく、拡張子が .diff のものを抽出
        if path.is_file() && file_name.ends_with(".diff") {
            let ts = utils::extract_timestamp_from_backup(&file_name).unwrap_or_default();
            list.push(DiffFileInfo {
                file_name,
                file_path: path.to_string_lossy().into_owned(),
                timestamp: ts,
                file_size: metadata.len() as i64,
            });
        }
    }
    Ok(list)
}

/// BackupOrHdiff のロジック部分
/// 戻り値: Ok(Some((base, work, diff))) => Sidecar実行が必要
///        Ok(None) => .baseコピーのみで終了
pub fn prepare_hdiff_paths(
    work_file: &str,
    target_dir: PathBuf,
) -> Result<Option<(String, String, String)>, String> {
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
        let path = Path::new(work_file);
    let base_name = Path::new(work_file)
        .file_name()
        .ok_or("Invalid work file name")?
        .to_string_lossy();
    let base_full = target_dir.join(format!("{}.base", base_name));

    
    if !base_full.exists() {
        let metadata = fs::metadata(path).map_err(|e| e.to_string())?;

        if metadata.is_dir() {
            let mut options = dir::CopyOptions::new();
            options.copy_inside = true;
            options.content_only = false;

            dir::copy(path, &base_full, &options)
                .map_err(|e| e.to_string())?;
        } else {
            fs::copy(path, &base_full).map_err(|e| e.to_string())?;
        }

        return Ok(None);
    }

    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let diff_path = target_dir.join(format!("{}.{}.diff", base_name, ts));

    Ok(Some((
        base_full.to_string_lossy().into_owned(),
        work_file.to_string(),
        diff_path.to_string_lossy().into_owned(),
    )))
}

/// ApplyHdiffWrapper のロジック部分
/// 戻り値: (base_path, out_path)
pub fn resolve_apply_paths(
    work_file: &str,
    diff_file: &str,
    temp_out_path: String, // app::utils::auto_output_path の結果をもらう
) -> Result<(String, String), String> {
    let diff_path = Path::new(diff_file);
    let backup_dir = diff_path.parent().ok_or("Invalid diff path")?;
    let diff_name = diff_path
        .file_name()
        .ok_or("Invalid diff name")?
        .to_string_lossy();

    let original_full_name = diff_name.split(".20").next().unwrap_or(&diff_name);
  
    let base_name = format!("{}.base", diff_name.split(".20").next().unwrap());
    let mut base_full = backup_dir.join(&base_name);

    if !base_full.exists() {
        let work_base_name = format!(
            "{}.base",
            Path::new(work_file)
                .file_name()
                .ok_or("Invalid work file")?
                .to_string_lossy()
        );
        base_full = backup_dir.join(work_base_name);
    }

let work_path = Path::new(work_file);
    let out_path = if let Some(ext) = work_path.extension() {
    // 【ファイルの場合】
    // 元の拡張子（.clipなど）を維持して出力
    Path::new(&temp_out_path)
        .with_extension(ext)
        .to_string_lossy()
        .into_owned()
} else {
    // 【フォルダの場合】
    // 拡張子がないので、テンポラリパスをそのまま（拡張子なしで）使う
    temp_out_path
};

    Ok((base_full.to_string_lossy().into_owned(), out_path))
}
