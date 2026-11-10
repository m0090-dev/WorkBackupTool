use crate::app::commands::get_language_text; // 既存のコマンドをインポート
use crate::app::state::AppState;
use crate::core::types::AppConfig;
use tauri::State;
use tauri::{AppHandle, Manager, Runtime};

#[cfg(desktop)]
use tauri::menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItem, SubmenuBuilder};

/// ネイティブメニュー(mock2.html のメニュー構成「ファイル / 設定 / ヘルプ」に合わせる)
///
/// ファイル: 古い世代を圧縮 / 終了
/// 設定    : 最前面 / コンパクト / トレイ | 言語 / 詳細設定(起動時の動作などはここに集約)
/// ヘルプ  : ソフト情報
///
/// ※ メニュー項目のIDは events.rs の handle_menu_event と対応している。
#[cfg(desktop)]
pub fn setup_menu<R: Runtime>(app: &AppHandle<R>, config: &AppConfig) -> tauri::Result<Menu<R>> {
    // get_language_text コマンドが必要とする State を取得します
    let state: State<'_, AppState> = app.state();

    // エラー時はキー名をそのまま返す
    let t = |key: &str| -> String {
        get_language_text(state.clone(), key).unwrap_or_else(|_| key.to_string())
    };

    // --- メニューアイテムの構築 ---

    // チェック状態は config から取得
    let always_on_top = CheckMenuItemBuilder::with_id("always_on_top", t("alwaysOnTop"))
        .checked(config.always_on_top)
        .build(app)?;

    let compact_mode = CheckMenuItemBuilder::with_id("compact_mode", t("compactMode"))
        .checked(config.compact_mode)
        .build(app)?;

    let tray_mode = CheckMenuItemBuilder::with_id("tray_mode", t("trayMode"))
        .checked(false)
        .build(app)?;

    let lang_en = CheckMenuItemBuilder::with_id("lang_en", t("english"))
        .checked(config.language == "en")
        .build(app)?;
    let lang_ja = CheckMenuItemBuilder::with_id("lang_ja", t("japanese"))
        .checked(config.language == "ja")
        .build(app)?;
    let advanced_settings_title = MenuItem::with_id(
        app,
        "advanced_settings",
        t("advancedSettingsTitle"),
        true,
        None::<&str>,
    )?;
    let generation_archive = MenuItem::with_id(
        app,
        "generation_archive",
        t("generationArchive"),
        true,
        None::<&str>,
    )?;

    // ショートカットはウィンドウがアクティブな間だけ有効なメニューのアクセラレータ。
    // (OS全体に効くグローバルショートカットは、他アプリの Ctrl+A 等を奪うため使わない)
    let quit = MenuItem::with_id(app, "quit", t("quit"), true, Some("Ctrl+Q"))?;
    // Ctrl+A は入力欄の「全選択」と衝突するため F1 にする
    let about = MenuItem::with_id(app, "about", t("about"), true, Some("F1"))?;

    // --- サブメニューの組み立て ---

    let file_menu = SubmenuBuilder::new(app, t("menuFile"))
        .item(&generation_archive)
        .separator()
        .item(&quit)
        .build()?;

    let language_menu = SubmenuBuilder::new(app, t("language"))
        .item(&lang_en)
        .item(&lang_ja)
        .build()?;

    let settings_menu = SubmenuBuilder::new(app, t("settings"))
        .item(&always_on_top)
        .item(&compact_mode)
        .item(&tray_mode)
        .separator()
        .item(&language_menu)
        .item(&advanced_settings_title)
        .build()?;

    let help_menu = SubmenuBuilder::new(app, t("menuHelp"))
        .item(&about)
        .build()?;

    // --- ルートメニューの生成 ---

    MenuBuilder::new(app)
        .items(&[&file_menu, &settings_menu, &help_menu])
        .build()
}
