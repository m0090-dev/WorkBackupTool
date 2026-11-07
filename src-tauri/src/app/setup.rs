use crate::app::menu::*;
use crate::app::state::AppState;
use crate::app::tray::*;
use crate::app::utils;
use crate::core::config::loader::*;
use std::fs;
use std::sync::Mutex;
use tauri::App;
use tauri::Manager;

use crate::app::events;

pub fn init(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app.get_webview_window("main").unwrap();
    let initial_size = tauri::Size::Logical(tauri::LogicalSize::new(420.0, 520.0));

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

        // メニューアイテムのチェック状態を同期
        if let Some(item) = menu
            .get("tray_mode")
            .and_then(|i| i.as_check_menuitem().cloned())
        {
            let _ = item.set_checked(tray_enabled);
        }

        // トレイモードでない場合は確実に表示（tauri.confのvisible:falseを考慮）
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

        // ※ 以前ここで Ctrl+Q / Ctrl+A を OS 全体のグローバルショートカットとして登録していたが、
        //   他アプリの操作(全選択など)を奪うため廃止した。
        //   終了・ソフト情報はメニューのアクセラレータ(Ctrl+Q / F1)で呼び出す。
        //   グローバルショートカット機能は別途、設定可能な形で追加する予定。
    }

    Ok(())
}
