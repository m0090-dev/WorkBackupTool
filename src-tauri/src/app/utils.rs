use crate::app::commands::get_language_text;
use crate::app::state::AppState;
use crate::core::types::AppConfig;
use tauri::WebviewWindow;
use tauri::{AppHandle, Manager};
use tauri::{LogicalSize, Size};

pub fn apply_compact_mode(window: &WebviewWindow, is_compact: bool) -> tauri::Result<()> {
    // 1. まず「何でもあり」の状態にする (制約の完全解除)
    #[cfg(desktop)]
    {
        window.set_resizable(true)?;
        window.set_min_size(None::<Size>)?;
        window.set_max_size(None::<Size>)?;
    }
    let (width, height, title) = if is_compact {
        (320.0, 70.0, "WorkBackupTool (Compact mode)")
    } else {
        (420.0, 340.0, "WorkBackupTool")
    };

    let new_size = Size::Logical(LogicalSize::new(width, height));

    #[cfg(desktop)]
    {
        // 2. タイトルを変更
        window.set_title(title)?;

        // 3. サイズを変更
        // ここで一旦サイズが変わるはずです
        window.set_size(new_size)?;

        // 4. (重要) サイズを固定したい場合は、サイズ変更のあとに設定する
        // デバッグのため、もしこれでも動かないなら下の2行を消してみてください
        window.set_min_size(Some(new_size))?;
        window.set_max_size(Some(new_size))?;
    }
    Ok(())
}

pub fn apply_tray_popup_mode(window: &WebviewWindow, is_tray_mode: bool) -> tauri::Result<()> {
    // 制約をいったん外す
    window.set_resizable(true)?;
    window.set_min_size(None::<Size>)?;
    window.set_max_size(None::<Size>)?;

    if is_tray_mode {
        // --- トレイポップアップ用 ---
        // 順序: 枠なし → 属性 → サイズ（Windows で client 領域がずれにくい順）
        window.set_decorations(false)?;
        window.set_always_on_top(true)?;
        window.set_skip_taskbar(true)?;

        // 通常コンパクト 70 + ドラッグバー約 12〜14 → 実測に近い 82
        // ここで max まで固定しない（JS が一回だけ合わせられる余地を残す）
        let size = Size::Logical(LogicalSize::new(320.0, 82.0));
        window.set_size(size)?;
        // 下限だけ軽く固定。上限は JS の set_window_height に任せる
        window.set_min_size(Some(Size::Logical(LogicalSize::new(320.0, 60.0))))?;
        // set_max_size はしない（ガチ固定が余白・ちらつきの元）

        // Windows: decorations 変更直後の client 再計算対策でサイズを一度だけ再適用
        #[cfg(target_os = "windows")]
        {
            let _ = window.set_size(size);
        }
    } else {
        // --- 通常モード復帰 ---
        window.set_decorations(true)?;
        window.set_always_on_top(false)?;
        window.set_skip_taskbar(false)?;

        let size = Size::Logical(LogicalSize::new(420.0, 340.0));
        window.set_size(size)?;
        window.set_min_size(Some(size))?;
        window.set_max_size(Some(size))?;
        window.center()?;
    }
    Ok(())
}

pub fn apply_window_visibility(app: AppHandle, show: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if show {
            #[cfg(desktop)]
            {
                window.show().map_err(|e| e.to_string())?;
                window.unminimize().map_err(|e| e.to_string())?; // 最小化されていても戻す
                window.set_focus().map_err(|e| e.to_string())?;
            }
        } else {
            #[cfg(desktop)]
            {
                window.hide().map_err(|e| e.to_string())?;
            }
        }
    } else {
        return Err("Main window not found".into());
    }
    Ok(())
}

pub fn apply_window_always_on_top(app: AppHandle, flag: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(desktop)]
        {
            let _ = window.set_always_on_top(flag);
        }
    } else {
        return Err("Main window not found".into());
    }
    Ok(())
}

// 共通化：トレイメニューだけを生成するヘルパー関数
#[cfg(desktop)]
pub fn create_tray_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    config: &AppConfig,
) -> tauri::Result<tauri::menu::Menu<R>> {
    let state = app.state::<AppState>();
    let t = |key: &str| get_language_text(state.clone(), key).unwrap_or_else(|_| key.to_string());

    let mode_full = tauri::menu::CheckMenuItemBuilder::with_id("mode_full", t("modeFull"))
        .checked(config.tray_backup_mode == "copy")
        .build(app)?;
    let mode_arc = tauri::menu::CheckMenuItemBuilder::with_id("mode_arc", t("modeArc"))
        .checked(config.tray_backup_mode == "archive")
        .build(app)?;
    let mode_diff = tauri::menu::CheckMenuItemBuilder::with_id("mode_diff", t("modeDiff"))
        .checked(config.tray_backup_mode == "diff")
        .build(app)?;
    let backup_mode_menu = tauri::menu::SubmenuBuilder::new(app, t("backupMode"))
        .item(&mode_full)
        .item(&mode_arc)
        .item(&mode_diff)
        .build()?;

    tauri::menu::MenuBuilder::new(app)
        .item(&tauri::menu::MenuItemBuilder::with_id("show_window", t("showWindow")).build(app)?)
        .item(&tauri::menu::MenuItemBuilder::with_id("show_compact", t("compactMode")).build(app)?)
        .separator()
        .item(&backup_mode_menu)
        .item(&tauri::menu::MenuItemBuilder::with_id("execute", t("executeBtn")).build(app)?)
        .item(&tauri::menu::MenuItemBuilder::with_id("change_work", t("workFileBtn")).build(app)?)
        .item(
            &tauri::menu::MenuItemBuilder::with_id("change_backup", t("backupDirBtn"))
                .build(app)?,
        )
        .separator()
        .item(&tauri::menu::MenuItemBuilder::with_id("quit", t("quit")).build(app)?)
        .build()
}

/// コンパクト/トレイ表示の高さを中身に合わせる（幅はそのまま）。
/// JS が測った値を渡す。ここでのみ min/max を同じ値で固定する。
pub fn apply_window_height(window: &WebviewWindow, height: f64) -> tauri::Result<()> {
    let scale = window.scale_factor()?;
    let width = window.inner_size()?.width as f64 / scale;
    let h = height.clamp(60.0, 700.0);
    let new_size = Size::Logical(LogicalSize::new(width, h));

    #[cfg(desktop)]
    {
        // 制約解除 → サイズ → 固定（1回のセットで完結させる）
        window.set_min_size(None::<Size>)?;
        window.set_max_size(None::<Size>)?;
        window.set_size(new_size)?;
        window.set_min_size(Some(new_size))?;
        window.set_max_size(Some(new_size))?;
    }
    Ok(())
}
