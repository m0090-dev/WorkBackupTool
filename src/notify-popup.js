/**
 * 完了の通知ポップアップ(Rust が作る小さなウィンドウ)の描画。
 * 表示内容は Rust が初期化スクリプトで window.__WBT_NOTIFY__ に入れて渡してくる。
 *   { kind: "success" | "failure", title: string, message: string }
 * このウィンドウではアプリ本体(App.js の初期化)は動かさない。
 */
export function renderNotifyPopup(data) {
  const kind = data?.kind === "failure" ? "failure" : "success";

  document.documentElement.classList.add("notify-popup");
  document.body.replaceChildren();

  const root = document.createElement("div");
  root.id = "notify-root";
  root.className = kind;

  const title = document.createElement("div");
  title.className = "notify-title";
  title.textContent = data?.title ?? "";

  const message = document.createElement("div");
  message.className = "notify-message";
  message.textContent = data?.message ?? "";

  root.append(title, message);
  document.body.appendChild(root);
}
