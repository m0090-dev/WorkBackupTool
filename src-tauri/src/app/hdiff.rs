use tauri_plugin_shell::ShellExt;

/// hdiffz を呼び出して差分を作成する
pub async fn create_hdiff(
    app: tauri::AppHandle,
    old_file: &str,
    new_file: &str,
    diff_file: &str,
) -> Result<(), String> {
    // Sidecar "hdiffz" を呼び出し。引数はGo版と同一。
    let sidecar_command = app
        .shell()
        .sidecar("hdiffz")
        .map_err(|e| e.to_string())?
        .args(["-f", "-s", "-c-zstd", old_file, new_file, diff_file]);

    // Windowsでのウィンドウ非表示はTauriのSidecar/Commandが内部で処理してくれます。
    let output = sidecar_command.output().await.map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("hdiffz error: {}", err_msg))
    }
}

/// hpatchz を呼び出してパッチを適用（復元）する
pub async fn apply_hdiff(
    app: tauri::AppHandle,
    base_full: &str,
    diff_file: &str,
    out_path: &str,
) -> Result<(), String> {
    // Sidecar "hpatchz" を呼び出し
    let sidecar_command = app
        .shell()
        .sidecar("hpatchz")
        .map_err(|e| e.to_string())?
        .args(["-f", "-s", base_full, diff_file, out_path]);

    let output = sidecar_command.output().await.map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("hpatchz error: {}", err_msg))
    }
}
