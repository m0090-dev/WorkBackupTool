use crate::app::commands::get_language_text;
use crate::app::state::AppState;
use crate::core::types::AppConfig;
use tauri::WebviewWindow;
use tauri::{AppHandle, Manager};
use tauri::{LogicalPosition, LogicalSize, PhysicalPosition, Position as TauriPosition, Size};

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

/// トレイポップアップを「作業領域（タスクバー除外）の下端」に貼る。
/// 高さ変更後も同じ関数を呼べば、下辺固定のまま上方向に伸びる。
pub fn position_tray_popup(window: &WebviewWindow) -> tauri::Result<()> {
    let monitor = match window.current_monitor()? {
        Some(m) => m,
        None => return Ok(()),
    };
    let work = monitor.work_area(); // タスクバーを除いた領域（Physical）
    let scale = window.scale_factor()?;
    let outer = window.outer_size()?; // Physical

    let margin = (8.0 * scale).round() as i32;
    let w = outer.width as i32;
    let h = outer.height as i32;

    // 横: 作業領域の右寄せ（トレイはだいたい右下）
    let x = work.position.x + work.size.width as i32 - w - margin;
    // 縦: ウィンドウ下辺 = 作業領域下辺 - margin → タスクバーに被らない
    let y = work.position.y + work.size.height as i32 - h - margin;

    window.set_position(TauriPosition::Physical(PhysicalPosition::new(x, y)))?;
    Ok(())
}

pub fn apply_tray_popup_mode(window: &WebviewWindow, is_tray_mode: bool) -> tauri::Result<()> {
    window.set_resizable(true)?;
    window.set_min_size(None::<Size>)?;
    window.set_max_size(None::<Size>)?;

    if window.is_decorated().unwrap_or(true) {
        window.set_decorations(false)?;
    }
    if is_tray_mode {
        window.set_always_on_top(true)?;
        window.set_skip_taskbar(true)?;

        // ドラッグバー込みの実サイズに近い値。大きめにしない。
        let size = Size::Logical(LogicalSize::new(320.0, 78.0));
        window.set_size(size)?;
        // 下限のみ。上限は apply_window_height で初めて固定する
        window.set_min_size(Some(Size::Logical(LogicalSize::new(320.0, 60.0))))?;

        #[cfg(target_os = "windows")]
        {
            let _ = window.set_size(size);
        }
    } else {
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

pub fn apply_window_height(window: &WebviewWindow, height: f64) -> tauri::Result<()> {
    let scale = window.scale_factor()?;
    let width = window.inner_size()?.width as f64 / scale;
    let h = height.clamp(60.0, 700.0);
    let new_size = Size::Logical(LogicalSize::new(width, h));

    #[cfg(desktop)]
    {
        // 現在の下辺を維持するため、変更前の outer 位置・高さを取る
        let was_tray = !window.is_decorated().unwrap_or(true);
        let old_outer = window.outer_size().ok();
        let old_pos = window.outer_position().ok();

        window.set_min_size(None::<Size>)?;
        window.set_max_size(None::<Size>)?;
        window.set_size(new_size)?;
        window.set_min_size(Some(new_size))?;
        window.set_max_size(Some(new_size))?;

        // トレイ枠なし時: 下辺固定で上に伸ばす（タスクバーに食い込まない）
        if was_tray {
            if let (Some(old_o), Some(old_p)) = (old_outer, old_pos) {
                let new_outer = window.outer_size()?;
                let dy = new_outer.height as i32 - old_o.height as i32;
                // 高くなった分だけ上へずらす
                let _ = window.set_position(TauriPosition::Physical(PhysicalPosition::new(
                    old_p.x,
                    old_p.y - dy,
                )));
            } else {
                let _ = position_tray_popup(window);
            }
        }
    }
    Ok(())
}







#[cfg(target_os = "windows")]
pub fn disable_dwm_transitions(window: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED,
    };

    // Tauri 2 の Windows 専用 API（raw_window_handle 不要）
    let Ok(hwnd_raw) = window.hwnd() else {
        return;
    };

    // tauri の HWND から windows クレートの HWND へ
    let hwnd = HWND(hwnd_raw.0 as *mut _);

    let disable = BOOL(1); // TRUE = トランジション無効
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &disable as *const _ as *const _,
            std::mem::size_of_val(&disable) as u32,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn disable_dwm_transitions(_window: &tauri::WebviewWindow) {}
