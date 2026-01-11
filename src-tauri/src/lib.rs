mod app;
use crate::app::commands::*;
use crate::app::config::*;
use crate::app::state::AppState;
use crate::app::utils;
use app::menu::*;
use app::tray::*;
use std::fs;
use std::sync::Mutex;
use tauri::AppHandle;
use tauri::{menu::MenuEvent, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

pub fn handle_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    let state = app.state::<AppState>();
    let id = event.id.as_ref();

    match id {
        // --- 1. 最前面表示 ---
        "always_on_top" => {
            // Config（現在のメモリ上の正解）を読み取って、その反対を「次の値」とする
            let next_val = {
                let cfg = state.config.lock().unwrap();
                !cfg.always_on_top
            };

            // ウィンドウ・メニューアイテム・Config のすべてを next_val で同期する
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(next_val);
            }
            if let Some(item) = app.menu().and_then(|m| m.get(id)).and_then(|i| i.as_check_menuitem().cloned()) {
                let _ = item.set_checked(next_val);
            }
            {
                let mut cfg = state.config.lock().unwrap();
                cfg.always_on_top = next_val;
            }
            let _ = state.save();
        }

        // --- 2. 状態復元 ---
        "restore_state" => {
            let next_val = {
                let cfg = state.config.lock().unwrap();
                !cfg.restore_previous_state
            };

            if let Some(item) = app.menu().and_then(|m| m.get(id)).and_then(|i| i.as_check_menuitem().cloned()) {
                let _ = item.set_checked(next_val);
            }
            {
                let mut cfg = state.config.lock().unwrap();
                cfg.restore_previous_state = next_val;
            }
            let _ = state.save();
        }

        // --- 3. トレイモード切替 ---
        "tray_mode" => {
            let next_val = {
                let cfg = state.config.lock().unwrap();
                !cfg.tray_mode
            };

            if let Some(item) = app.menu().and_then(|m| m.get(id)).and_then(|i| i.as_check_menuitem().cloned()) {
                let _ = item.set_checked(next_val);
            }
            {
                let mut cfg = state.config.lock().unwrap();
                cfg.tray_mode = next_val;
            }
            let _ = state.save();

            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                // tray_mode=true なら ウィンドウを隠す(show=false)
                let _ = utils::apply_window_visibility(app_clone, !next_val);
            });
        }

        // --- 4. コンパクトモード ---
        "compact_mode" => {
            if let Some(window) = app.get_webview_window("main") {
                let current_title = window.title().unwrap_or_default();
                let next_val = !current_title.contains("Compact mode");

                if let Some(item) = app.menu().and_then(|m| m.get(id)).and_then(|i| i.as_check_menuitem().cloned()) {
                    let _ = item.set_checked(next_val);
                }
                
                let window_clone = window.clone();
                let _ = window.run_on_main_thread(move || {
                    let _ = utils::apply_compact_mode(&window_clone, next_val);
                });
                let _ = app.emit("compact-mode-event", next_val);
            }
        }

        // --- アクション系 ---
    
        "show_window" => {
            // 1. まずウィンドウを表示する
            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                let _ = utils::apply_window_visibility(app_clone, true);
            });

            // 2. tray_mode の設定を false に戻す
            {
                let mut cfg = state.config.lock().unwrap();
                cfg.tray_mode = false;
            }
            let _ = state.save();

            // 3. メニューの「tray_mode」のチェックをオフにする
            if let Some(item) = app.menu()
                .and_then(|m| m.get("tray_mode"))
                .and_then(|i| i.as_check_menuitem().cloned()) 
            {
                let _ = item.set_checked(false);
            }
        }


        "execute" => { let _ = app.emit("tray-execute-clicked", ()); }
        "change_work" => { let _ = app.emit("tray-change-work-clicked", ()); }
        "change_backup" => { let _ = app.emit("tray-change-backup-clicked", ()); }

        // --- 言語・About・Quit ---
        "lang_en" | "lang_ja" => {
            let lang = if id == "lang_en" { "en" } else { "ja" };
            {
                let mut cfg = state.config.lock().unwrap();
                cfg.language = lang.to_string();
            }
            let _ = state.save();
            let _ = app.dialog().message("Restart required").title("Language").show(|_| {});
        }
        "about" => {
            let t = |key: &str| get_language_text(state.clone(), key).unwrap_or_else(|_| key.to_string());
            let _ = app.dialog().message(t("aboutText")).title(t("about")).show(|_| {});
        }
        "quit" => { app.exit(0); }
        _ => {}
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
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
            let tray = setup_tray(app.handle(), &config);
            // アプリ全体にメニューをセット
            app.set_menu(menu)?;

            app.on_menu_event(move |app_handle, event| {
                handle_menu_event(app_handle, event);
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
