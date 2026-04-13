use work_backup_tool::core::utils;

#[test]
fn test_extract_timestamp_from_backup() {
    let case1 = "my_work.clip.20260411_200000.diff";
    assert_eq!(
        utils::extract_timestamp_from_backup(case1).unwrap(),
        "20260411_200000"
    );

    let case2 = "ver.1.2.project.20260411_200000.diff";
    assert_eq!(
        utils::extract_timestamp_from_backup(case2).unwrap(),
        "20260411_200000"
    );

    let case3 = "old_backup.diff";
    assert_eq!(
        utils::extract_timestamp_from_backup(case3).unwrap(),
        "No Timestamp"
    );
}

#[test]
fn test_get_cache_root_logic() {
    let work_file = r"C:\path\to\work.clip";
    let custom_backup_dir = r"D:\backups";

    // 1. backup_dir指定なし + use_same_dir = true
    let res = utils::get_cache_root(true, "", work_file);
    let res_str = res.to_string_lossy();
    assert!(res_str.contains(".wbt_cache"));

    // 2. backup_dir指定あり + use_same_dir = true
    // 引数を true にすることで、custom_backup_dir が優先的に使われるロジックを通します
    let res_custom = utils::get_cache_root(true, custom_backup_dir, work_file);

    // Windowsのパス区切り（\）とRust内部の扱いの差異を避けるため、PathBufで比較
    let expected = std::path::PathBuf::from(custom_backup_dir).join(".wbt_cache");
    assert_eq!(res_custom, expected);
}
