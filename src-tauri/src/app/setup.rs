use crate::app::commands::*;
use crate::app::menu::*;
use crate::app::state::AppState;
use crate::app::tray::*;
use crate::app::utils;
use crate::core::config::loader::*;
use std::fs;
use std::sync::Mutex;
use tauri::App;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

#[cfg(desktop)]
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::app::events;

pub fn init(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app.get_webview_window("main").unwrap();
    let initial_size = tauri::Size::Logical(tauri::LogicalSize::new(640.0, 450.0));

    #[cfg(desktop)]
    {
        let _ = window.set_min_size(Some(initial_size));
        let _ = window.set_max_size(Some(initial_size));
    }
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
        i18n: default_i18n(),
    });

    #[cfg(desktop)]
    {
        let menu = setup_menu(app.handle(), &config)?;
        let _tray = setup_tray(app.handle(), &config);
        app.set_menu(menu.clone())?;
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_focus();
        }
        // --- 起動時の完全同期ロジック ---
        let tray_enabled = config.tray_mode;
        let always_on_top_enabled = config.always_on_top;

        // ウィンドウの可視性設定を復元
        let _ = utils::apply_window_visibility(app.handle().clone(), !tray_enabled);

        // ウィンドウの最前面設定を復元
        let _ = utils::apply_window_always_on_top(app.handle().clone(), always_on_top_enabled);

        // 2. メニューアイテムのチェック状態を同期
        #[cfg(desktop)]
        {
            if let Some(item) = menu
                .get("tray_mode")
                .and_then(|i| i.as_check_menuitem().cloned())
            {
                let _ = item.set_checked(tray_enabled);
            }
        }

        // 3. トレイモードでない場合は確実に表示（tauri.confのvisible:falseを考慮）
        if !tray_enabled {
            let _ = window.show();
        }

        app.on_menu_event(move |app_handle, event| {
            events::handle_menu_event(app_handle, event);
        });
        let window_for_event = window.clone();

        window.on_window_event(move |event| {
            events::handle_window_event(&window_for_event, event);
        });

        // 1. ショートカットの定義
        let quit_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyQ);
        let about_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyA);

        // 2. プラグインをハンドラ付きで登録
        app.handle().plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app_handle, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if shortcut == &quit_shortcut {
                            app_handle.exit(0);
                        } else if shortcut == &about_shortcut {
                            let state = app_handle.state::<AppState>();
                            let t = |key: &str| -> String {
                                get_language_text(state.clone(), key)
                                    .unwrap_or_else(|_| key.to_string())
                            };

                            if let Some(window) = app_handle.get_webview_window("main") {
                                // --- 修正ポイント ---
                                // 現在の状態を保存
                                let is_always_on_top = state.config.lock().unwrap().always_on_top;

                                // ダイアログを出す前に一時的に解除（これでダイアログが上に来れる）
                                if is_always_on_top {
                                    let _ = window.set_always_on_top(false);
                                }

                                let window_clone = window.clone();
                                window
                                    .dialog()
                                    .message(t("aboutText"))
                                    .title(t("about"))
                                    .show(move |_| {
                                        // ダイアログが閉じられたら元の設定に戻す
                                        if is_always_on_top {
                                            let _ = window_clone.set_always_on_top(true);
                                        }
                                    });
                            }
                        }
                    }
                })
                .build(),
        )?;

        // 3. ショートカットを OS に登録
        app.global_shortcut().register(quit_shortcut)?;
        app.global_shortcut().register(about_shortcut)?;
    }

    Ok(())
}
