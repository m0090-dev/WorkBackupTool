use crate::core::utils;

#[tauri::command]
pub fn get_file_size(path: String) -> Result<i64, String> {
    utils::get_file_size(&path)
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    utils::read_text_file(&path)
}

#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    utils::write_text_file(&path, &content)
}

#[tauri::command]
pub fn dir_exists(path: String) -> Result<bool, String> {
    Ok(utils::dir_exists(&path))
}

#[tauri::command]
pub fn file_exists(path: String) -> Result<bool, String> {
    Ok(utils::file_exists(&path))
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn show_about(app: tauri::AppHandle, state: tauri::State<'_, crate::app::state::AppState>) {
    use tauri::Manager;
    use tauri_plugin_dialog::DialogExt;

    let t = |key: &str| -> String { state.translate(key).unwrap_or_else(|_| key.to_string()) };
    let is_always_on_top = state.config.lock().unwrap().always_on_top;

    if let Some(window) = app.get_webview_window("main") {
        if is_always_on_top {
            let _ = window.set_always_on_top(false);
        }
        let window_clone = window.clone();
        window
            .dialog()
            .message(t("aboutText"))
            .title(t("about"))
            .show(move |_| {
                if is_always_on_top {
                    let _ = window_clone.set_always_on_top(true);
                }
            });
    }
}

/// 復元したファイル/フォルダを、OS の既定の方法で開く。
/// open_parent が true のときは、その置き場所(親フォルダ)を開く。
/// 実行形式のファイルは起動せず、置き場所のフォルダを開く。
#[tauri::command]
pub fn open_path(path: String, open_parent: Option<bool>) -> Result<(), String> {
    utils::open_in_os(&path, open_parent.unwrap_or(false))
}
