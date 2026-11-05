// UI描画・メッセージ系（通常版）
export function showFloatingMessage(text) {
  const msgArea = document.getElementById("message-area");
  if (!msgArea) return;
  msgArea.classList.remove("error");
  msgArea.textContent = text;
  msgArea.classList.remove("hidden");
  setTimeout(() => msgArea.classList.add("hidden"), 3000);
}

// エラー版
export function showFloatingError(text) {
  const msgArea = document.getElementById("message-area");
  if (!msgArea) return;
  msgArea.classList.add("error");
  msgArea.textContent = text;
  msgArea.classList.remove("hidden");
  setTimeout(() => {
    msgArea.classList.add("hidden");
    setTimeout(() => {
      if (msgArea.classList.contains("hidden")) {
        msgArea.classList.remove("error");
      }
    }, 500);
  }, 3000);
}
