use std::fs;
use tempfile::tempdir;
use work_backup_tool::core::ext::hdiff_common;

// =====================================================================
// build_hdiffz_args
// =====================================================================

#[test]
fn test_build_hdiffz_args_zstd() {
    let args = hdiff_common::build_hdiffz_args("old.bin", "new.bin", "out.diff", "zstd", &[]);
    assert_eq!(
        args,
        vec!["-f", "-s", "-c-zstd", "old.bin", "new.bin", "out.diff"]
            .into_iter().map(String::from).collect::<Vec<_>>()
    );
}

#[test]
fn test_build_hdiffz_args_lzma2() {
    let args = hdiff_common::build_hdiffz_args("o", "n", "d", "lzma2", &[]);
    assert_eq!(args[2], "-c-lzma2");
}

#[test]
fn test_build_hdiffz_args_lzma() {
    let args = hdiff_common::build_hdiffz_args("o", "n", "d", "lzma", &[]);
    assert_eq!(args[2], "-c-lzma");
}

#[test]
fn test_build_hdiffz_args_zlib() {
    let args = hdiff_common::build_hdiffz_args("o", "n", "d", "zlib", &[]);
    assert_eq!(args[2], "-c-zlib");
}

#[test]
fn test_build_hdiffz_args_ldef() {
    let args = hdiff_common::build_hdiffz_args("o", "n", "d", "ldef", &[]);
    assert_eq!(args[2], "-c-ldef");
}

#[test]
fn test_build_hdiffz_args_bzip2() {
    let args = hdiff_common::build_hdiffz_args("o", "n", "d", "bzip2", &[]);
    assert_eq!(args[2], "-c-bzip2");
}

#[test]
fn test_build_hdiffz_args_pbzip2() {
    let args = hdiff_common::build_hdiffz_args("o", "n", "d", "pbzip2", &[]);
    assert_eq!(args[2], "-c-pbzip2");
}

#[test]
fn test_build_hdiffz_args_none_no_compress_flag() {
    // "none" の場合は -c-xxx が付かないので引数は5つ
    let args = hdiff_common::build_hdiffz_args("old.bin", "new.bin", "out.diff", "none", &[]);
    assert_eq!(args.len(), 5);
    assert_eq!(args[0], "-f");
    assert_eq!(args[1], "-s");
    assert_eq!(args[2], "old.bin");
    assert_eq!(args[3], "new.bin");
    assert_eq!(args[4], "out.diff");
}

#[test]
fn test_build_hdiffz_args_unknown_defaults_to_zstd() {
    // 未知のアルゴはzstdにフォールバック
    let args = hdiff_common::build_hdiffz_args("o", "n", "d", "unknown_algo", &[]);
    assert_eq!(args[2], "-c-zstd");
}

#[test]
fn test_build_hdiffz_args_paths_preserved() {
    let old = "/path/to/old file.bin";
    let new = "/path/to/new file.bin";
    let diff = "/output/my.diff";
    let args = hdiff_common::build_hdiffz_args(old, new, diff, "zstd", &[]);
    assert_eq!(args[3], old);
    assert_eq!(args[4], new);
    assert_eq!(args[5], diff);
}

// =====================================================================
// build_hpatchz_args
// =====================================================================

#[test]
fn test_build_hpatchz_args_basic() {
    let args = hdiff_common::build_hpatchz_args("base.clip", "patch.diff", "out.clip", false);
    assert_eq!(
        args,
        vec!["-f", "-s", "base.clip", "patch.diff", "out.clip"]
    );
}

#[test]
fn test_build_hpatchz_args_always_5_elements() {
    let args = hdiff_common::build_hpatchz_args("a", "b", "c", false);
    assert_eq!(args.len(), 5);
}

#[test]
fn test_build_hpatchz_args_paths_preserved() {
    let base = "/backup/base1/work.clip.base";
    let diff = "/backup/base1/work.clip.20260101.hdiff.diff";
    let out = "/work/work_restored.clip";
    let args = hdiff_common::build_hpatchz_args(base, diff, out, false);
    assert_eq!(args[2], base);
    assert_eq!(args[3], diff);
    assert_eq!(args[4], out);
}

// =====================================================================
// prepare_hdiff_paths
// =====================================================================

#[test]
fn test_prepare_hdiff_paths_no_base_creates_base_and_returns_none() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"canvas").unwrap();

    let result =
        hdiff_common::prepare_hdiff_paths(&work.to_string_lossy(), dir.path().to_path_buf())
            .unwrap();

    // .base が存在しなかった → コピーして None を返す
    assert!(result.is_none());
    assert!(dir.path().join("work.clip.base").exists());
    assert_eq!(
        fs::read(dir.path().join("work.clip.base")).unwrap(),
        b"canvas"
    );
}

#[test]
fn test_prepare_hdiff_paths_with_base_returns_some() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"v2").unwrap();

    // 既にbaseが存在する状態
    fs::write(dir.path().join("work.clip.base"), b"v1").unwrap();

    let result =
        hdiff_common::prepare_hdiff_paths(&work.to_string_lossy(), dir.path().to_path_buf())
            .unwrap();

    // baseが存在した → Some((base_path, work_path, diff_path)) が返る
    assert!(result.is_some());
    let (base, work_out, _diff) = result.unwrap();
    assert!(base.ends_with("work.clip.base"));
    assert!(work_out.ends_with("work.clip"));
}

#[test]
fn test_prepare_hdiff_paths_creates_target_dir() {
    let dir = tempdir().unwrap();
    let work = dir.path().join("work.clip");
    fs::write(&work, b"data").unwrap();

    // 存在しないサブディレクトリを渡す
    let target = dir.path().join("new_gen_dir");
    assert!(!target.exists());

    hdiff_common::prepare_hdiff_paths(&work.to_string_lossy(), target.clone()).unwrap();

    assert!(target.exists());
}

#[test]
fn test_prepare_hdiff_paths_nonexistent_work_file() {
    let dir = tempdir().unwrap();
    // work fileが存在しない + baseもない → fs::copy失敗でエラー
    let result = hdiff_common::prepare_hdiff_paths("/no/work.clip", dir.path().to_path_buf());
    assert!(result.is_err());
}

// =====================================================================
// resolve_apply_paths
// =====================================================================

#[test]
fn test_resolve_apply_paths_basic() {
    let dir = tempdir().unwrap();
    let base = dir.path().join("work.clip.base");
    fs::write(&base, b"base").unwrap();

    // diff名: work.clip.20260101_100000.hdiff.diff
    let diff = dir.path().join("work.clip.20260101_100000.hdiff.diff");
    let work_file = "/some/path/work.clip";
    let temp_out = "/tmp/work_restored.clip";

    let (base_path, out_path) =
        hdiff_common::resolve_apply_paths(work_file, &diff.to_string_lossy(), temp_out.to_string())
            .unwrap();

    // base_path が backup_dir/work.clip.base を指しているか
    assert!(base_path.ends_with("work.clip.base"));
    assert!(std::path::Path::new(&base_path).exists());
    // out_path の拡張子がclipになっているか
    assert!(out_path.ends_with(".clip"));
}

#[test]
fn test_resolve_apply_paths_fallback_to_work_basename() {
    let dir = tempdir().unwrap();

    // diff名と一致するbaseファイルがない → work_file名のbaseにフォールバック
    let work_base = dir.path().join("work.clip.base");
    fs::write(&work_base, b"fallback base").unwrap();

    let diff = dir.path().join("other.clip.20260101_100000.hdiff.diff");
    let work_file = "/path/to/work.clip";

    let (base_path, _) = hdiff_common::resolve_apply_paths(
        work_file,
        &diff.to_string_lossy(),
        "/tmp/out.clip".to_string(),
    )
    .unwrap();

    assert!(base_path.ends_with("work.clip.base"));
}
