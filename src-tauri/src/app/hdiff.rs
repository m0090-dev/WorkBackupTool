use crate::core::ext::hdiff_common::*;
use crate::core::utils;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

/// hdiffz を呼び出して差分を作成する
/// strict_hash_check: true のとき -C-all を付与して全ファイルのハッシュ検証を行う
pub async fn create_hdiff(
    app: AppHandle,
    old_file: &str,
    new_file: &str,
    diff_file: &str,
    compress_algo: &str,
) -> Result<(), String> {
    let mut args = build_hdiffz_args(old_file, new_file, diff_file, compress_algo);
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
/// strict_hash_check: true のとき -C-all を付与して全ファイルのハッシュ検証を行う
pub async fn apply_hdiff(
    app: AppHandle,
    base_full: &str,
    diff_file: &str,
    out_path: &str,
    strict_hash_check: bool,
) -> Result<(), String> {
    let mut args = build_hpatchz_args(base_full, diff_file, out_path);
    if strict_hash_check {
        args.push("-C-all");
    }

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
    strict_hash_check: bool,
) -> Result<(), String> {
    // 1. 仮の出力先パスを生成
    let temp_out = utils::auto_output_path(work_file);

    // 2. 共通モジュール(hdiff_common.rs)を使用してパスを確定させる
    let (base_full, out_path) = resolve_apply_paths(work_file, diff_file, temp_out)?;

    // 3. Sidecar (hpatchz) の実行
    crate::app::hdiff::apply_hdiff(app, &base_full, diff_file, &out_path, strict_hash_check).await
}
