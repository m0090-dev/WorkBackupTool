use crate::core::backup::auto_generation;
use crate::core::types::BackupItem;
use crate::core::utils;
use chrono::{DateTime, Local};
use std::fs;
use std::path::{Path, PathBuf};

/// バックアップディレクトリとキャッシュディレクトリを走査してアイテム一覧を返す
/// フルコピー（.diff / .zip / .tar.gz 以外）は復元操作が成立しないため返さない
pub fn scan_backups(
    work_path: &str,
    backup_dir: &str,
    strict_match: bool,
    use_same_dir_for_temp: bool,
) -> Vec<BackupItem> {
    let mut list = Vec::new();

    // 1. ルートディレクトリとキャッシュルートの決定
    let root = if backup_dir.is_empty() {
        utils::default_backup_dir(work_path)
    } else {
        PathBuf::from(backup_dir)
    };
    if !root.exists() {
        return list;
    }

    let cache_root = utils::get_cache_root(use_same_dir_for_temp, backup_dir, work_path);

    // 2. 判定用データの準備
    let file_path_obj = Path::new(work_path);
    // フォルダの場合も file_name() でフォルダ名を取得できる
    let base_name_only = file_path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(
            file_path_obj
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(""),
        );
    let base_lower = base_name_only.to_lowercase();

    // 復元可能な拡張子のみ（フルコピーは除外）
    let restorable_exts: &[&str] = &[".diff", ".zip", ".tar.gz"];

    // 拡張子判定ヘルパー（復元可能なもののみ）
    let is_restorable = |name: &str| -> bool {
        let n = name.to_lowercase();
        restorable_exts.iter().any(|ext| n.ends_with(ext))
    };

    // 走査対象：(スキャンするディレクトリ, アーカイブ展開フラグ)
    let scan_roots = vec![(&root, false), (&cache_root, true)];

    for (current_root, is_archived_flag) in scan_roots {
        if !current_root.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(current_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                if path.is_dir() {
                    // --- 世代フォルダ (baseN_...) の中身をスキャン ---
                    if file_name.starts_with("base") {
                        let gen_idx = file_name
                            .strip_prefix("base")
                            .and_then(|s| s.split('_').next())
                            .and_then(|s| s.parse::<i32>().ok())
                            .unwrap_or(0);

                        if let Ok(gen_entries) = fs::read_dir(&path) {
                            for gen_entry in gen_entries.flatten() {
                                let gen_path = gen_entry.path();
                                let f_name =
                                    gen_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                                // .base ファイル / .base フォルダは管理用なので除外
                                if f_name.ends_with(".base") {
                                    continue;
                                }
                                // フォルダ自体は作業フォルダの .base コピー（world/ 形式）なので除外
                                if gen_path.is_dir() {
                                    continue;
                                }

                                // 復元可能な拡張子のみ対象
                                if (!strict_match || f_name.to_lowercase().contains(&base_lower))
                                    && is_restorable(f_name)
                                {
                                    if let Ok(metadata) = fs::metadata(&gen_path) {
                                        list.push(create_backup_item(
                                            f_name,
                                            &gen_path,
                                            &metadata,
                                            gen_idx,
                                            is_archived_flag,
                                            false, // diff/archive はフォルダではない
                                        ));
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // --- ルート直下のファイルをスキャン ---
                    // フルコピーを含まない復元可能なものだけ
                    if (!strict_match || file_name.to_lowercase().contains(&base_lower))
                        && is_restorable(file_name)
                    {
                        if let Ok(metadata) = fs::metadata(&path) {
                            list.push(create_backup_item(
                                file_name, &path, &metadata, 0, false, false,
                            ));
                        }
                    }
                }
            }
        }
    }

    list
}

/// アーカイブ可能な世代フォルダ一覧を取得する
pub fn scan_generation_folders(
    work_path: &str,
    backup_dir: &str,
) -> Result<Vec<BackupItem>, String> {
    let root = if backup_dir.is_empty() {
        utils::default_backup_dir(work_path)
    } else {
        PathBuf::from(backup_dir)
    };

    if !root.exists() {
        return Ok(Vec::new());
    }

    // 最新の世代フォルダを特定（除外用）
    let latest_path = auto_generation::get_latest_generation(&root)?.map(|i| i.dir_path);

    let mut list = Vec::new();
    let entries = fs::read_dir(&root).map_err(|e| e.to_string())?;
    let re = regex::Regex::new(r"^base(\d+)_").unwrap();

    for entry in entries.flatten() {
        if entry.file_type().map_or(false, |t| t.is_dir()) {
            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();

            if let Some(caps) = re.captures(&name) {
                // 現在進行中の最新世代はスキップ
                if let Some(ref lp) = latest_path {
                    if &path == lp {
                        continue;
                    }
                }

                let gen_idx = caps[1].parse::<i32>().unwrap_or(0);
                let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;

                list.push(create_backup_item(
                    &name, &path, &metadata, gen_idx, false, true,
                ));
            }
        }
    }

    list.sort_by(|a, b| a.generation.cmp(&b.generation));
    Ok(list)
}

// ヘルパー関数: アイテム生成 (日付フォーマット含む)
fn create_backup_item(
    name: &str,
    path: &Path,
    meta: &fs::Metadata,
    gen: i32,
    is_archived: bool,
    is_folder: bool,
) -> BackupItem {
    let modified: DateTime<Local> = meta
        .modified()
        .unwrap_or_else(|_| std::time::SystemTime::now())
        .into();
    BackupItem {
        file_name: name.to_string(),
        file_path: path.to_string_lossy().into_owned(),
        timestamp: modified.format("%Y-%m-%d %H:%M:%S").to_string(),
        file_size: meta.len() as i64,
        generation: gen,
        is_archived,
        is_folder,
    }
}
