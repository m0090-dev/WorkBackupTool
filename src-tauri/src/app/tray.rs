use crate::app::commands::get_language_text;
use crate::app::state::AppState;
use crate::app::types::AppConfig;
use tauri::menu::MenuBuilder;
use tauri::menu::MenuItemBuilder;
use tauri::menu::PredefinedMenuItem;
use tauri::tray::TrayIcon;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri::{AppHandle, Manager, Runtime, State};

pub fn setup_tray<R: Runtime>(
    app: &AppHandle<R>,
    config: &AppConfig,
) -> tauri::Result<TrayIcon<R>> {
    // 1. Stateから多言語テキスト取得用ヘルパーを用意
    let state: State<'_, AppState> = app.state();
    let t = |key: &str| -> String {
        get_language_text(state.clone(), key).unwrap_or_else(|_| key.to_string())
    };

    // 2. メニューアイテムの構築
    // ウィンドウ表示
    let show_window = MenuItemBuilder::with_id("show_window", t("showWindow")).build(app)?;

    // アクション系 (既存i18nキーを使用)
    let execute = MenuItemBuilder::with_id("execute", t("executeBtn")).build(app)?;
    let change_work = MenuItemBuilder::with_id("change_work", t("workFileBtn")).build(app)?;
    let change_backup = MenuItemBuilder::with_id("change_backup", t("backupDirBtn")).build(app)?;

    // 区切り線
    let separator = PredefinedMenuItem::separator(app)?;

    // 終了アイテム
    let quit = MenuItemBuilder::with_id("quit", t("quit")).build(app)?;

    // 3. トレイメニューの構築
    // 順序: ウィンドウ表示 -> (線) -> 実行 -> ファイル選択 -> 保存先選択 -> (線) -> 終了
    let tray_menu = MenuBuilder::new(app)
        .items(&[
            &show_window,
            &separator,
            &execute,
            &change_work,
            &change_backup,
            &separator,
            &quit,
        ])
        .build()?;

    // 4. トレイアイコンの構築
    TrayIconBuilder::with_id("main-tray")
        .tooltip(t("trayTitle"))
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&tray_menu)
        .menu_on_left_click(true) // 左クリックでメニューを表示
        .build(app)
}
