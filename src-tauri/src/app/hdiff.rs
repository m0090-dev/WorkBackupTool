use crate::core::ext::hdiff_common::*;
use crate::core::types::DiffFileInfo;
use crate::core::utils;
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
/// hdiffz を呼び出して差分を作成する
pub async fn create_hdiff(
    app: AppHandle,
    old_file: &str,
    new_file: &str,
    diff_file: &str,
    compress_algo: &str,
) -> Result<(), String> {
    let args = build_hdiffz_args(old_file, new_file, diff_file, compress_algo);

    let output = app
        .shell()
        .sidecar("hdiffz")
        .map_err(|e| e.to_string())?
        .args(&args)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("hdiffz error: {}", err_msg))
    }
}

/// hpatchz を呼び出してパッチを適用（復元）する
pub async fn apply_hdiff(
    app: AppHandle,
    base_full: &str,
    diff_file: &str,
    out_path: &str,
) -> Result<(), String> {
    let args = build_hpatchz_args(base_full, diff_file, out_path);

    let output = app
        .shell()
        .sidecar("hpatchz")
        .map_err(|e| e.to_string())?
        .args(&args)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("hpatchz error: {}", err_msg))
    }
}

pub async fn apply_hdiff_wrapper(
    app: tauri::AppHandle,
    work_file: &str,
    diff_file: &str,
) -> Result<(), String> {
    // 1. 仮の出力先パスを生成
    let temp_out = utils::auto_output_path(work_file);

    // 2. 共通モジュール(hdiff_common.rs)を使用してパスを確定させる
    // ここで .20 分割、.base 探索、拡張子復元が「従来通り」行われます
    let (base_full, out_path) = resolve_apply_paths(work_file, diff_file, temp_out)?;

    // 3. Sidecar (hpatchz) の実行を既存の apply_hdiff 関数に投げる
    crate::app::hdiff::apply_hdiff(app, &base_full, diff_file, &out_path).await
}
