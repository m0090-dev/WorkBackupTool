import { recentFiles } from "../state";
import { escapeHtml } from "./html.js";

export function renderRecentFiles() {
  // 通常画面の作業対象メニューと、コンパクト画面のパネルの両方に出す
  const html =
    recentFiles.length === 0
      ? `<span class="recent-empty">No recent files</span>`
      : recentFiles
          .map((path) => {
            const fileName = path.split(/[\\/]/).pop();
            const p = escapeHtml(path);
            return `<div class="recent-item" title="${p}" data-path="${p}" data-close>${escapeHtml(fileName)}</div>`;
          })
          .join("");
  ["recent-list", "compact-recent-list"].forEach((id) => {
    const list = document.getElementById(id);
    if (list) list.innerHTML = html;
  });
}
