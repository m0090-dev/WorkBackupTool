use serde_json::json;
use std::fs;
use tempfile::tempdir;
use work_backup_tool::core::backup::archive;
use work_backup_tool::core::config::loader::default_config;
use work_backup_tool::core::config::update::apply_config_value;
use work_backup_tool::core::ext::hdiff_common;
use work_backup_tool::core::utils;

// ---- 削除済みの「ドット入りフォルダ」対象 -------------------------------------

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
    let name = std::path::Path::new(&out).file_name().unwrap().to_string_lossy().into_owned();
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
    let work = tmp.path().join("my.project"); // 存在しない
    assert!(hdiff_common::restore_target_is_folder(
        work.to_str().unwrap(),
        diff.to_str().unwrap()
    ));

    // ファイル対象
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

#[test]
fn test_resolve_apply_paths_deleted_dotted_folder_keeps_out_path() {
    let tmp = tempdir().unwrap();
    let bdir = tmp.path().join("wbt_backup_my.project");
    fs::create_dir_all(bdir.join("my.project.base")).unwrap();
    let diff = bdir.join("my.project.20250101_000000.hdiff.diff");
    fs::write(&diff, b"x").unwrap();
    let work = tmp.path().join("my.project");
    let out = utils::auto_output_path_as(work.to_str().unwrap(), true);
    let (_, resolved) = hdiff_common::resolve_apply_paths(
        work.to_str().unwrap(),
        diff.to_str().unwrap(),
        out.clone(),
    )
    .unwrap();
    assert_eq!(resolved, out);
}

// ---- アーカイブの中身がフォルダかどうか ---------------------------------------

#[test]
fn test_archive_holds_folder_zip_and_tar() {
    let tmp = tempdir().unwrap();
    let folder = tmp.path().join("my.project");
    fs::create_dir_all(folder.join("sub")).unwrap();
    fs::write(folder.join("a.txt"), b"a").unwrap();
    fs::write(folder.join("sub").join("b.txt"), b"b").unwrap();
    let file = tmp.path().join("one.txt");
    fs::write(&file, b"one").unwrap();

    let z_dir = tmp.path().join("f.zip");
    archive::compress_dir_zip(&folder, &z_dir, "").unwrap();
    assert!(archive::archive_holds_folder(z_dir.to_str().unwrap()).unwrap());

    let t_dir = tmp.path().join("f.tar.gz");
    archive::compress_dir_tar(&folder, &t_dir).unwrap();
    assert!(archive::archive_holds_folder(t_dir.to_str().unwrap()).unwrap());

    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let z_file = archive::zip_backup_file(file.to_str().unwrap(), &out, "").unwrap();
    assert!(!archive::archive_holds_folder(&z_file).unwrap());
    let t_file = archive::tar_backup_file(file.to_str().unwrap(), &out).unwrap();
    assert!(!archive::archive_holds_folder(&t_file).unwrap());

    // パスワード付きZIPでも、中身の種類は調べられる
    let z_pass = archive::zip_backup_file(file.to_str().unwrap(), &out, "pw").unwrap();
    assert!(!archive::archive_holds_folder(&z_pass).unwrap());
}

// ---- 新しい設定 -----------------------------------------------------------------

#[test]
fn test_restore_relink_defaults_and_update() {
    let mut cfg = default_config();
    assert!(cfg.restore_relink_enabled);
    assert!(!cfg.restore_relink_auto);
    assert!(cfg.restore_relink_only_when_missing);
    assert!(cfg.restore_relink_reset_backup_dir);

    apply_config_value(&mut cfg, "restoreRelinkEnabled", &json!(false)).unwrap();
    apply_config_value(&mut cfg, "restoreRelinkAuto", &json!(true)).unwrap();
    apply_config_value(&mut cfg, "restoreRelinkOnlyWhenMissing", &json!(false)).unwrap();
    apply_config_value(&mut cfg, "restoreRelinkResetBackupDir", &json!(false)).unwrap();
    assert!(!cfg.restore_relink_enabled);
    assert!(cfg.restore_relink_auto);
    assert!(!cfg.restore_relink_only_when_missing);
    assert!(!cfg.restore_relink_reset_backup_dir);
    assert!(apply_config_value(&mut cfg, "restoreRelinkAuto", &json!("yes")).is_err());
}
