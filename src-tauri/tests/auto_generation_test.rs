use std::fs;
use tempfile::tempdir;
use work_backup_tool::core::backup::auto_generation;

// =====================================================================
// get_latest_generation
// =====================================================================

#[test]
fn test_get_latest_generation_empty_dir() {
    let dir = tempdir().unwrap();
    let result = auto_generation::get_latest_generation(dir.path()).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_get_latest_generation_nonexistent_root() {
    let result =
        auto_generation::get_latest_generation(std::path::Path::new("/no/such/dir")).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_get_latest_generation_single() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("base1_20260101_100000")).unwrap();

    let info = auto_generation::get_latest_generation(dir.path())
        .unwrap()
        .unwrap();
    assert_eq!(info.base_idx, 1);
    assert_eq!(
        info.dir_path.file_name().unwrap().to_string_lossy(),
        "base1_20260101_100000"
    );
}

#[test]
fn test_get_latest_generation_picks_highest_index() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("base1_20260101_100000")).unwrap();
    fs::create_dir(dir.path().join("base2_20260101_110000")).unwrap();
    fs::create_dir(dir.path().join("base10_20260101_120000")).unwrap();
    // 関係ないフォルダは無視されるか
    fs::create_dir(dir.path().join("not_a_backup")).unwrap();
    fs::write(dir.path().join("somefile.diff"), b"").unwrap();

    let info = auto_generation::get_latest_generation(dir.path())
        .unwrap()
        .unwrap();
    assert_eq!(info.base_idx, 10);
    assert_eq!(
        info.dir_path.file_name().unwrap().to_string_lossy(),
        "base10_20260101_120000"
    );
}

#[test]
fn test_get_latest_generation_same_index_picks_lexicographically_later() {
    // 同じindexが複数ある場合は文字列的に後ろ（タイムスタンプが新しい）を選ぶ
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("base3_20260101_090000")).unwrap();
    fs::create_dir(dir.path().join("base3_20260101_100000")).unwrap();

    let info = auto_generation::get_latest_generation(dir.path())
        .unwrap()
        .unwrap();
    assert_eq!(info.base_idx, 3);
    assert_eq!(
        info.dir_path.file_name().unwrap().to_string_lossy(),
        "base3_20260101_100000"
    );
}

#[test]
fn test_get_latest_generation_ignores_no_underscore() {
    // "base1" (アンダースコアなし) は対象外
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("base1")).unwrap();
    fs::create_dir(dir.path().join("base2_20260101_100000")).unwrap();

    let info = auto_generation::get_latest_generation(dir.path())
        .unwrap()
        .unwrap();
    assert_eq!(info.base_idx, 2);
}

// =====================================================================
// create_new_generation
// =====================================================================

#[test]
fn test_create_new_generation_creates_folder_and_base() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("art.clip");
    fs::write(&work, b"canvas data").unwrap();

    let new_path =
        auto_generation::create_new_generation(dir.path(), 1, &work.to_string_lossy()).unwrap();

    // フォルダ名がbase1_で始まるか
    assert!(new_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("base1_"));
    // .baseファイルが作られているか
    let base_file = new_path.join("art.clip.base");
    assert!(base_file.exists());
    assert_eq!(fs::read(&base_file).unwrap(), b"canvas data");
}

#[test]
fn test_create_new_generation_arbitrary_index() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("proj.psd");
    fs::write(&work, b"psd data").unwrap();

    let path =
        auto_generation::create_new_generation(dir.path(), 99, &work.to_string_lossy()).unwrap();
    assert!(path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("base99_"));
}

#[test]
fn test_create_new_generation_nonexistent_work_file() {
    let dir = tempdir().unwrap();
    let result = auto_generation::create_new_generation(dir.path(), 1, "/nonexistent/file.clip");
    assert!(result.is_err());
}

// =====================================================================
// resolve_generation_dir
// =====================================================================

#[test]
fn test_resolve_generation_dir_creates_when_empty() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"data").unwrap();

    let (path, idx) =
        auto_generation::resolve_generation_dir(dir.path(), &work.to_string_lossy()).unwrap();
    assert_eq!(idx, 1);
    assert!(path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("base1_"));
    assert!(path.exists());
}

#[test]
fn test_resolve_generation_dir_returns_existing() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"data").unwrap();

    // 既存の世代フォルダを作っておく
    let existing = dir.path().join("base3_20260101_100000");
    fs::create_dir(&existing).unwrap();

    let (path, idx) =
        auto_generation::resolve_generation_dir(dir.path(), &work.to_string_lossy()).unwrap();
    assert_eq!(idx, 3);
    assert_eq!(path, existing);
}

#[test]
fn test_resolve_generation_dir_idempotent() {
    // 2回呼んでも新しいフォルダが増えないか
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"data").unwrap();

    let (p1, i1) =
        auto_generation::resolve_generation_dir(dir.path(), &work.to_string_lossy()).unwrap();
    let (p2, i2) =
        auto_generation::resolve_generation_dir(dir.path(), &work.to_string_lossy()).unwrap();

    assert_eq!(p1, p2);
    assert_eq!(i1, i2);

    // ルート直下のフォルダ数が1つだけか
    let count = fs::read_dir(dir.path()).unwrap().count();
    // work.clipと世代フォルダ1つ = 2
    assert_eq!(count, 2);
}

// =====================================================================
// should_rotate
// =====================================================================

#[test]
fn test_should_rotate_exceeds_threshold() {
    let dir = tempdir().unwrap();
    let base = dir.path().join("base.clip");
    let diff = dir.path().join("out.diff");

    // base: 1000 bytes, diff: 900 bytes, threshold: 0.8 → 900 > 800 → true
    fs::write(&base, vec![0u8; 1000]).unwrap();
    fs::write(&diff, vec![0u8; 900]).unwrap();

    assert!(auto_generation::should_rotate(&base, &diff, 0.8));
}

#[test]
fn test_should_rotate_under_threshold() {
    let dir = tempdir().unwrap();
    let base = dir.path().join("base.clip");
    let diff = dir.path().join("out.diff");

    // base: 1000, diff: 500, threshold: 0.8 → 500 < 800 → false
    fs::write(&base, vec![0u8; 1000]).unwrap();
    fs::write(&diff, vec![0u8; 500]).unwrap();

    assert!(!auto_generation::should_rotate(&base, &diff, 0.8));
}

#[test]
fn test_should_rotate_zero_base_size() {
    let dir = tempdir().unwrap();
    let base = dir.path().join("base.clip");
    let diff = dir.path().join("out.diff");

    fs::write(&base, b"").unwrap();
    fs::write(&diff, vec![0u8; 100]).unwrap();

    // base_size == 0 → false（ゼロ除算防止）
    assert!(!auto_generation::should_rotate(&base, &diff, 0.8));
}

#[test]
fn test_should_rotate_nonexistent_files() {
    // ファイルがない場合はサイズ0扱いでfalse
    assert!(!auto_generation::should_rotate(
        std::path::Path::new("/no/base"),
        std::path::Path::new("/no/diff"),
        0.5
    ));
}
