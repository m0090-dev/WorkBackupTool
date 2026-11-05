import { GetBackupList, ReadTextFile } from "../tauri_exports";
import { i18n, getActiveTab, formatSize } from "../state";
import { parseNoteContent } from "../memo.js";
import { isBackupExecuting } from "./progress.js";

export async function UpdateHistory() {
  const tab = getActiveTab();
  const list = document.getElementById("diff-history-list");
  const searchInput = document.getElementById("history-search");
  const searchTerm = (tab?.searchQuery || "").toLowerCase().trim();
  const clearBtn = document.getElementById("search-clear-btn");
  const executeBtn = document.getElementById("execute-backup-btn");
  const compactExecuteBtn = document.getElementById("compact-execute-btn");
  if (!list || !i18n) return;
  if (searchInput) {
    if (document.activeElement !== searchInput) {
      searchInput.value = tab.searchQuery || "";
    }
  }
  if (clearBtn) {
    if (searchTerm.length > 0) {
      clearBtn.classList.add("visible");
    } else {
      clearBtn.classList.remove("visible");
    }
  }
  if (!tab?.workFile) {
    list.innerHTML = `<div class="info-msg">${i18n.selectFileFirst}</div>`;
    return;
  }

  try {
    let data = await GetBackupList(tab.workFile, tab.backupDir);
    if (!data || data.length === 0) {
      list.innerHTML = `<div class="info-msg">${i18n.noHistory}</div>`;
      return;
    }

    data.sort((a, b) => b.fileName.localeCompare(a.fileName));

    const latestGenNumber = Math.max(
      ...data.map((item) => item.generation || 0),
    );

    let activeDirPath = tab.selectedTargetDir;
    if (!activeDirPath) {
      const first = data[0];
      activeDirPath = first.filePath.replace(/[\\/][^\\/]+$/, "");
    }

    const highlight = (text, term) => {
      if (!term) return text;
      const regex = new RegExp(`(${term})`, "gi");
      return text.replace(
        regex,
        `<mark style="background-color: #ffeb3b; color: #000; padding: 0 2px; border-radius: 2px;">$1</mark>`,
      );
    };

    let isTargetArchivedGeneration = false;
    const itemsHtml = await Promise.all(
      data.map(async (item) => {
        const raw = await ReadTextFile(item.filePath + ".note").catch(() => "");
        const { text: note, meta: noteMeta } = parseNoteContent(raw);

        if (searchTerm) {
          const inFileName = item.fileName.toLowerCase().includes(searchTerm);
          const inNote = note.toLowerCase().includes(searchTerm);
          if (!inFileName && !inNote) return null;
        }

        const isDiffFile = item.fileName.toLowerCase().endsWith(".diff");
        const isArchive = !isDiffFile && item.generation === 0;

        const itemDir =
          item.filePath.substring(0, item.filePath.lastIndexOf("/")) ||
          item.filePath.substring(0, item.filePath.lastIndexOf("\\"));

        let statusHtml = "";
        let genBadge = "";
        const mark = noteMeta?.mark ?? 0;
        const markBadge =
          mark > 0
            ? (() => {
                const markColors = { 1: "#e6a817", 2: "#db741f", 3: "#c0392b" };
                const markLabels = {
                  1: i18n?.priorityLow || "Low",
                  2: i18n?.priorityMid || "Mid",
                  3: i18n?.priorityHigh || "High",
                };
                return `<span style="font-size:10px; color:#fff; background:${markColors[mark]}; padding:1px 4px; border-radius:3px; margin-left:3px;">${markLabels[mark]}</span>`;
              })()
            : "";

        if (isArchive) {
          const archiveText = i18n.fullArchive || " Full Archive";
          statusHtml = `<div style="color:#2f8f5b; font-weight:bold;">${archiveText}</div>`;
          genBadge = `<span style="font-size:10px; color:#fff; background:#2f8f5b; padding:1px 4px; border-radius:3px; margin-left:5px;">Archive</span>`;
        } else {
          const currentGen = item.generation || 1;
          const isTarget = itemDir === activeDirPath;
          if (isTarget && item.isArchived) {
            isTargetArchivedGeneration = true;
          }
          const subLabel = item.isArchived
            ? ` <span style="font-size:9px; opacity:0.9;">(Archive)</span>`
            : isTarget
              ? ` <span style="font-size:9px; opacity:0.9;">(Target)</span>`
              : "";
          let statusColor = isTarget ? "#2f8f5b" : "#3B5998";
          let statusIcon = isTarget ? "✅" : "";
          let statusText = isTarget
            ? i18n.compatible || "書き込み先 (Active)"
            : i18n.genMismatch || "別世代 (クリックで切替)";

          if (item.isArchived) {
            statusColor = "#666";
            statusText =
              i18n.archived_generation_extracting ||
              "世代アーカイブ (一時展開中)";
            statusIcon = "📦";
          }

          const genLabel = i18n.generationLabel || "Gen";
          const currentLabel = isTarget
            ? ` <span style="font-size:9px; opacity:0.9;">(Target)</span>`
            : "";
          const badgeStyle = `font-size:10px; color:#fff; background:${statusColor}; padding:1px 4px; border-radius:3px; margin-left:5px; ${isTarget ? "outline: 2px solid #2f8f5b; outline-offset: 1px;" : ""} cursor:pointer;`;

          statusHtml = `<div style="color:${statusColor}; font-weight:bold;">${statusIcon} ${statusText}</div>
                        <div style="font-size:11px; color:#666;">${genLabel}: ${currentGen} ${isTarget ? "★" : ""}</div>`;

          genBadge = `<span class="gen-selector-badge" data-dir="${itemDir}" style="${badgeStyle}">${genLabel}.${currentGen}${currentLabel}</span>`;
        }

        const markLabels = {
          1: i18n?.priorityLow || "Low",
          2: i18n?.priorityMid || "Mid",
          3: i18n?.priorityHigh || "High",
        };
        const markText =
          mark > 0
            ? `<br><strong>${i18n?.priorityLabel || "Priority"}:</strong> ${markLabels[mark]}`
            : "";

        const popupContent = `${statusHtml}<hr style="border:0; border-top:1px solid #eee; margin:5px 0;"><strong>Path:</strong> ${item.filePath}${markText}${note ? `<br><hr style="border:0; border-top:1px dashed #ccc; margin:5px 0;"><strong>${i18n.backupMemo}:</strong> ${note}` : ""}`;

        const displayedFileName = highlight(item.fileName, searchTerm);
        const displayedNote = highlight(note, searchTerm);

        return `<div class="diff-item" style="${itemDir === activeDirPath ? "border-left: 4px solid #2f8f5b; background: #f0fff4;" : ""}">
          <div style="display:flex; align-items:center; width:100%;">
            <label style="display:flex; align-items:center; cursor:pointer; flex:1; min-width:0;">
              <input type="checkbox" class="diff-checkbox" value="${item.filePath}" style="margin-right:10px;">
              <div style="display:flex; flex-direction:column; flex:1; min-width:0;">
                <span class="diff-name" data-hover-content="${encodeURIComponent(popupContent)}" style="font-weight:bold; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                  ${displayedFileName} ${genBadge} ${markBadge} <span style="font-size:10px; color:#3B5998;">(${formatSize(item.fileSize)})</span>
                </span>
                <span style="font-size:10px; color:#888;">${item.timestamp}</span>
                ${note ? `<div style="font-size:10px; color:#2f8f5b; font-style:italic; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;"> ${displayedNote}</div>` : ""}
              </div>
            </label>
            <button class="note-btn" data-path="${item.filePath}" style="background:none; border:none; cursor:pointer; font-size:14px; padding:4px;">📝</button>
          </div>
        </div>`;
      }),
    );
    list.innerHTML = itemsHtml.filter((html) => html !== null).join("");

    if (executeBtn) {
      if (isTargetArchivedGeneration) {
        executeBtn.setAttribute("disabled", "");
      } else if (!isBackupExecuting()) {
        executeBtn.removeAttribute("disabled");
      }
    }
    if (compactExecuteBtn) {
      if (isTargetArchivedGeneration) {
        compactExecuteBtn.setAttribute("disabled", "");
      } else if (!isBackupExecuting()) {
        compactExecuteBtn.removeAttribute("disabled");
      }
    }
    setupHistoryPopups();
  } catch (err) {
    console.error(err);
    list.innerHTML = `<div class="info-msg" style="color:red;">Error: ${err.message || "loading history"}</div>`;
  }
}

function setupHistoryPopups() {
  const tooltip =
    document.getElementById("history-tooltip") || createTooltipElement();
  const targets = document.querySelectorAll(".diff-name");

  targets.forEach((target) => {
    target.onmouseenter = (e) => {
      const content = decodeURIComponent(
        target.getAttribute("data-hover-content"),
      );
      tooltip.innerHTML = content;
      tooltip.classList.remove("hidden");

      const rect = target.getBoundingClientRect();
      tooltip.style.left = `${rect.left}px`;
      tooltip.style.top = `${rect.bottom + 5}px`;

      const tooltipHeight = tooltip.offsetHeight;
      const windowHeight = window.innerHeight;

      let topPosition = rect.bottom + 2;
      if (topPosition + tooltipHeight > windowHeight) {
        topPosition = rect.top - tooltipHeight - 2;
      }

      tooltip.style.top = `${topPosition}px`;
    };

    target.onmouseleave = () => {
      tooltip.classList.add("hidden");
    };
  });
}

function createTooltipElement() {
  const el = document.createElement("div");
  el.id = "history-tooltip";
  el.className = "history-tooltip hidden";
  document.body.appendChild(el);
  return el;
}
