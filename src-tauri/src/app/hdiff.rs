use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

/// hdiffz 用の引数リストを生成するロジック
pub fn build_hdiffz_args<'a>(
    old_file: &'a str,
    new_file: &'a str,
    diff_file: &'a str,
    compress_algo: &'a str,
) -> Vec<&'a str> {
    let mut args = vec!["-f", "-s"];

    match compress_algo {
        "zstd" => args.push("-c-zstd"),
        "lzma2" => args.push("-c-lzma2"),
        "lzma" => args.push("-c-lzma"),
        "zlib" => args.push("-c-zlib"),
        "ldef" => args.push("-c-ldef"),
        "pbzip2" => args.push("-c-pbzip2"),
        "bzip2" => args.push("-c-bzip2"),
        "none" => {}               // uncompress
        _ => args.push("-c-zstd"), // default
    };

    args.push(old_file);
    args.push(new_file);
    args.push(diff_file);
    args
}

/// hpatchz 用の引数リストを生成するロジック
pub fn build_hpatchz_args<'a>(
    base_full: &'a str,
    diff_file: &'a str,
    out_path: &'a str,
) -> Vec<&'a str> {
    vec!["-f", "-s", base_full, diff_file, out_path]
}

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
