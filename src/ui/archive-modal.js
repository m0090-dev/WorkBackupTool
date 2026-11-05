import { GetGenerationFolders } from "../tauri_exports";
import { i18n, getActiveTab } from "../state";
import { showFloatingError } from "./messages.js";

export async function showArchiveModal() {
  const tab = getActiveTab();
  const modal = document.getElementById("archive-modal");
  const listContainer = document.getElementById("archive-gen-list");

  if (!tab || !modal || !listContainer || !i18n) return;

  document.getElementById("title-gen-archive").textContent =
    i18n.generationArchive || "Generation Archive";
  document.getElementById("label-archive-desc").textContent =
    i18n.archiveWarningText ||
    "Original folders will be deleted. Restore is still possible.";
  document.getElementById("label-archive-select-all").textContent =
    i18n.selectAllBtn || "Select All";
  document.getElementById("archive-cancel-btn").textContent =
    i18n.cancel || "Cancel";
  document.getElementById("archive-execute-btn").textContent =
    i18n.executeBtn || "Execute";

  try {
    const archiveCandidates = await GetGenerationFolders(
      tab.workFile,
      tab.backupDir,
    );

    archiveCandidates.sort((a, b) => b.generation - a.generation);

    if (archiveCandidates.length === 0) {
      listContainer.innerHTML = `
        <div style="font-size:11px; color:#888; text-align:center; padding:15px;">
          ${i18n.noArchiveCandidates || "No folders available to archive."}
        </div>`;
      document.getElementById("archive-execute-btn").disabled = true;
    } else {
      listContainer.innerHTML = archiveCandidates
        .map(
          (c) => `
        <label class="archive-item">
          <input type="checkbox" class="archive-gen-check" 
                 value="${c.filePath}" 
                 data-gen="${c.generation}">
          <div style="display:flex; flex-direction:column; text-align:left;">
            <span style="font-weight:bold; color:#fff;">${i18n.generationLabel}.${c.generation}</span>
            <span style="font-size:10px; color:#bbb;">${c.timestamp}</span>
          </div>
        </label>
      `,
        )
        .join("");
      document.getElementById("archive-execute-btn").disabled = false;
    }

    const selectAll = document.getElementById("archive-select-all-check");
    if (selectAll) selectAll.checked = false;

    modal.classList.remove("hidden");
  } catch (err) {
    console.error("Failed to load archive candidates:", err);
    showFloatingError(
      i18n.errorLoadingHistory || "Failed to load generation folders",
    );
  }
}
