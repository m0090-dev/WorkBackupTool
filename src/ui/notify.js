import { NotifyResult } from "../tauri_exports";

/**
 * 実行結果(成功/失敗)を通知基盤に渡す。
 * 通知のオン/オフ・方法・表示時間・失敗時のみ は Rust 側が設定に従って判断する。
 * @returns {Promise<boolean>} 通知を出したら true。
 *   false(通知オフなど)のときは、呼び出し側が画面内のメッセージで代わりに知らせる。
 */
export async function notifyResult(ok, message) {
  try {
    return !!(await NotifyResult(!!ok, String(message ?? "")));
  } catch (err) {
    console.error("notifyResult failed:", err);
    return false;
  }
}
