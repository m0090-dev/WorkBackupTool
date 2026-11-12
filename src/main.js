import "./style.css";
import "./app.css";
import "./App.js";
import { renderNotifyPopup } from "./notify-popup.js";

// 完了の通知ポップアップ用ウィンドウ(Rust が作る小さなウィンドウ)は、
// アプリ本体を動かさず、通知だけを描画する(App.js 側でも初期化を止めている)
if (window.__WBT_NOTIFY__) {
  renderNotifyPopup(window.__WBT_NOTIFY__);
}
