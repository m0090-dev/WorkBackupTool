import { recentFiles } from "../state";

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
      return `<div class="recent-item" title="${path}" data-path="${path}"><i></i> ${fileName}</div>`;
    })
    .join("");
}
