mod app;
use crate::app::commands::*;
use crate::app::config::*;
use crate::app::state::AppState;
use crate::app::utils;
use app::menu::*;
use app::tray::*;
use std::fs;
use std::sync::Mutex;
use tauri::{menu::MenuEvent, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

pub fn handle_menu_event(app: &tauri::AppHandle, event: MenuEvent) {
    // AppStateを取得
    let state = app.state::<AppState>();

    match event.id.as_ref() {
        "always_on_top" => {
            if let Some(window) = app.get_webview_window("main") {
                // 1. ウィンドウ状態の反転（OS側の現在の状態を取得）
                let new_value = !window.is_always_on_top().unwrap_or(false);
                let _ = window.set_always_on_top(new_value);

                // 2. 設定の更新と保存
                {
                    let mut cfg = state.config.lock().unwrap();
                    cfg.always_on_top = new_value;
                }
                let _ = state.save();

                // 3. メニューのチェックマーク状態も更新（同期）
                if let Some(item) = app
                    .menu()
                    .and_then(|m| m.get("always_on_top"))
                    .and_then(|i| i.as_check_menuitem().cloned())
                {
                    let _ = item.set_checked(new_value);
                }
            }
        }

        "restore_state" => {
            // CheckMenuItemとして取得
            if let Some(item) = app
                .menu()
                .and_then(|m| m.get("restore_state"))
                .and_then(|i| i.as_check_menuitem().cloned())
            {
                let new_value = !item.is_checked().unwrap_or(false);
                let _ = item.set_checked(new_value);

                // 設定の更新と保存
                {
                    let mut cfg = state.config.lock().unwrap();
                    cfg.restore_previous_state = new_value;
                }
                let _ = state.save();
            }
        }
        "compact_mode" => {
            if let Some(window) = app.get_webview_window("main") {
                // 1. 現在のウィンドウタイトルで状態を判定 (メニューに依存しないので確実)
                let current_title = window.title().unwrap_or_default();
                let is_now_compact = current_title.contains("Compact mode");
                let new_flag = !is_now_compact;

                // 2. メニューのチェックマークを更新
                // .cloned() を入れることで所有権エラーを解決します
                if let Some(m) = app.menu() {
                    if let Some(item) = m
                        .get("compact_mode")
                        .and_then(|i| i.as_check_menuitem().cloned())
                    {
                        let _ = item.set_checked(new_flag);
                    }
                }

                // 3. ウィンドウ操作の実行
                let window_clone = window.clone();
                let _ = window.run_on_main_thread(move || {
                    let _ = utils::apply_compact_mode(&window_clone, new_flag);
                });

                // 4. フロントエンドへ通知
                let _ = app.emit("compact-mode-event", new_flag);
            }
        }

        "lang_en" | "lang_ja" => {
            let lang = if event.id.as_ref() == "lang_en" {
                "en".to_string()
            } else {
                "ja".to_string()
            };

            // 設定の更新と保存
            {
                let mut cfg = state.config.lock().unwrap();
                cfg.language = lang;
            }
            let _ = state.save();

            // 再起動を促すダイアログ
            app.dialog()
                .message("Restart required / 再起動が必要です")
                .kind(MessageDialogKind::Info)
                .title("Language")
                .show(|_| {});
        }

        "quit" => {
            app.exit(0);
        }

        "about" => {
            // 1. AppState を取得
            let state = app.state::<AppState>();

            // 2. ヘルパーを用意（i18n からテキストを抽出）
            let t = |key: &str| -> String {
                get_language_text(state.clone(), key).unwrap_or_else(|_| key.to_string())
            };

            // 3. ダイアログを表示
            app.dialog()
                // メッセージ本文を i18n の "about" キーから取得
                .message(t("aboutText"))
                .title(t("about"))
                .kind(tauri_plugin_dialog::MessageDialogKind::Info)
                .show(|_| {});
        }
        _ => {}
    }
}

pub fn handle_tray_event(app: &tauri::AppHandle, event: MenuEvent){
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let initial_size = tauri::Size::Logical(tauri::LogicalSize::new(640.0, 450.0));

            // 起動時に現在のサイズ（通常モード）で固定
            let _ = window.set_min_size(Some(initial_size));
            let _ = window.set_max_size(Some(initial_size));
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            // 1. 保存先パスを決定 (OS標準のAppConfigディレクトリ)
            let config_dir = app.path().app_config_dir()?;
            let config_path = config_dir.join("AppConfig.json");

            // 2. 読み込み試行
            let config = if config_path.exists() {
                let content = fs::read_to_string(&config_path)?;
                serde_json::from_str(&content).unwrap_or_else(|_| default_config())
            } else {
                default_config() // 初回起動時
            };

            // 3. TauriのStateに登録 (これで各コマンドから参照可能になる)
            app.manage(AppState {
                config: Mutex::new(config.clone()),
                config_path,
            });
            let menu = setup_menu(app.handle(), &config)?;
            let tray = setup_tray(app.handle(),&config);
            // アプリ全体にメニューをセット
            app.set_menu(menu)?;

            app.on_menu_event(move |app_handle, event| {
                handle_menu_event(app_handle, event);
            });
            app.on_menu_event(move |app_handle,event| {
                handle_tray_event(app_handle,event);
            });


            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 設定・i18n関連
            get_config,
            set_always_on_top,
            get_restore_previous_state,
            get_bsdiff_max_file_size,
            get_auto_base_generation_threshold,
            get_language_text,
            get_i18n,
            set_language,
            get_config_dir,
            // バックアップ・差分コアロジック
            backup_or_diff,
            apply_multi_diff,
            copy_backup_file,
            archive_backup_file,
            dir_exists,
            restore_backup,
            // ファイル操作・UI関連
            get_file_size,
            select_any_file,
            select_backup_folder,
            open_directory,
            toggle_compact_mode,
            write_text_file,
            read_text_file,
            get_backup_list,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
