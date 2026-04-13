use std::fs;
use tempfile::tempdir;

#[test]
fn test_hdiff_args_with_temp_files() {
    // 1. 一時ディレクトリの作成 (スコープを抜けると自動削除される)
    let dir = tempdir().expect("Failed to create temp dir");

    let old_path = dir.path().join("old.bin");
    let new_path = dir.path().join("new.bin");
    let diff_path = dir.path().join("out.diff");

    // 2. 実際にダミーデータを書き込む
    fs::write(&old_path, b"original data v1").unwrap();
    fs::write(&new_path, b"modified data v2").unwrap();

    let old_p_str = old_path.to_string_lossy();
    let new_p_str = new_path.to_string_lossy();
    let diff_p_str = diff_path.to_string_lossy();

    // 3. 引数生成ロジックのテスト (zstd)
    let args = work_backup_tool::core::ext::hdiff_common::build_hdiffz_args(
        &old_p_str,
        &new_p_str,
        &diff_p_str,
        "zstd",
    );

    assert_eq!(args[0], "-f");
    assert_eq!(args[1], "-s");
    assert_eq!(args[2], "-c-zstd");
    assert_eq!(args[3], old_p_str);
    assert_eq!(args[4], new_p_str);
    assert_eq!(args[5], diff_p_str);

    // 4. 引数生成ロジックのテスト (none)
    let args_none = work_backup_tool::core::ext::hdiff_common::build_hdiffz_args(
        &old_p_str,
        &new_p_str,
        &diff_p_str,
        "none",
    );
    assert_eq!(args_none.len(), 5); // "-c-..." が無いので 5 つ
    assert_eq!(args_none[2], old_p_str);

    // 5. パッチ適用引数のテスト
    let restore_path = dir.path().join("restore.bin");
    let restore_p_str = restore_path.to_string_lossy();

    let patch_args = work_backup_tool::core::ext::hdiff_common::build_hpatchz_args(
        &old_p_str,
        &diff_p_str,
        &restore_p_str,
    );
    assert_eq!(
        patch_args,
        vec!["-f", "-s", &*old_p_str, &*diff_p_str, &*restore_p_str]
    );
}
