use std::fs;
use tempfile::tempdir;
use work_backup_tool::core::utils;

// =====================================================================
// extract_timestamp_from_backup
// =====================================================================

#[test]
fn test_extract_timestamp_standard() {
    assert_eq!(
        utils::extract_timestamp_from_backup("work.clip.20260411_200000.diff").unwrap(),
        "20260411_200000"
    );
}

#[test]
fn test_extract_timestamp_multi_dot_name() {
    // ドットが多いファイル名でも末尾から2番目を取れるか
    assert_eq!(
        utils::extract_timestamp_from_backup("ver.1.2.project.20260411_200000.diff").unwrap(),
        "20260411_200000"
    );
}

#[test]
fn test_extract_timestamp_no_timestamp() {
    // タイムスタンプがない（パーツが2つ以下）
    assert_eq!(
        utils::extract_timestamp_from_backup("old_backup.diff").unwrap(),
        "No Timestamp"
    );
}

#[test]
fn test_extract_timestamp_just_filename() {
    // 拡張子なし
    assert_eq!(
        utils::extract_timestamp_from_backup("nodot").unwrap(),
        "No Timestamp"
    );
}

// =====================================================================
// timestamped_name
// =====================================================================

#[test]
fn test_timestamped_name_with_extension() {
    let result = utils::timestamped_name("work.clip");
    // 形式: work_YYYYMMDD_HHMMSS.clip
    assert!(result.starts_with("work_"));
    assert!(result.ends_with(".clip"));
    // タイムスタンプ部分が含まれているか（数字8桁_6桁）
    assert!(result.len() > "work_.clip".len());
}

#[test]
fn test_timestamped_name_without_extension() {
    let result = utils::timestamped_name("myfile");
    assert!(result.starts_with("myfile_"));
    assert!(!result.contains('.'));
}

#[test]
fn test_timestamped_name_multiple_dots() {
    // test.project.psd -> stem=test.project, ext=psd
    let result = utils::timestamped_name("test.project.psd");
    assert!(result.ends_with(".psd"));
}

// =====================================================================
// auto_output_path
// =====================================================================

#[test]
fn test_auto_output_path_with_extension() {
    let result = utils::auto_output_path("/some/dir/work.clip");

    // 文字列ではなくPathとして扱う
    let path = std::path::Path::new(&result);

    // ファイル名のチェック
    let file_name = path.file_name().unwrap().to_string_lossy();
    assert!(file_name.contains("work_restored_"));
    assert!(file_name.ends_with(".clip"));

    // 親ディレクトリに "some" と "dir" が含まれているかチェック
    let parent = path.parent().unwrap().to_string_lossy();
    assert!(parent.contains("some"));
    assert!(parent.contains("dir"));
}

#[test]
fn test_auto_output_path_without_extension() {
    let result = utils::auto_output_path("/dir/noext");
    assert!(result.contains("noext_restored_"));
    assert!(!result.ends_with('.'));
}

// =====================================================================
// default_backup_dir
// =====================================================================

#[test]
fn test_default_backup_dir() {
    let result = utils::default_backup_dir("/home/user/art/work.clip");
    let name = result.file_name().unwrap().to_string_lossy();
    assert_eq!(name, "wbt_backup_work");
    // 親がwork.clipと同じディレクトリか
    let parent = result.parent().unwrap().to_string_lossy();
    assert!(parent.contains("/home/user/art"));
}

#[test]
fn test_default_backup_dir_no_extension() {
    let result = utils::default_backup_dir("/dir/myfile");
    let name = result.file_name().unwrap().to_string_lossy();
    assert_eq!(name, "wbt_backup_myfile");
}

// =====================================================================
// copy_file
// =====================================================================

#[test]
fn test_copy_file_basic() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, b"hello world").unwrap();

    utils::copy_file(&src.to_string_lossy(), &dst.to_string_lossy()).unwrap();

    assert!(dst.exists());
    assert_eq!(fs::read_to_string(&dst).unwrap(), "hello world");
}

#[test]
fn test_copy_file_creates_parent_dirs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("nested/deep/dst.txt");
    fs::write(&src, b"data").unwrap();

    utils::copy_file(&src.to_string_lossy(), &dst.to_string_lossy()).unwrap();

    assert!(dst.exists());
}

#[test]
fn test_copy_file_nonexistent_src() {
    let dir = tempdir().unwrap();
    let result = utils::copy_file(
        "/nonexistent/file.txt",
        &dir.path().join("dst.txt").to_string_lossy(),
    );
    assert!(result.is_err());
}

// =====================================================================
// move_file_safe
// =====================================================================

#[test]
fn test_move_file_safe_same_fs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.tmp");
    let dst = dir.path().join("dst.final");
    fs::write(&src, b"move me").unwrap();

    utils::move_file_safe(&src, &dst).unwrap();

    assert!(!src.exists());
    assert!(dst.exists());
    assert_eq!(fs::read(&dst).unwrap(), b"move me");
}

// =====================================================================
// save_to_work_file
// =====================================================================

#[test]
fn test_save_to_work_file() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("output.bin");
    let data: &[u8] = b"save this data";

    utils::save_to_work_file(data, &target.to_string_lossy()).unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"save this data");
}

// =====================================================================
// get_cache_root
// =====================================================================

#[test]
fn test_get_cache_root_same_dir_no_backup_dir() {
    let result = utils::get_cache_root(true, "", "/path/to/work.clip");
    assert!(result.to_string_lossy().contains(".wbt_cache"));
    // default_backup_dir 配下になるはず
    assert!(result.to_string_lossy().contains("wbt_backup_work"));
}

#[test]
fn test_get_cache_root_same_dir_with_backup_dir() {
    let expected = std::path::PathBuf::from("/my/backup").join(".wbt_cache");
    let result = utils::get_cache_root(true, "/my/backup", "/path/to/work.clip");
    assert_eq!(result, expected);
}

#[test]
fn test_get_cache_root_temp_dir() {
    // use_same_dir=false → OS tempディレクトリ下になる
    let result = utils::get_cache_root(false, "", "/path/to/work.clip");
    let s = result.to_string_lossy();
    assert!(s.contains("wbt_cache_work"));
    // ハッシュが付いているので長さが一定以上
    assert!(s.len() > "wbt_cache_work_".len());
}

#[test]
fn test_get_cache_root_temp_dir_deterministic() {
    // 同じ引数なら同じパスが返るか
    let r1 = utils::get_cache_root(false, "", "/same/file.psd");
    let r2 = utils::get_cache_root(false, "", "/same/file.psd");
    assert_eq!(r1, r2);
}

#[test]
fn test_get_cache_root_temp_dir_different_files() {
    // 異なるファイルなら異なるパスになるか（ハッシュ衝突がない限り）
    let r1 = utils::get_cache_root(false, "", "/dir/fileA.psd");
    let r2 = utils::get_cache_root(false, "", "/dir/fileB.psd");
    assert_ne!(r1, r2);
}

// =====================================================================
// get_file_size
// =====================================================================

#[test]
fn test_get_file_size_normal() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("f.txt");
    fs::write(&f, b"12345").unwrap();
    assert_eq!(utils::get_file_size(&f.to_string_lossy()).unwrap(), 5);
}

#[test]
fn test_get_file_size_empty_path() {
    assert!(utils::get_file_size("").is_err());
}


#[test]
fn test_get_file_size_directory() {
    let dir = tempdir().unwrap();
    let size = utils::get_file_size(&dir.path().to_string_lossy()).unwrap();
    assert_eq!(size, 0);
}

#[test]
fn test_get_file_size_nonexistent() {
    assert!(utils::get_file_size("/no/such/file.txt").is_err());
}

// =====================================================================
// read_text_file / write_text_file
// =====================================================================

#[test]
fn test_write_and_read_text_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("note.txt");

    utils::write_text_file(&path.to_string_lossy(), "hello\nworld").unwrap();
    let content = utils::read_text_file(&path.to_string_lossy()).unwrap();
    assert_eq!(content, "hello\nworld");
}

#[test]
fn test_read_text_file_nonexistent_returns_empty() {
    // 存在しないファイルは空文字を返す（エラーではない）
    let result = utils::read_text_file("/no/such/file.txt").unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_write_text_file_creates_parent() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sub/dir/note.txt");
    utils::write_text_file(&path.to_string_lossy(), "data").unwrap();
    assert!(path.exists());
}

// =====================================================================
// dir_exists / file_exists
// =====================================================================

#[test]
fn test_dir_exists() {
    let dir = tempdir().unwrap();
    assert!(utils::dir_exists(&dir.path().to_string_lossy()));
    assert!(!utils::dir_exists("/absolutely/nonexistent/dir"));
}

#[test]
fn test_file_exists() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("x.txt");
    assert!(!utils::file_exists(&f.to_string_lossy()));
    fs::write(&f, b"").unwrap();
    assert!(utils::file_exists(&f.to_string_lossy()));
    // ディレクトリはfalse
    assert!(!utils::file_exists(&dir.path().to_string_lossy()));
}
