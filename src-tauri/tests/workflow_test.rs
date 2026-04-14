use std::fs;
use tempfile::tempdir;
use work_backup_tool::core::backup::workflow;

// =====================================================================
// should_transition_to_next_gen
// =====================================================================

#[test]
fn test_should_transition_exceeds_threshold() {
    // work: 200KB, diff: 170KB, threshold: 0.8 → 170K > 160K → true
    assert!(workflow::should_transition_to_next_gen(
        200 * 1024,
        170 * 1024,
        0.8
    ));
}

#[test]
fn test_should_transition_under_threshold() {
    // work: 200KB, diff: 100KB, threshold: 0.8 → 100K < 160K → false
    assert!(!workflow::should_transition_to_next_gen(
        200 * 1024,
        100 * 1024,
        0.8
    ));
}

#[test]
fn test_should_transition_work_too_small() {
    // work < 100KB → 常にfalse（スモールファイルは世代交代しない）
    assert!(!workflow::should_transition_to_next_gen(
        50 * 1024,
        49 * 1024,
        0.8
    ));
}

#[test]
fn test_should_transition_exactly_at_boundary() {
    // diff == work * threshold → 超えていないのでfalse
    let work = 200 * 1024u64;
    let diff = (work as f64 * 0.8) as u64;
    assert!(!workflow::should_transition_to_next_gen(work, diff, 0.8));
}

#[test]
fn test_should_transition_threshold_one() {
    // threshold=1.0 → diffがworkと同じサイズでもfalse
    let size = 200 * 1024u64;
    assert!(!workflow::should_transition_to_next_gen(size, size, 1.0));
}

// =====================================================================
// resolve_backup_target
// =====================================================================

#[test]
fn test_resolve_backup_target_auto_route() {
    // フォルダ名が "base" で始まらない → 自動ルート
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"data").unwrap();

    // backup ルートとして dir.path() を渡す
    let info =
        workflow::resolve_backup_target(dir.path().to_path_buf(), &work.to_string_lossy()).unwrap();

    // 自動作成なのでbase1_...フォルダができているはず
    assert_eq!(info.current_idx, 1);
    assert!(info.project_root == dir.path());
    assert!(info
        .target_dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("base1_"));
}

#[test]
fn test_resolve_backup_target_manual_route() {
    // フォルダ名が "base3_..." → マニュアルルート
    let dir = tempdir().unwrap();
    let gen_dir = dir.path().join("base3_20260101_120000");
    fs::create_dir(&gen_dir).unwrap();

    let work = dir.path().join("work.clip");
    fs::write(&work, b"data").unwrap();

    let info = workflow::resolve_backup_target(gen_dir.clone(), &work.to_string_lossy()).unwrap();

    assert_eq!(info.current_idx, 3);
    assert_eq!(info.target_dir, gen_dir);
    assert_eq!(info.project_root, dir.path());
}

#[test]
fn test_resolve_backup_target_manual_index_extraction() {
    let dir = tempdir().unwrap();
    let gen_dir = dir.path().join("base42_20260101_120000");
    fs::create_dir(&gen_dir).unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"").unwrap();

    let info = workflow::resolve_backup_target(gen_dir, &work.to_string_lossy()).unwrap();

    assert_eq!(info.current_idx, 42);
}

// =====================================================================
// detect_diff_algo
// =====================================================================

#[test]
fn test_detect_diff_algo_hdiff() {
    let algo = workflow::detect_diff_algo("work.clip.20260101_100000.hdiff.diff");
    assert!(matches!(algo, workflow::DiffAlgo::HDiff));
}

#[test]
fn test_detect_diff_algo_bsdiff() {
    let algo = workflow::detect_diff_algo("work.clip.20260101_100000.bsdiff.diff");
    assert!(matches!(algo, workflow::DiffAlgo::BsDiff));
}

#[test]
fn test_detect_diff_algo_unknown() {
    // 古い形式（アルゴ名なし）
    let algo = workflow::detect_diff_algo("work.clip.20260101_100000.diff");
    assert!(matches!(algo, workflow::DiffAlgo::Unknown));
}

#[test]
fn test_detect_diff_algo_path_with_dirs() {
    // フルパスでも正しく判定できるか
    let algo = workflow::detect_diff_algo("/backup/base1_xxx/work.clip.20260101.hdiff.diff");
    assert!(matches!(algo, workflow::DiffAlgo::HDiff));
}

// =====================================================================
// execute_copy_backup
// =====================================================================

#[test]
fn test_execute_copy_backup_with_dir() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("work.clip");
    fs::write(&src, b"original").unwrap();

    let backup_dir = dir.path().join("my_backups");
    fs::create_dir(&backup_dir).unwrap();

    let dest =
        workflow::execute_copy_backup(&src.to_string_lossy(), Some(backup_dir.clone())).unwrap();

    assert!(std::path::Path::new(&dest).exists());
    assert_eq!(fs::read(&dest).unwrap(), b"original");
    // コピー先がbackup_dir配下か
    assert!(dest.starts_with(backup_dir.to_string_lossy().as_ref()));
}

#[test]
fn test_execute_copy_backup_without_dir_uses_default() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("art.clip");
    fs::write(&src, b"art data").unwrap();

    let dest = workflow::execute_copy_backup(&src.to_string_lossy(), None).unwrap();

    assert!(std::path::Path::new(&dest).exists());
    // wbt_backup_art 配下に作られているか
    assert!(dest.contains("wbt_backup_art"));
}

#[test]
fn test_execute_copy_backup_creates_dir_if_missing() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("work.clip");
    fs::write(&src, b"data").unwrap();

    // 存在しないディレクトリを指定
    let new_dir = dir.path().join("new_backup_dir");
    assert!(!new_dir.exists());

    let dest =
        workflow::execute_copy_backup(&src.to_string_lossy(), Some(new_dir.clone())).unwrap();

    assert!(new_dir.exists());
    assert!(std::path::Path::new(&dest).exists());
}

#[test]
fn test_execute_copy_backup_nonexistent_src() {
    let dir = tempdir().unwrap();
    let result =
        workflow::execute_copy_backup("/no/such/file.clip", Some(dir.path().to_path_buf()));
    assert!(result.is_err());
}

#[test]
fn test_execute_copy_backup_timestamped_filename() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("work.clip");
    fs::write(&src, b"data").unwrap();

    let dest =
        workflow::execute_copy_backup(&src.to_string_lossy(), Some(dir.path().to_path_buf()))
            .unwrap();

    let filename = std::path::Path::new(&dest)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // タイムスタンプ付きファイル名になっているか (work_YYYYMMDD_HHMMSS.clip)
    assert!(filename.starts_with("work_"));
    assert!(filename.ends_with(".clip"));
}
