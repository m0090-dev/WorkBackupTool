use crate::app::notify::{notify_completion, NotifyKind};

/// 実行結果(成功/失敗)を通知基盤に渡す。
/// 通知のオン/オフ・方法・表示時間・失敗時のみ、は設定に従って Rust 側で判断する。
/// 戻り値: 通知を出したら true。false のときは、呼び出し側が画面内のメッセージなどで代わりに知らせる。
#[tauri::command]
pub async fn notify_result(
    app: tauri::AppHandle,
    ok: bool,
    message: String,
) -> Result<bool, String> {
    let kind = if ok {
        NotifyKind::Success
    } else {
        NotifyKind::Failure
    };
    Ok(notify_completion(&app, kind, &message))
}
