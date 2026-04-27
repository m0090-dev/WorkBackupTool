use std::fs;
use tempfile::tempdir;
use work_backup_tool::core::backup::scanner;

fn setup_backup_tree(root: &std::path::Path) {
    // base1 世代
    let g1 = root.join("base1_20260101_100000");
    fs::create_dir(&g1).unwrap();
    fs::write(g1.join("work.clip.20260101_100500.hdiff.diff"), b"d1").unwrap();
    fs::write(g1.join("work.clip.20260101_101000.hdiff.diff"), b"d2").unwrap();
    fs::write(g1.join("work.clip.base"), b"base").unwrap(); // .base は除外対象

    // base2 世代
    let g2 = root.join("base2_20260101_110000");
    fs::create_dir(&g2).unwrap();
    fs::write(g2.join("work.clip.20260101_110500.hdiff.diff"), b"d3").unwrap();

    // ルート直下のフルコピー
    fs::write(root.join("work_20260101_090000.clip"), b"full").unwrap();
}

// =====================================================================
// scan_backups
// =====================================================================

#[test]
fn test_scan_backups_basic() {
    let dir = tempdir().unwrap();
    setup_backup_tree(dir.path());

    let work_file = dir.path().join("work.clip").to_string_lossy().to_string();
    let backup_dir = dir.path().to_string_lossy().to_string();

    let items = scanner::scan_backups(&work_file, &backup_dir, false, true);

    // 世代内diff x3
    // .base は除外されているはず
    assert_eq!(items.len(), 3);
}

#[test]
fn test_scan_backups_generation_index_correct() {
    let dir = tempdir().unwrap();
    setup_backup_tree(dir.path());

    let work_file = dir.path().join("work.clip").to_string_lossy().to_string();
    let backup_dir = dir.path().to_string_lossy().to_string();

    let items = scanner::scan_backups(&work_file, &backup_dir, false, true);

    let gen1_items: Vec<_> = items.iter().filter(|i| i.generation == 1).collect();
    let gen2_items: Vec<_> = items.iter().filter(|i| i.generation == 2).collect();
    let gen0_items: Vec<_> = items.iter().filter(|i| i.generation == 0).collect();

    assert_eq!(gen1_items.len(), 2);
    assert_eq!(gen2_items.len(), 1);
    assert_eq!(gen0_items.len(), 0);
}

#[test]
fn test_scan_backups_strict_match() {
    let dir = tempdir().unwrap();
    let g1 = dir.path().join("base1_20260101_100000");
    fs::create_dir(&g1).unwrap();
    fs::write(g1.join("work.clip.20260101_100000.hdiff.diff"), b"").unwrap();
    fs::write(g1.join("other.psd.20260101_100000.hdiff.diff"), b"").unwrap(); // 別ファイルの差分

    let work_file = dir.path().join("work.clip").to_string_lossy().to_string();
    let backup_dir = dir.path().to_string_lossy().to_string();

    // strict=true → work.clip に関係するものだけ
    let strict_items = scanner::scan_backups(&work_file, &backup_dir, true, true);
    assert_eq!(strict_items.len(), 1);

    // strict=false → 全部
    let all_items = scanner::scan_backups(&work_file, &backup_dir, false, true);
    assert_eq!(all_items.len(), 2);
}

#[test]
fn test_scan_backups_nonexistent_dir_returns_empty() {
    let items = scanner::scan_backups("/no/work.clip", "/no/backup/dir", false, true);
    assert!(items.is_empty());
}

#[test]
fn test_scan_backups_empty_backup_dir_uses_default() {
    let dir = tempdir().unwrap();
    // work_file と同じディレクトリに wbt_backup_work フォルダを作る
    let work_file = dir.path().join("work.clip");
    fs::write(&work_file, b"").unwrap();
    let default_backup = dir.path().join("wbt_backup_work");
    let g1 = default_backup.join("base1_20260101_100000");
    fs::create_dir_all(&g1).unwrap();
    fs::write(g1.join("work.clip.20260101_100000.hdiff.diff"), b"d").unwrap();

    // backup_dir を空文字にするとデフォルトパスを使うはず
    let items = scanner::scan_backups(&work_file.to_string_lossy(), "", false, true);
    assert_eq!(items.len(), 1);
}

#[test]
fn test_scan_backups_base_files_excluded() {
    let dir = tempdir().unwrap();
    let g1 = dir.path().join("base1_20260101_100000");
    fs::create_dir(&g1).unwrap();
    fs::write(g1.join("work.clip.base"), b"base content").unwrap();

    let work_file = dir.path().join("work.clip").to_string_lossy().to_string();
    let backup_dir = dir.path().to_string_lossy().to_string();

    let items = scanner::scan_backups(&work_file, &backup_dir, false, true);
    assert!(items.is_empty(), ".base ファイルは除外されるべき");
}

#[test]
fn test_scan_backups_item_fields() {
    let dir = tempdir().unwrap();
    let g1 = dir.path().join("base1_20260101_100000");
    fs::create_dir(&g1).unwrap();
    let diff = g1.join("work.clip.20260101_100000.hdiff.diff");
    fs::write(&diff, b"content").unwrap();

    let work_file = dir.path().join("work.clip").to_string_lossy().to_string();
    let backup_dir = dir.path().to_string_lossy().to_string();

    let items = scanner::scan_backups(&work_file, &backup_dir, false, true);
    assert_eq!(items.len(), 1);

    let item = &items[0];
    assert_eq!(item.file_name, "work.clip.20260101_100000.hdiff.diff");
    assert_eq!(item.generation, 1);
    assert_eq!(item.file_size, 7); // "content" = 7 bytes
    assert!(!item.is_archived);
    assert!(!item.timestamp.is_empty());
}

// =====================================================================
// scan_generation_folders
// =====================================================================

#[test]
fn test_scan_generation_folders_basic() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"data").unwrap();

    // 世代フォルダを複数作成（最新はbase3）
    fs::create_dir(dir.path().join("base1_20260101_100000")).unwrap();
    fs::create_dir(dir.path().join("base2_20260101_110000")).unwrap();
    fs::create_dir(dir.path().join("base3_20260101_120000")).unwrap();

    let backup_dir = dir.path().to_string_lossy().to_string();
    let items = scanner::scan_generation_folders(&work.to_string_lossy(), &backup_dir).unwrap();

    // 最新のbase3はスキップ → base1, base2 の2つ
    assert_eq!(items.len(), 2);
}

#[test]
fn test_scan_generation_folders_sorted_by_generation() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"data").unwrap();

    fs::create_dir(dir.path().join("base3_20260101_120000")).unwrap();
    fs::create_dir(dir.path().join("base1_20260101_100000")).unwrap();
    fs::create_dir(dir.path().join("base2_20260101_110000")).unwrap();
    // 最新はbase3なのでスキップ、base1とbase2が返る
    // さらにbase4を追加することでbase3もスキップ対象から外れる
    fs::create_dir(dir.path().join("base4_20260101_130000")).unwrap();

    let backup_dir = dir.path().to_string_lossy().to_string();
    let items = scanner::scan_generation_folders(&work.to_string_lossy(), &backup_dir).unwrap();

    // base1, base2, base3 の3つ、昇順にソートされているか
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].generation, 1);
    assert_eq!(items[1].generation, 2);
    assert_eq!(items[2].generation, 3);
}

#[test]
fn test_scan_generation_folders_nonexistent_returns_empty() {
    let result = scanner::scan_generation_folders("/no/work.clip", "/no/backup").unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_scan_generation_folders_only_one_gen_returns_empty() {
    // 世代が1つしかない場合、それが最新なのでスキップ → 空
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"data").unwrap();
    fs::create_dir(dir.path().join("base1_20260101_100000")).unwrap();

    let backup_dir = dir.path().to_string_lossy().to_string();
    let items = scanner::scan_generation_folders(&work.to_string_lossy(), &backup_dir).unwrap();
    assert!(items.is_empty());
}

#[test]
fn test_scan_generation_folders_ignores_non_base_dirs() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"data").unwrap();
    fs::create_dir(dir.path().join("base1_20260101_100000")).unwrap();
    fs::create_dir(dir.path().join("base2_20260101_110000")).unwrap();
    fs::create_dir(dir.path().join("not_a_gen_folder")).unwrap();
    fs::create_dir(dir.path().join(".wbt_cache")).unwrap();

    let backup_dir = dir.path().to_string_lossy().to_string();
    let items = scanner::scan_generation_folders(&work.to_string_lossy(), &backup_dir).unwrap();

    // base1のみ（base2が最新でスキップ）、not_a_gen_folderや.wbt_cacheは無視
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].generation, 1);
}
