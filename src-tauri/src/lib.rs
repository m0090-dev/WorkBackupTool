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
use tauri_plugin_notification::NotificationExt;


pub fn handle_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    let state = app.state::<AppState>();
    let id = event.id.as_ref();

    match id {
        // --- トレイモード切替 ---
        "tray_mode" => {
            // 1. Configを読み取って「反転」させる (ここを絶対の正解とする)
            let next_is_tray_enabled = {
                let mut cfg = state.config.lock().unwrap();
                cfg.tray_mode = !cfg.tray_mode;
                cfg.tray_mode // 反転後の値を保持
            };

            // 2. ウィンドウ表示状態を反映 (trueならhide、falseならshow)
            let _ = utils::apply_window_visibility(app.clone(), !next_is_tray_enabled);

            // 3. メニューアイテムのチェック状態を強制同期
            if let Some(item) = app.menu().and_then(|m| m.get(id)).and_then(|i| i.as_check_menuitem().cloned()) {
                let _ = item.set_checked(next_is_tray_enabled);
            }
            let _ = app.menu().and_then(|m| m.get("tray_mode")).map(|i| {
                if let Some(check) = i.as_check_menuitem() {
                    let _ = check.set_checked(next_is_tray_enabled);
                }
            });

            // 4. 保存
            let _ = state.save();
        }
     
        // --- ウィンドウ表示アクション (トレイから復帰時) ---
        "show_window" => {
            // 1. Configを「トレイモードOFF」に固定して保存
            let config = {
                let mut cfg = state.config.lock().unwrap();
                cfg.tray_mode = false;
                cfg.clone() // 新しいメニュー作成用にクローン
            };
            let _ = state.save();

            // 2. ウィンドウを出す
            let _ = utils::apply_window_visibility(app.clone(), true);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }

            // 3. 【最重要】メニューを「現在のConfig（tray_mode=false）」で作り直して再適用
            // これにより、OS側の古いチェック状態を強制的に上書き（リフレッシュ）します
            if let Ok(new_menu) = setup_menu(app, &config) {
                let _ = app.set_menu(new_menu);
            }
        }

        // --- 1. 最前面表示 (ここもConfig基準に修正) ---
        "always_on_top" => {
            let next_val = {
                let mut cfg = state.config.lock().unwrap();
                cfg.always_on_top = !cfg.always_on_top;
                cfg.always_on_top
            };

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(next_val);
            }
            if let Some(item) = app.menu().and_then(|m| m.get(id)).and_then(|i| i.as_check_menuitem().cloned()) {
                let _ = item.set_checked(next_val);
            }
            let _ = state.save();
        }

      
        // --- コンパクトモード ---
        "compact_mode" => {
            let next_val = {
                let mut cfg = state.config.lock().unwrap();
                cfg.compact_mode = !cfg.compact_mode; // ここで状態を反転
                cfg.compact_mode
            };

            if let Some(window) = app.get_webview_window("main") {
                let _ = utils::apply_compact_mode(&window, next_val);
                let _ = app.emit("compact-mode-event", next_val);
            }
            
            // メニューのチェックも更新
            if let Some(item) = app.menu().and_then(|m| m.get(id)).and_then(|i| i.as_check_menuitem().cloned()) {
                let _ = item.set_checked(next_val);
            }
            let _ = state.save();
        }

        // --- 5. 状態復元 ---
        "restore_state" => {
            let next_val = {
                let mut cfg = state.config.lock().unwrap();
                cfg.restore_previous_state = !cfg.restore_previous_state;
                cfg.restore_previous_state
            };
            if let Some(item) = app.menu().and_then(|m| m.get(id)).and_then(|i| i.as_check_menuitem().cloned()) {
                let _ = item.set_checked(next_val);
            }
            let _ = state.save();
        }

        "execute" => { 
            let _ = app.emit("tray-execute-clicked", ()); 
            app.notification()
            .builder()
            .title("Tauri")
            .body("Tauri is awesome")
            .show()
            .unwrap();
        }
        "change_work" => { 
            let _ = app.emit("tray-change-work-clicked", ()); 
            app.notification()
            .builder()
            .title("Tauri")
            .body("Tauri is awesome")
            .show()
            .unwrap();
        }
        "change_backup" => { 
            let _ = app.emit("tray-change-backup-clicked", ()); 
            app.notification()
            .builder()
            .title("Tauri")
            .body("Tauri is awesome")
            .show()
            .unwrap();
        }
        "lang_en" | "lang_ja" => {
            let lang = if id == "lang_en" { "en" } else { "ja" };
            state.config.lock().unwrap().language = lang.to_string();
            let _ = state.save();
            let _ = app.dialog().message("Restart required").show(|_| {});
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

            let _ = window.set_min_size(Some(initial_size));
            let _ = window.set_max_size(Some(initial_size));
            
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let config_dir = app.path().app_config_dir()?;
            let config_path = config_dir.join("AppConfig.json");

            let config = if config_path.exists() {
                let content = fs::read_to_string(&config_path)?;
                serde_json::from_str(&content).unwrap_or_else(|_| default_config())
            } else {
                default_config()
            };

            app.manage(AppState {
                config: Mutex::new(config.clone()),
                config_path,
            });

            let menu = setup_menu(app.handle(), &config)?;
            let _tray = setup_tray(app.handle(), &config);
            app.set_menu(menu.clone())?;

            // --- 起動時の完全同期ロジック ---
            let tray_enabled = config.tray_mode;
            
            // 1. ウィンドウの可視性設定
            let _ = utils::apply_window_visibility(app.handle().clone(), !tray_enabled);

            // 2. メニューアイテムのチェック状態を同期
            if let Some(item) = menu.get("tray_mode").and_then(|i| i.as_check_menuitem().cloned()) {
                let _ = item.set_checked(tray_enabled);
            }
            
            // 3. トレイモードでない場合は確実に表示（tauri.confのvisible:falseを考慮）
            if !tray_enabled {
                let _ = window.show();
            }

            app.on_menu_event(move |app_handle, event| {
                handle_menu_event(app_handle, event);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config, set_always_on_top, get_restore_previous_state, get_bsdiff_max_file_size,
            get_auto_base_generation_threshold, get_language_text, get_i18n, set_language,
            get_config_dir, backup_or_diff, apply_multi_diff, copy_backup_file,
            archive_backup_file, dir_exists, restore_backup, get_file_size,
            select_any_file, select_backup_folder, open_directory, toggle_compact_mode,
            write_text_file, read_text_file, get_backup_list,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
