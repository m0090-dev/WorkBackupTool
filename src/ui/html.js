/** innerHTML に差し込む文字列のエスケープ(パス・ファイル名・メモ用) */
export function escapeHtml(text) {
  return String(text ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** 正規表現の特殊文字をエスケープ(検索語のハイライト用) */
export function escapeRegExp(text) {
  return String(text ?? "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
