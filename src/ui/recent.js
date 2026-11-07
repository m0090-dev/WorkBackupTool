import { recentFiles } from "../state";
import { escapeHtml } from "./html.js";

export function renderRecentFiles() {
  const list = document.getElementById("recent-list");
  if (!list) return;

  if (recentFiles.length === 0) {
    list.innerHTML = `<span class="recent-empty">No recent files</span>`;
    return;
  }
  list.innerHTML = recentFiles
    .map((path) => {
      const fileName = path.split(/[\\/]/).pop();
      const p = escapeHtml(path);
      return `<div class="recent-item" title="${p}" data-path="${p}" data-close>${escapeHtml(fileName)}</div>`;
    })
    .join("");
}
