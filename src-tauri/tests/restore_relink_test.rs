use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use work_backup_tool::core::backup::{archive, scanner};
use work_backup_tool::core::config::loader::default_config;
use work_backup_tool::core::config::update::apply_config_value;
use work_backup_tool::core::ext::hdiff_common;
use work_backup_tool::core::utils;

// =====================================================================
// 作業対象が存在しない(削除済み)ときの名前・場所の判断
// =====================================================================

#[test]
fn test_default_backup_dir_deleted_dotted_folder_uses_existing_backup() {
    let tmp = tempdir().unwrap();
    let backup = tmp.path().join("wbt_backup_my.project");
    fs::create_dir_all(&backup).unwrap();
    let work = tmp.path().join("my.project"); // 存在しない
    assert_eq!(utils::default_backup_dir(work.to_str().unwrap()), backup);
}

#[test]
fn test_default_backup_dir_deleted_file_still_uses_stem() {
    let tmp = tempdir().unwrap();
    let work = tmp.path().join("art.clip"); // 存在しない
    assert_eq!(
        utils::default_backup_dir(work.to_str().unwrap()),
        tmp.path().join("wbt_backup_art")
    );
}

#[test]
fn test_default_backup_dir_existing_dotted_folder() {
    let tmp = tempdir().unwrap();
    let work = tmp.path().join("my.project");
    fs::create_dir_all(&work).unwrap();
    assert_eq!(
        utils::default_backup_dir(work.to_str().unwrap()),
        tmp.path().join("wbt_backup_my.project")
    );
}

#[test]
fn test_auto_output_path_as_folder_keeps_dotted_name() {
    let out = utils::auto_output_path_as("/dir/my.project", true);
    let name = Path::new(&out).file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("my.project_restored_"), "{}", name);
    assert!(!name.ends_with(".project"), "{}", name);
}

#[test]
fn test_restore_target_is_folder_when_work_is_missing() {
    let tmp = tempdir().unwrap();
    let bdir = tmp.path().join("wbt_backup_my.project");
    fs::create_dir_all(bdir.join("my.project.base")).unwrap();
    let diff = bdir.join("my.project.20250101_000000.hdiff.diff");
    fs::write(&diff, b"x").unwrap();
    let work = tmp.path().join("my.project");
    assert!(hdiff_common::restore_target_is_folder(
        work.to_str().unwrap(),
        diff.to_str().unwrap()
    ));

    let bdir2 = tmp.path().join("wbt_backup_art");
    fs::create_dir_all(&bdir2).unwrap();
    fs::write(bdir2.join("art.clip.base"), b"base").unwrap();
    let diff2 = bdir2.join("art.clip.20250101_000000.hdiff.diff");
    fs::write(&diff2, b"x").unwrap();
    let work2 = tmp.path().join("art.clip");
    assert!(!hdiff_common::restore_target_is_folder(
        work2.to_str().unwrap(),
        diff2.to_str().unwrap()
    ));
}

// =====================================================================
// 作業対象が「空」のときの、復元先フォルダの決め方
// =====================================================================

#[test]
fn test_auto_output_dir_standard_layout() {
    assert_eq!(
        utils::auto_output_dir("/x/art/wbt_backup_work"),
        Some(PathBuf::from("/x/art"))
    );
    // 末尾の区切りがあっても同じ
    assert_eq!(
        utils::auto_output_dir("/x/art/wbt_backup_work/"),
        Some(PathBuf::from("/x/art"))
    );
    // 世代フォルダが保存先に指定されていても、その親を見る
    assert_eq!(
        utils::auto_output_dir("/x/art/wbt_backup_work/base2_20250101_000000"),
        Some(PathBuf::from("/x/art"))
    );
}

#[test]
fn test_auto_output_dir_non_standard_returns_none() {
    assert_eq!(utils::auto_output_dir("/mnt/backups/work"), None);
    assert_eq!(utils::auto_output_dir(""), None);
    // base で始まるだけの普通のフォルダ名は、世代フォルダ扱いしない
    assert_eq!(utils::auto_output_dir("/x/baseball"), None);
    // 親が無い(相対の単独名)
    assert_eq!(utils::auto_output_dir("wbt_backup_work"), None);
}

#[test]
fn test_resolve_output_dir_priority() {
    // ユーザーが選んだフォルダが最優先(「常に尋ねる」でも使う)
    assert_eq!(
        utils::resolve_output_dir("/x/wbt_backup_a", Some("/chosen"), true).unwrap(),
        PathBuf::from("/chosen")
    );
    // 標準の配置は自動
    assert_eq!(
        utils::resolve_output_dir("/x/wbt_backup_a", None, false).unwrap(),
        PathBuf::from("/x")
    );
    // 「常に尋ねる」なら、標準の配置でも尋ねる
    assert_eq!(
        utils::resolve_output_dir("/x/wbt_backup_a", None, true).unwrap_err(),
        utils::ERR_OUTPUT_DIR_REQUIRED
    );
    // 標準でない配置は尋ねる
    assert_eq!(
        utils::resolve_output_dir("/mnt/backups", None, false).unwrap_err(),
        utils::ERR_OUTPUT_DIR_REQUIRED
    );
    // 空文字の選択は「選んでいない」扱い
    assert!(utils::resolve_output_dir("/mnt/backups", Some(""), false).is_err());
}

#[test]
fn test_diff_entry_name() {
    assert_eq!(
        hdiff_common::diff_entry_name("/b/base1_x/art.clip.20251231_120000.hdiff.diff").as_deref(),
        Some("art.clip")
    );
    // 名前に「.20」が含まれても取り違えない
    assert_eq!(
        hdiff_common::diff_entry_name("/b/report.2024.psd.20251231_120000.hdiff.diff").as_deref(),
        Some("report.2024.psd")
    );
    assert_eq!(
        hdiff_common::diff_entry_name("/b/my.project.20251231_120000.hdiff.diff").as_deref(),
        Some("my.project")
    );
    assert_eq!(hdiff_common::diff_entry_name("/b/broken.diff"), None);
}

#[test]
fn test_get_cache_root_empty_work_uses_backup_dir() {
    let a = utils::get_cache_root(false, "/mnt/b1", "");
    let b = utils::get_cache_root(false, "/mnt/b2", "");
    assert_ne!(a, b, "保存先が違えば、キャッシュも別");
    // 末尾の区切りがあっても同じ
    assert_eq!(a, utils::get_cache_root(false, "/mnt/b1/", ""));
    // 同じ保存先なら、いつも同じ
    assert_eq!(a, utils::get_cache_root(false, "/mnt/b1", ""));
    // 保存先と同じ場所に置く設定
    assert_eq!(
        utils::get_cache_root(true, "/mnt/b1", ""),
        PathBuf::from("/mnt/b1").join(".wbt_cache")
    );
}

#[test]
fn test_scan_backups_with_empty_work_path() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("wbt_backup_art");
    let gen = root.join("base1_20250101_000000");
    fs::create_dir_all(&gen).unwrap();
    fs::write(gen.join("art.clip.base"), b"base").unwrap();
    fs::write(gen.join("art.clip.20250101_000100.hdiff.diff"), b"d").unwrap();

    let list = scanner::scan_backups("", root.to_str().unwrap(), true, true);
    assert_eq!(list.len(), 1, "{:?}", list);
    assert!(list[0].file_name.ends_with(".diff"));
}

// =====================================================================
// アーカイブの中身(元の名前・フォルダかどうか)
// =====================================================================

#[test]
fn test_archive_target_info_zip_and_tar() {
    let tmp = tempdir().unwrap();
    let folder = tmp.path().join("my.project");
    fs::create_dir_all(folder.join("sub")).unwrap();
    fs::write(folder.join("a.txt"), b"a").unwrap();
    fs::write(folder.join("sub").join("b.txt"), b"b").unwrap();
    let file = tmp.path().join("one.txt");
    fs::write(&file, b"one").unwrap();

    let z_dir = tmp.path().join("f.zip");
    archive::compress_dir_zip(&folder, &z_dir, "").unwrap();
    assert_eq!(
        archive::archive_target_info(z_dir.to_str().unwrap()).unwrap(),
        ("my.project".to_string(), true)
    );
    assert!(archive::archive_holds_folder(z_dir.to_str().unwrap()).unwrap());

    let t_dir = tmp.path().join("f.tar.gz");
    archive::compress_dir_tar(&folder, &t_dir).unwrap();
    assert_eq!(
        archive::archive_target_info(t_dir.to_str().unwrap()).unwrap(),
        ("my.project".to_string(), true)
    );

    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let z_file = archive::zip_backup_file(file.to_str().unwrap(), &out, "").unwrap();
    assert_eq!(
        archive::archive_target_info(&z_file).unwrap(),
        ("one.txt".to_string(), false)
    );
    let t_file = archive::tar_backup_file(file.to_str().unwrap(), &out).unwrap();
    assert_eq!(
        archive::archive_target_info(&t_file).unwrap(),
        ("one.txt".to_string(), false)
    );
    // パスワード付きZIPでも、パスワードなしで中身の種類を調べられる
    let z_pass = archive::zip_backup_file(file.to_str().unwrap(), &out, "pw").unwrap();
    assert_eq!(
        archive::archive_target_info(&z_pass).unwrap(),
        ("one.txt".to_string(), false)
    );
}

#[test]
fn test_restore_zip_with_empty_work_target_end_to_end() {
    // 作業対象が空 → 保存先の隣に「元の名前_restored_日時.拡張子」で復元される流れ
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("proj");
    let backup_root = project.join("wbt_backup_art");
    fs::create_dir_all(&backup_root).unwrap();
    let original = tmp.path().join("art.clip");
    fs::write(&original, b"hello art").unwrap();
    let zip = archive::zip_backup_file(original.to_str().unwrap(), &backup_root, "").unwrap();

    let dir = utils::resolve_output_dir(backup_root.to_str().unwrap(), None, false).unwrap();
    assert_eq!(dir, project);
    let (name, is_folder) = archive::archive_target_info(&zip).unwrap();
    let virtual_work = dir.join(&name);
    let out = utils::auto_output_path_as(virtual_work.to_str().unwrap(), is_folder);
    archive::restore_archive_with_password(&zip, &out, None).unwrap();

    let out_name = Path::new(&out).file_name().unwrap().to_string_lossy().into_owned();
    assert!(out_name.starts_with("art_restored_") && out_name.ends_with(".clip"), "{}", out_name);
    assert_eq!(Path::new(&out).parent().unwrap(), project);
    assert_eq!(fs::read(&out).unwrap(), b"hello art");
}

// =====================================================================
// 設定
// =====================================================================

#[test]
fn test_restore_settings_defaults_and_update() {
    let mut cfg = default_config();
    assert!(cfg.restore_relink_enabled);
    assert!(!cfg.restore_relink_auto);
    assert!(cfg.restore_relink_only_when_missing);
    assert!(cfg.restore_relink_reset_backup_dir);
    assert!(!cfg.restore_always_ask_output_dir);

    apply_config_value(&mut cfg, "restoreRelinkEnabled", &json!(false)).unwrap();
    apply_config_value(&mut cfg, "restoreRelinkAuto", &json!(true)).unwrap();
    apply_config_value(&mut cfg, "restoreRelinkOnlyWhenMissing", &json!(false)).unwrap();
    apply_config_value(&mut cfg, "restoreRelinkResetBackupDir", &json!(false)).unwrap();
    apply_config_value(&mut cfg, "restoreAlwaysAskOutputDir", &json!(true)).unwrap();
    assert!(!cfg.restore_relink_enabled);
    assert!(cfg.restore_relink_auto);
    assert!(!cfg.restore_relink_only_when_missing);
    assert!(!cfg.restore_relink_reset_backup_dir);
    assert!(cfg.restore_always_ask_output_dir);
    assert!(apply_config_value(&mut cfg, "restoreRelinkAuto", &json!("yes")).is_err());
}
