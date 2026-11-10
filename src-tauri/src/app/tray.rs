use crate::app::commands::get_language_text;
use crate::app::state::AppState;
use crate::app::utils;
use crate::core::types::AppConfig;
use tauri::{AppHandle, Emitter, Manager, State, Wry};

#[cfg(desktop)]
use tauri_plugin_positioner::{Position, WindowExt};

#[cfg(desktop)]
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

#[cfg(desktop)]
pub fn setup_tray(app: &AppHandle, config: &AppConfig) -> tauri::Result<TrayIcon<Wry>> {
    // 1. Stateから多言語テキスト取得用ヘルパーを用意
    let state: State<'_, AppState> = app.state();
    let t = |key: &str| -> String {
        get_language_text(state.clone(), key).unwrap_or_else(|_| key.to_string())
    };

    // 2. トレイメニューは utils::create_tray_menu に一本化
    //    (モード変更後の再生成と同じ処理にして、項目やチェック判定のずれを防ぐ)
    let tray_menu = utils::create_tray_menu(app, config)?;

    // 3. トレイアイコンの構築
    TrayIconBuilder::with_id("main-tray")
        .tooltip(t("trayTitle"))
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&tray_menu)
        .menu_on_left_click(false) // 左クリックでメニューを出さない設定
        .on_tray_icon_event(|tray, event| {
            // Positionerプラグインにイベントを渡す（座標計算に必要）
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

            match event {
                // 左クリックが離された瞬間
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    let app_handle = tray.app_handle();
                    if let Some(window) = app_handle.get_webview_window("main") {
                        // 1. 先に JS に通知（compact-mode / frameless のクラスを先に付ける）
                        let _ = app_handle.emit("compact-mode-event", true);
                        let _ = app_handle.emit("compact-frameless", true);

                        // 2. 枠なし + 初期サイズ（max 固定なし）
                        let _ = utils::apply_tray_popup_mode(&window, true);

                        // 3. トレイ真上へ
                        let _ = window.as_ref().window().move_window(Position::TrayCenter);

                        // 4. 表示
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        })
        .build(app)
}
