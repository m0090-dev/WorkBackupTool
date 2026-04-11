use app_lib::app::auto_generation;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_get_latest_generation_logic() {
    let dir = tempdir().expect("Failed to create temp dir");
    let root = dir.path();

    // 1. 空の状態でのテスト
    let latest = auto_generation::get_latest_generation(root).unwrap();
    assert!(latest.is_none());

    // 2. 複数の世代フォルダを作成
    fs::create_dir(root.join("base1_20260101_100000")).unwrap();
    fs::create_dir(root.join("base2_20260101_110000")).unwrap();
    fs::create_dir(root.join("base10_20260101_120000")).unwrap();
    fs::create_dir(root.join("not_a_backup_dir")).unwrap();

    let latest_info = auto_generation::get_latest_generation(root)
        .unwrap()
        .unwrap();

    // フィールド名 base_idx を使用
    assert_eq!(latest_info.base_idx, 10);
    // dir_path からディレクトリ名を取得して比較
    let actual_dir_name = latest_info.dir_path.file_name().unwrap().to_string_lossy();
    assert_eq!(actual_dir_name, "base10_20260101_120000");
}

#[test]
fn test_create_new_generation() {
    let dir = tempdir().expect("Failed to create temp dir");
    let root = dir.path();

    let work_file_path = dir.path().join("work.clip");
    fs::write(&work_file_path, b"sample content").unwrap();
    let work_file_str = work_file_path.to_string_lossy();

    // 新しい世代 (index: 5) を作成
    let new_path = auto_generation::create_new_generation(root, 5, &work_file_str).unwrap();

    let dir_name = new_path.file_name().unwrap().to_string_lossy();
    assert!(dir_name.starts_with("base5_"));

    let base_file = new_path.join("work.clip.base");
    assert!(base_file.exists());

    let content = fs::read_to_string(base_file).unwrap();
    assert_eq!(content, "sample content");
}

#[test]
fn test_resolve_generation_logic() {
    let dir = tempdir().expect("Failed to create temp dir");
    let root = dir.path();

    let work_file_path = dir.path().join("project.clip");
    fs::write(&work_file_path, b"data").unwrap();
    let work_file_str = work_file_path.to_string_lossy();

    // 1. 最初の世代作成 (resolve_generation_dir は存在しない場合 index 1 で作成する)
    let (path1, idx1) = auto_generation::resolve_generation_dir(root, &work_file_str).unwrap();
    assert_eq!(idx1, 1);
    assert!(path1
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("base1_"));

    // 2. 既存がある場合の取得 (既に 1 があるので、resolve は新しいのを作らず既存の 1 を返す)
    let (path2, idx2) = auto_generation::resolve_generation_dir(root, &work_file_str).unwrap();
    assert_eq!(idx2, 1);
    assert_eq!(path1, path2);

    // 3. 強制的に次の番号で新しい世代を作る場合は create_new_generation を使用
    let path3 = auto_generation::create_new_generation(root, idx2 + 1, &work_file_str).unwrap();
    assert!(path3
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("base2_"));
}
