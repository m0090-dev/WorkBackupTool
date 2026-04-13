use crate::app::commands::get_language_text;
use crate::app::state::AppState;
use crate::core::types::AppConfig;
use chrono::Local;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use tar::Builder;
use tauri::WebviewWindow;
use tauri::{AppHandle, Manager};
use tauri::{LogicalSize, Size, Window};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::ShellExt;
use zip::write::SimpleFileOptions;
use zip::ZipArchive;
use zip::ZipWriter;
use zip::{AesMode, CompressionMethod};

pub fn apply_compact_mode(window: &WebviewWindow, is_compact: bool) -> tauri::Result<()> {
    // 1. まず「何でもあり」の状態にする (制約の完全解除)
    #[cfg(desktop)]
    {
        window.set_resizable(true)?;
        window.set_min_size(None::<Size>)?;
        window.set_max_size(None::<Size>)?;
    }
    let (width, height, title) = if is_compact {
        (300.0, 260.0, "WorkBackupTool (Compact mode)")
    } else {
        (640.0, 430.0, "WorkBackupTool")
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
    // 制約解除
    window.set_resizable(true)?;
    window.set_min_size(None::<Size>)?;
    window.set_max_size(None::<Size>)?;

    if is_tray_mode {
        // --- トレイポップアップ用 ---
        window.set_decorations(false)?; // 枠なし
        window.set_always_on_top(true)?; // 最前面
        window.set_skip_taskbar(true)?; // タスクバーに出さない

        let size = Size::Logical(LogicalSize::new(300.0, 210.0));
        window.set_size(size)?;
        window.set_min_size(Some(size))?;
        window.set_max_size(Some(size))?;
    } else {
        // --- 通常モード（復帰）用 ---
        window.set_decorations(true)?; // 枠あり
        window.set_always_on_top(false)?;
        window.set_skip_taskbar(false)?;

        let size = Size::Logical(LogicalSize::new(640.0, 430.0));
        window.set_size(size)?;
        window.set_min_size(Some(size))?;
        window.set_max_size(Some(size))?;
        window.center()?; // 中央に戻す
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
