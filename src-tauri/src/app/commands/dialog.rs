// 標準ライブラリ

// 外部クレート
use tauri::{AppHandle, Manager};

// Tauriプラグイン
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::ShellExt;

// 内部モジュール (自作)
use crate::app::state::AppState;

#[tauri::command]
pub async fn select_any_file(app: AppHandle, title: String) -> Result<Option<String>, String> {
    // 1. メインウィンドウとAppStateを取得
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    let state = app.state::<AppState>();

    // 2. 現在の AlwaysOnTop 設定を確認し、有効なら一時解除
    let is_always_on_top = {
        let cfg = state.config.lock().unwrap();
        cfg.always_on_top
    };

    if is_always_on_top {
        #[cfg(desktop)]
        {
            let _ = window.set_always_on_top(false);
        }
    }

    // 3. ダイアログを表示 (既存ロジック)
    let file_path = window
        .dialog()
        .file()
        .set_title(&title)
        .blocking_pick_file();

    // 4. 設定を元に戻す
    if is_always_on_top {
        #[cfg(desktop)]
        {
            let _ = window.set_always_on_top(true);
        }
    }

    match file_path {
        Some(path) => Ok(Some(path.to_string())),
        None => Ok(None),
    }
}

/// フォルダ選択ダイアログを表示する
#[tauri::command]
pub async fn select_any_folder(app: AppHandle,title: String) -> Result<Option<String>, String> {
    // 1. メインウィンドウとAppStateを取得
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    let state = app.state::<AppState>();

    // 2. 現在の AlwaysOnTop 設定を確認し、有効なら一時解除
    let is_always_on_top = {
        let cfg = state.config.lock().unwrap();
        cfg.always_on_top
    };

    if is_always_on_top {
        #[cfg(desktop)]
        {
            let _ = window.set_always_on_top(false);
        }
    }

    // 3. ダイアログを表示
    let folder_path: Option<tauri_plugin_dialog::FilePath> = {
        #[cfg(desktop)]
        {
            window
                .dialog()
                .file()
                .set_title(&title)
                .blocking_pick_folder()
        }
        #[cfg(mobile)]
        {
            // モバイルではとりあえず None を返してコンパイルを通す
            None
        }
    };
    // 4. 設定を元に戻す
    if is_always_on_top {
        #[cfg(desktop)]
        {
            let _ = window.set_always_on_top(true);
        }
    }

    match folder_path {
        Some(path) => Ok(Some(path.to_string())),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn open_directory(app: tauri::AppHandle, path: String) -> Result<(), String> {
    // 1. パスの親ディレクトリ（フォルダ）を取得
    let target = std::path::Path::new(&path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    // シェル操作（フォルダを開く）は、ダイアログとは異なり
    // OS自体の別アプリ（Explorer/Finder）を起動するため app.shell() のままで問題ありません
    app.shell()
        .open(target.to_string_lossy().to_string(), None)
        .map_err(|e| e.to_string())?;

    Ok(())
}
