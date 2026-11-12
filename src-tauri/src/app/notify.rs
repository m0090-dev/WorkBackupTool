//! 通知基盤。
//!
//! アプリからの「お知らせ」(完了の通知、将来の取り忘れの知らせなど)は、すべてここを通す。
//! - `notify_completion` : 設定(オン/オフ・方法・表示時間・失敗時のみ)に従って完了を知らせる
//! - `show_popup` / `play_sound` : 設定が別の通知(取り忘れの知らせなど)から直接使える部品
//!
//! ポップアップは、メイン画面とは別の小さな枠なしウィンドウ。
//! メイン画面がトレイで隠れていても表示でき、指定の秒数で自動的に閉じる。
//! 中身の描画は src/notify-popup.js が行う(Rust は表示内容を初期化スクリプトで渡すだけ)。

use crate::app::state::AppState;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyKind {
    Success,
    Failure,
}

impl NotifyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NotifyKind::Success => "success",
            NotifyKind::Failure => "failure",
        }
    }
}

/// 完了の通知。設定に従って、ポップアップ/音を出す。
/// 戻り値: 何か通知を出したら true(通知オフ・「失敗時のみ」で成功だった場合などは false)
pub fn notify_completion(app: &AppHandle, kind: NotifyKind, message: &str) -> bool {
    let (enabled, only_failure, style, duration) = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().unwrap();
        (
            cfg.notify_enabled,
            cfg.notify_only_on_failure,
            cfg.notify_style.clone(),
            cfg.notify_duration_sec,
        )
    };

    if !enabled {
        return false;
    }
    if kind == NotifyKind::Success && only_failure {
        return false;
    }

    let want_popup = matches!(style.as_str(), "popup" | "both");
    let want_sound = matches!(style.as_str(), "sound" | "both");
    let mut notified = false;

    if want_sound {
        play_sound(kind);
        notified = true;
    }
    if want_popup {
        let key = match kind {
            NotifyKind::Success => "notifyTitleSuccess",
            NotifyKind::Failure => "notifyTitleFailure",
        };
        let title = translate(app, key);
        match show_popup(app, kind, &title, message, duration) {
            Ok(()) => notified = true,
            Err(e) => eprintln!("[notify] popup failed: {}", e),
        }
    }
    notified
}

fn translate(app: &AppHandle, key: &str) -> String {
    let state = app.state::<AppState>();
    let text = state.translate(key).unwrap_or_else(|_| key.to_string());
    text
}

// ---------------------------------------------------------------- ポップアップ

#[cfg(desktop)]
static POPUP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// 直近に出したポップアップのラベル(次のポップアップを出すときに閉じる)
#[cfg(desktop)]
static LAST_POPUP: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// 長いメッセージ(エラー文など)を、ポップアップに収まる長さに切る
#[cfg(desktop)]
fn clamp_message(message: &str) -> String {
    const MAX_CHARS: usize = 160;
    let mut out: String = message.chars().take(MAX_CHARS).collect();
    if message.chars().count() > MAX_CHARS {
        out.push('…');
    }
    out
}

/// ポップアップの左上位置(論理px)。メイン画面のあるモニタの作業領域の右下に置く
#[cfg(desktop)]
fn popup_position(app: &AppHandle, width: f64, height: f64, margin: f64) -> (f64, f64) {
    let monitor = app
        .get_webview_window("main")
        .and_then(|w| w.current_monitor().ok().flatten());
    match monitor {
        Some(m) => {
            let scale = m.scale_factor();
            let work = m.work_area();
            let right = (work.position.x as f64 + work.size.width as f64) / scale;
            let bottom = (work.position.y as f64 + work.size.height as f64) / scale;
            (right - width - margin, bottom - height - margin)
        }
        None => (100.0, 100.0),
    }
}

/// 小さなポップアップを出す。duration_sec 秒(1〜5)で自動的に閉じる。
#[cfg(desktop)]
pub fn show_popup(
    app: &AppHandle,
    kind: NotifyKind,
    title: &str,
    message: &str,
    duration_sec: u32,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    const WIDTH: f64 = 300.0;
    const HEIGHT: f64 = 72.0;
    const MARGIN: f64 = 12.0;

    // 前のポップアップが残っていれば閉じる(重ねて溜めない)
    let previous = LAST_POPUP.lock().unwrap().take();
    if let Some(label) = previous {
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.destroy();
        }
    }

    let seq = POPUP_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let label = format!("notify-{}", seq);

    let payload = serde_json::json!({
        "kind": kind.as_str(),
        "title": title,
        "message": clamp_message(message),
    });
    let script = format!("window.__WBT_NOTIFY__ = {};", payload);

    let (x, y) = popup_position(app, WIDTH, HEIGHT, MARGIN);
    WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title("WorkBackupTool")
        .inner_size(WIDTH, HEIGHT)
        .position(x, y)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .initialization_script(script.as_str())
        .build()
        .map_err(|e| e.to_string())?;

    *LAST_POPUP.lock().unwrap() = Some(label.clone());

    let secs = duration_sec.clamp(1, 5) as u64;
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(secs)).await;
        if let Some(w) = app_handle.get_webview_window(&label) {
            let _ = w.destroy();
        }
    });
    Ok(())
}

#[cfg(not(desktop))]
pub fn show_popup(
    _app: &AppHandle,
    _kind: NotifyKind,
    _title: &str,
    _message: &str,
    _duration_sec: u32,
) -> Result<(), String> {
    Err("popup is not supported on this platform".to_string())
}

// ---------------------------------------------------------------- 音

/// 音を鳴らす(OS 標準の通知音。成功と失敗で音を変える)
pub fn play_sound(kind: NotifyKind) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{MB_ICONASTERISK, MB_ICONHAND};
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBeep(uType: u32) -> i32;
        }
        let style = match kind {
            NotifyKind::Success => MB_ICONASTERISK,
            NotifyKind::Failure => MB_ICONHAND,
        };
        unsafe {
            MessageBeep(MB_ICONASTERISK.0);
        }
    }
    #[cfg(target_os = "macos")]
    {
        let file = match kind {
            NotifyKind::Success => "/System/Library/Sounds/Glass.aiff",
            NotifyKind::Failure => "/System/Library/Sounds/Basso.aiff",
        };
        let _ = spawn_detached("afplay", &[file]);
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let (event, file) = match kind {
            NotifyKind::Success => (
                "complete",
                "/usr/share/sounds/freedesktop/stereo/complete.oga",
            ),
            NotifyKind::Failure => (
                "dialog-error",
                "/usr/share/sounds/freedesktop/stereo/dialog-error.oga",
            ),
        };
        if !spawn_detached("canberra-gtk-play", &["-i", event]) {
            let _ = spawn_detached("paplay", &[file]);
        }
    }
}

/// 外部コマンドを待たずに起動する(終了は別スレッドで回収する)。起動できたら true
#[cfg(unix)]
fn spawn_detached(program: &str, args: &[&str]) -> bool {
    use std::process::{Command, Stdio};
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            true
        }
        Err(_) => false,
    }
}
