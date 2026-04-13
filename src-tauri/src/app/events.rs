use crate::app::commands::*;
use crate::app::menu::*;
use crate::app::state::AppState;
use crate::app::utils;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

#[cfg(desktop)]
use crate::app::utils::create_tray_menu;
#[cfg(desktop)]
use tauri::menu::MenuEvent;
#[cfg(desktop)]
use tauri_plugin_positioner::{Position, WindowExt};

#[cfg(desktop)]
pub fn handle_window_event(window: &tauri::WebviewWindow, event: &tauri::WindowEvent) {
    match event {
        tauri::WindowEvent::Focused(false) => {
            let app_handle = window.app_handle();
            let state = app_handle.state::<AppState>();

            // ロックを最小限のスコープで取得
            let is_tray_mode = {
                let cfg = state.config.lock().unwrap();
                cfg.tray_mode
            };

            if is_tray_mode {
                let _ = window.hide();
            }
        }
        _ => {}
    }
}

#[cfg(desktop)]
pub fn handle_menu_event(app: &tauri::AppHandle, event: MenuEvent) {
    let state = app.state::<AppState>();
    let id = event.id.as_ref();
    //println!("--- Menu Event: '{}' ---", id);

    match id {
        // --- 1. バックアップモード切替 (フル / アーカイブ / 差分) トレイモード用 ---
        "mode_full" | "mode_arc" | "mode_diff" => {
            let mode_str = match id {
                "mode_full" => "copy",
                "mode_arc" => "archive",
                _ => "diff",
            };

            // Configを更新して保存
            let config = {
                let mut cfg = state.config.lock().unwrap();
                cfg.tray_backup_mode = mode_str.to_string();
                cfg.clone()
            };
            let _ = state.save();

            // JS側に同期を依頼
            let _ = app.emit("tray-mode-change", mode_str);

            // 【重要】トレイを再生成せず、トレイの「メニュー」だけを更新する
            if let Some(tray) = app.tray_by_id("main-tray") {
                // setup_menu ではなく、トレイ用のメニュー生成ロジックを呼ぶ
                // ※setup_tray 内部で作っている Menu を取得する関数があればベスト
                // ここでは再度メニューオブジェクトを構築してセットします
                if let Ok(new_menu) = create_tray_menu(app, &config) {
                    let _ = tray.set_menu(Some(new_menu));
                }
            }
        }

        // --- 2. トレイモード切替 ---
        "tray_mode" => {
            let next_is_tray_enabled = {
                let mut cfg = state.config.lock().unwrap();
                cfg.tray_mode = !cfg.tray_mode;
                cfg.tray_mode
            };

            let _ = utils::apply_window_visibility(app.clone(), !next_is_tray_enabled);

            // メインメニューのチェック状態を同期
            if let Some(item) = app
                .menu()
                .and_then(|m| m.get(id))
                .and_then(|i| i.as_check_menuitem().cloned())
            {
                let _ = item.set_checked(next_is_tray_enabled);
            }
            let _ = state.save();
        }

        // --- 3. ウィンドウ表示 (トレイから復帰時) ---
        "show_window" => {
            let config = {
                let mut cfg = state.config.lock().unwrap();
                cfg.tray_mode = false;
                cfg.compact_mode = false;
                cfg.clone()
            };
            let _ = state.save();

            let _ = utils::apply_window_visibility(app.clone(), true);
            let _ = app.emit("compact-mode-event", false);

            if let Some(window) = app.get_webview_window("main") {
                let _ = utils::apply_tray_popup_mode(&window, false);
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }

            // メインメニュー全体をリフレッシュ
            if let Ok(new_menu) = setup_menu(app, &config) {
                let _ = app.set_menu(new_menu);
            }
        }
        "show_compact" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = utils::apply_tray_popup_mode(&window, true);
                let _ = app.emit("compact-mode-event", true);
                #[cfg(target_os = "windows")]
                let _ = window.as_ref().window().move_window(Position::TrayCenter);
                #[cfg(not(target_os = "windows"))]
                let _ = window.as_ref().window().move_window(Position::TopRight);
                let _ = window.show();
                let _ = window.set_focus();
            }
        }

        // --- 4. 最前面表示 / コンパクトモード / 状態復元 ---
        "always_on_top"
        | "compact_mode"
        | "restore_state"
        | "use_same_dir_for_temp"
        | "rebuild_cache_on_startup"
        | "show_memo_after_backup" => {
            // スコープ（波括弧）を使って、ロックの寿命を短くします
            let (next_val, id_clone) = {
                let mut cfg = state.config.lock().unwrap();
                let val = match id {
                    "always_on_top" => {
                        cfg.always_on_top = !cfg.always_on_top;
                        cfg.always_on_top
                    }
                    "compact_mode" => {
                        cfg.compact_mode = !cfg.compact_mode;
                        cfg.compact_mode
                    }
                    "restore_state" => {
                        cfg.restore_previous_state = !cfg.restore_previous_state;
                        cfg.restore_previous_state
                    }
                    "use_same_dir_for_temp" => {
                        cfg.use_same_dir_for_temp = !cfg.use_same_dir_for_temp;
                        cfg.use_same_dir_for_temp
                    }
                    "rebuild_cache_on_startup" => {
                        cfg.rebuild_cache_on_startup = !cfg.rebuild_cache_on_startup;
                        cfg.rebuild_cache_on_startup
                    }
                    "show_memo_after_backup" => {
                        cfg.show_memo_after_backup = !cfg.show_memo_after_backup;
                        cfg.show_memo_after_backup
                    }
                    _ => false,
                };
                (val, id.to_string())
            };
            let _ = state.save();

            // 以降の処理
            if id_clone == "always_on_top" {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_always_on_top(next_val);
                }
            } else if id_clone == "compact_mode" {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = utils::apply_compact_mode(&window, next_val);
                    let _ = app.emit("compact-mode-event", next_val);
                }
            }

            if let Some(item) = app
                .menu()
                .and_then(|m| m.get(&id_clone))
                .and_then(|i| i.as_check_menuitem().cloned())
            {
                let _ = item.set_checked(next_val);
            }
        }

        // --- 5. アクション系 ---
        "execute" => {
            let _ = app.emit("tray-execute-clicked", ());
        }
        "change_work" => {
            let _ = app.emit("tray-change-work-clicked", ());
        }
        "change_backup" => {
            let _ = app.emit("tray-change-backup-clicked", ());
        }
        // 詳細設定
        "advanced_settings" => {
            let _ = app.emit("open-advanced-settings", ());
        }

        "quit" => {
            app.exit(0);
        }
        "lang_en" | "lang_ja" => {
            let lang_code = if id == "lang_en" { "en" } else { "ja" };

            // 1. まずConfigを更新・保存
            let (config, is_always_on_top) = {
                let mut cfg = state.config.lock().unwrap();
                cfg.language = lang_code.to_string();
                (cfg.clone(), cfg.always_on_top) // always_on_top の状態も一緒に取得しておく
            };
            let _ = state.save();

            // 2. メニュー全体を再生成してセットし直す
            if let Ok(new_menu) = setup_menu(app, &config) {
                let _ = app.set_menu(new_menu);
            }

            // 3. 通知（Always on Top を考慮）
            let t = |key: &str| -> String {
                get_language_text(state.clone(), key).unwrap_or_else(|_| key.to_string())
            };

            if let Some(window) = app.get_webview_window("main") {
                // ダイアログ表示前に一時解除
                if is_always_on_top {
                    let _ = window.set_always_on_top(false);
                }

                let window_clone = window.clone();
                window
                    .dialog()
                    .message(&t("restartRequired"))
                    .show(move |_| {
                        // ダイアログを閉じたら復元
                        if is_always_on_top {
                            let _ = window_clone.set_always_on_top(true);
                        }
                    });
            } else {
                // 万が一ウィンドウがない場合は app 経由で出す（フォールバック）
                let _ = app.dialog().message(&t("restartRequired")).show(|_| {});
            }
        }

        "about" => {
            let t = |key: &str| -> String {
                get_language_text(state.clone(), key).unwrap_or_else(|_| key.to_string())
            };

            if let Some(window) = app.get_webview_window("main") {
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

        _ => {}
    }
}
