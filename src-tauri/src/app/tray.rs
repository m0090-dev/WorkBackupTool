use tauri::{
  menu::{Menu, MenuItem},
  tray::TrayIconBuilder,
};
use tauri::{AppHandle,State,Runtime,Manager};
use crate::app::commands::get_language_text;
use crate::app::state::AppState;
use crate::app::types::AppConfig;
use tauri::menu::MenuItemBuilder;
use tauri::tray::TrayIcon;
use tauri::menu::MenuBuilder;



pub fn setup_tray<R: Runtime>(app: &AppHandle<R>, config: &AppConfig) -> tauri::Result<TrayIcon<R>> {
    // 1. Stateから多言語テキスト取得用ヘルパーを用意
    let state: State<'_, AppState> = app.state();
    let t = |key: &str| -> String {
        get_language_text(state.clone(), key).unwrap_or_else(|_| key.to_string())
    };

    // 2. トレイ専用のメニューアイテムを作成 (Quitのみ)
    let quit = MenuItemBuilder::with_id("quit", t("quit")).build(app)?;

    // 3. トレイメニューの構築
    let tray_menu = MenuBuilder::new(app)
        .items(&[&quit])
        .build()?;

    // 4. トレイアイコンの構築
    TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&tray_menu)
        .menu_on_left_click(true) // 左クリックですぐメニューを出す設定
        .build(app)
}
