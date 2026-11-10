use crate::app::commands::get_language_text;
use crate::app::state::AppState;
use crate::core::types::AppConfig;
use tauri::WebviewWindow;
use tauri::{AppHandle, Manager};
use tauri::{
    LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position as TauriPosition, Size,
};

/// 位置とサイズを1回で変更する。
/// Windows は SetWindowPos 1発なので、途中状態が画面に出ない。
pub fn set_bounds_atomic(
    window: &WebviewWindow,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> tauri::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};
        let hwnd = HWND(window.hwnd()?.0 as *mut _);
        unsafe {
            let _ = SetWindowPos(hwnd, None, x, y, w, h, SWP_NOACTIVATE | SWP_NOZORDER);
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        window.set_size(PhysicalSize::new(w as u32, h as u32))?;
        window.set_position(PhysicalPosition::new(x, y))?;
        Ok(())
    }
}

/// トレイポップアップを作業領域の右下に、指定の論理サイズで配置する(1回で)
pub fn place_tray_popup(window: &WebviewWindow, width: f64, height: f64) -> tauri::Result<()> {
    let monitor = match window.current_monitor()? {
        Some(m) => m,
        None => return Ok(()),
    };
    let work = monitor.work_area();
    let scale = window.scale_factor()?;
    let margin = (8.0 * scale).round() as i32;
    let w = (width * scale).round() as i32;
    let h = (height * scale).round() as i32;
    let x = work.position.x + work.size.width as i32 - w - margin;
    let y = work.position.y + work.size.height as i32 - h - margin;
    set_bounds_atomic(window, x, y, w, h)
}

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
    if !is_tray_mode {
        // 復帰処理は今まで通り
        window.set_decorations(true)?;
        window.set_always_on_top(false)?;
        window.set_skip_taskbar(false)?;
        let size = Size::Logical(LogicalSize::new(420.0, 340.0));
        window.set_resizable(true)?;
        window.set_min_size(None::<Size>)?;
        window.set_max_size(None::<Size>)?;
        window.set_size(size)?;
        window.set_min_size(Some(size))?;
        window.set_max_size(Some(size))?;
        window.center()?;
        return Ok(());
    }

    // --- トレイへ ---
    // すでにトレイ向けなら、サイズ・制約に一切触らない
    let already_tray = !window.is_decorated().unwrap_or(true);
    let size_ok = match (window.inner_size(), window.scale_factor()) {
        (Ok(sz), Ok(scale)) => {
            let h = sz.height as f64 / scale;
            let w = sz.width as f64 / scale;
            (h - 78.0).abs() < 6.0 && (w - 320.0).abs() < 6.0
        }
        _ => false,
    };

    if already_tray && size_ok {
        let _ = window.set_always_on_top(true);
        let _ = window.set_skip_taskbar(true);
        return Ok(()); // ★ ここで終了（set_size しない）
    }

    // 初回だけ構成
    window.set_resizable(true)?;
    window.set_min_size(None::<Size>)?;
    window.set_max_size(None::<Size>)?;
    if window.is_decorated().unwrap_or(true) {
        window.set_decorations(false)?;
    }
    window.set_always_on_top(true)?;
    window.set_skip_taskbar(true)?;

    let size = Size::Logical(LogicalSize::new(320.0, 78.0));
    window.set_size(size)?;
    window.set_min_size(Some(Size::Logical(LogicalSize::new(320.0, 60.0))))?;
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
    let h = height.clamp(60.0, 700.0);
    let new_h = (h * scale).round() as i32;
    let old_o = window.outer_size()?;
    let old_p = window.outer_position()?;
    if new_h == old_o.height as i32 {
        return Ok(()); // 同じ高さなら何もしない
    }

    let was_tray = !window.is_decorated().unwrap_or(true);
    if was_tray {
        // 下辺固定で上に伸ばす。min/max には触らず、1回で変更。
        let bottom = old_p.y + old_o.height as i32;
        set_bounds_atomic(window, old_p.x, bottom - new_h, old_o.width as i32, new_h)?;
    } else {
        let width = old_o.width as f64 / scale;
        let new_size = Size::Logical(LogicalSize::new(width, h));
        window.set_min_size(None::<Size>)?;
        window.set_max_size(None::<Size>)?;
        window.set_size(new_size)?;
        window.set_min_size(Some(new_size))?;
        window.set_max_size(Some(new_size))?;
    }
    Ok(())
}
#[cfg(target_os = "windows")]
pub fn disable_dwm_transitions(window: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED};

    let Ok(hwnd_raw) = window.hwnd() else {
        return;
    };

    // tauri の HWND → windows の HWND
    // 型が合わないときは `HWND(hwnd_raw.0)` や `HWND(hwnd_raw.0 as _)` に変える
    let hwnd = HWND(hwnd_raw.0 as *mut _);

    // TRUE = 1（BOOL 型は使わない）
    let disable: i32 = 1;
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &disable as *const i32 as *const _,
            std::mem::size_of_val(&disable) as u32,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn disable_dwm_transitions(_window: &tauri::WebviewWindow) {}
