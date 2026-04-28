import {
  GetBackupList,
  GetFileSize,
  WriteTextFile,
  ReadTextFile,
  GetConfigDir,
  GetGenerationFolders,
  UpdateConfigValue,
  UpdateSessionTabValue,
  GetConfig,
  FileExists,
  DirExists,
} from "./tauri_exports";

import {
  i18n,
  tabs,
  recentFiles,
  getActiveTab,
  formatSize,
  saveCurrentSession,
  addToRecentFiles,
} from "./state";

import { showMemoDialog, parseNoteContent } from "./memo.js";

import { switchTab, removeTab, reorderTabs } from "./actions";

import { settingsSchema, categoryLabels } from "./settings-schema.js";

let isExecuting = false;

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

//TODO: Drag and drop関係はtauri v2が修整されたら正常動作するはず
export function renderTabs() {
  const list = document.getElementById("tabs-list");
  if (!list) return;

  list.innerHTML = "";

  const clearGlobals = () => {
    const existingMenu = document.querySelector(".tab-context-menu");
    if (existingMenu) existingMenu.remove();
    const existingTooltips = document.querySelectorAll(".tab-tooltip");
    existingTooltips.forEach((t) => t.remove());
  };
  clearGlobals();

  tabs.forEach((tab, index) => {
    const el = document.createElement("div");
    el.className = `tab-item ${tab.active ? "active" : ""}`;

    const fileName = tab.workFile
      ? tab.workFile.split(/[\\/]/).pop()
      : i18n?.selectedWorkFile || "No file selected";
    el.textContent = fileName;

    setupTabTooltip(el, tab);

    el.draggable = true;
    el.dataset.id = tab.id;
    el.ondragstart = (e) => {
      if (el._removeTooltip) el._removeTooltip();
      el.classList.add("dragging");
      e.dataTransfer.setData("text/plain", tab.id);
    };
    el.ondragover = (e) => {
      e.preventDefault();
      e.stopPropagation();
      el.classList.add("drag-over");
    };
    el.ondragleave = () => el.classList.remove("drag-over");
    el.ondragend = () => {
      el.classList.remove("dragging");
      list
        .querySelectorAll(".tab-item")
        .forEach((i) => i.classList.remove("drag-over"));
    };
    el.ondrop = async (e) => {
      e.preventDefault();
      const dId = e.dataTransfer.getData("text/plain");
      if (dId && dId !== el.dataset.id) await reorderTabs(dId, el.dataset.id);
    };

    el.onclick = async () => {
      if (el._removeTooltip) el._removeTooltip();
      await switchTab(tab.id);
    };

    el.oncontextmenu = (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (el._removeTooltip) el._removeTooltip();

      const existingMenu = document.querySelector(".tab-context-menu");
      if (existingMenu) existingMenu.remove();

      const menuItems = [];

      if (index > 0) {
        const item = document.createElement("div");
        item.className = "tab-menu-item";
        item.innerHTML = `<span>${i18n.tabMenuMoveLeft}</span><span class="tab-menu-shortcut">◀</span>`;
        item.onclick = async (ev) => {
          ev.stopPropagation();
          await reorderTabs(tab.id, tabs[index - 1].id);
          menu.remove();
        };
        menuItems.push(item);
      }

      if (index < tabs.length - 1) {
        const item = document.createElement("div");
        item.className = "tab-menu-item";
        item.innerHTML = `<span>${i18n.tabMenuMoveRight}</span><span class="tab-menu-shortcut">▶</span>`;
        item.onclick = async (ev) => {
          ev.stopPropagation();
          await reorderTabs(tab.id, tabs[index + 1].id);
          menu.remove();
        };
        menuItems.push(item);
      }

      if (tabs.length > 1) {
        const sep = document.createElement("div");
        sep.className = "tab-menu-separator";
        menuItems.push(sep);

        const del = document.createElement("div");
        del.className = "tab-menu-item danger";
        del.innerHTML = `<span>${i18n.tabMenuClose}</span><span class="tab-menu-shortcut">×</span>`;
        del.onclick = async (ev) => {
          ev.stopPropagation();
          await removeTab(tab.id);
          menu.remove();
        };
        menuItems.push(del);
      }

      if (menuItems.length === 0) return;

      const menu = document.createElement("div");
      menu.className = "tab-context-menu";
      menuItems.forEach((item) => menu.appendChild(item));

      document.body.appendChild(menu);

      const menuRect = menu.getBoundingClientRect();
      let left = e.clientX;
      let top = e.clientY;
      if (left + menuRect.width > window.innerWidth) left -= menuRect.width;
      if (top + menuRect.height > window.innerHeight) top -= menuRect.height;

      menu.style.left = `${left}px`;
      menu.style.top = `${top}px`;

      const closeMenu = (ev) => {
        if (!menu.contains(ev.target)) {
          menu.remove();
          document.removeEventListener("mousedown", closeMenu);
        }
      };
      setTimeout(() => document.addEventListener("mousedown", closeMenu), 50);
    };

    list.appendChild(el);
  });
}

function setupTabTooltip(el, tab) {
  if (!el || !tab || !i18n) return;

  let tooltip = null;

  const removeTooltip = () => {
    if (tooltip) {
      tooltip.remove();
      tooltip = null;
    }
  };

  el.addEventListener("mouseenter", () => {
    if (document.querySelector(".tab-context-menu")) return;

    removeTooltip();

    const fileName = tab.workFile
      ? tab.workFile.split(/[\\/]/).pop()
      : i18n?.selectedWorkFile || "No file selected";
    const workPath =
      tab.workFile || i18n?.selectedWorkFile || "No file selected";
    const backupPath =
      tab.backupDir || i18n?.selectedBackupDir || "Default location";
    const mode = tab.backupMode || "diff";

    tooltip = document.createElement("div");
    tooltip.className = "tab-tooltip";
    tooltip.innerHTML = `
      <div style="margin-bottom:4px;"><b>${fileName}</b></div>
      <div style="font-size:10px; opacity:0.8; line-height: 1.4;">
        <b>${i18n.labelWorkFile}</b> <code>${workPath}</code><br>
        <b>${i18n.labelLocation}</b> <code>${backupPath}</code><br>
        <b>${i18n.backupMode}:</b> <span style="color:#ffd700">${mode.toUpperCase()}</span>
      </div>
    `;

    document.body.appendChild(tooltip);

    const rect = el.getBoundingClientRect();
    tooltip.style.left = `${rect.left}px`;
    tooltip.style.top = `${rect.bottom + 5}px`;
  });

  el.addEventListener("mouseleave", removeTooltip);
  el.addEventListener("mousedown", removeTooltip);

  el._removeTooltip = removeTooltip;
}

function setupPathTooltip(el, fullPath) {
  if (!el || !fullPath) return;

  if (el._pathTooltipSetup) {
    el._tooltipPath = fullPath;
    return;
  }

  el._pathTooltipSetup = true;
  el._tooltipPath = fullPath;

  el.addEventListener("mouseenter", () => {
    const tooltip = document.createElement("div");
    tooltip.className = "tab-tooltip";
    const displayPath = el._tooltipPath.replace(/\n/g, "<br>");
    tooltip.innerHTML = `<code>${displayPath}</code>`;

    document.body.appendChild(tooltip);
    const rect = el.getBoundingClientRect();
    tooltip.style.left = `${rect.left}px`;
    tooltip.style.top = `${rect.bottom + 5}px`;
    el._tooltip = tooltip;
  });

  el.addEventListener("mouseleave", () => {
    if (el._tooltip) {
      el._tooltip.remove();
      el._tooltip = null;
    }
  });
}

// 全体のUI更新
export async function UpdateDisplay() {
  let workTargetExists = false;
  let dirExists = true;
  const tab = getActiveTab();
  if (!i18n || !tab) return;

  if (tab.workFile) {
    const isFile = await FileExists(tab.workFile);
    const isDir = await DirExists(tab.workFile);
    workTargetExists = isFile || isDir;
  } else {
    workTargetExists = true;
  }
  if (tab.backupDir) {
    dirExists = await DirExists(tab.backupDir);
  }
  const tabSelect = document.getElementById("compact-tab-select");
  if (tabSelect) {
    tabSelect.innerHTML = tabs
      .map((t) => {
        const fileName = t.workFile
          ? t.workFile.split(/[\\/]/).pop()
          : "No File";
        return `<option value="${t.id}" ${t.active ? "selected" : ""}>${fileName}</option>`;
      })
      .join("");
    tabSelect.value = tab.id;
  }

  const fileEl = document.getElementById("selected-workfile");
  const dirEl = document.getElementById("selected-backupdir");
  if (fileEl) {
    const baseName = tab.workFile
      ? tab.workFile.split(/[\\/]/).pop()
      : i18n.selectedWorkFile;
    const sizeText = tab.workFile ? ` [${formatSize(tab.workFileSize)}]` : "";
    if (!workTargetExists && tab.workFile) {
      fileEl.innerHTML = `<span style="color: #ff4d4d; font-weight: bold;">${baseName} (Not Found)</span>`;
    } else {
      fileEl.textContent = `${baseName}${sizeText}`;
      fileEl.style.color = "";
    }
  }
  if (dirEl) {
    const baseName = tab.backupDir
      ? tab.backupDir.split(/[\\/]/).pop()
      : i18n.selectedBackupDir;
    if (!dirExists && tab.backupDir) {
      dirEl.innerHTML = `<span style="color: #ff4d4d; font-weight: bold;">${baseName} (Not Found)</span>`;
    } else {
      dirEl.textContent = baseName;
      dirEl.style.color = "";
    }
  }

  const radio = document.querySelector(
    `input[name="backupMode"][value="${tab.backupMode}"]`,
  );
  if (radio) radio.checked = true;
  const compactModeSel = document.getElementById("compact-mode-select");
  if (compactModeSel) compactModeSel.value = tab.backupMode;

  const locked = tab.isLocked || false;

  document.querySelectorAll('input[name="backupMode"]').forEach((r) => {
    r.disabled = locked;
  });
  const normalComp = document.getElementById("hdiff-compress");
  const compactComp = document.getElementById("compact-hdiff-compress");
  const compress = tab.compressMode || "zstd";
  const normalAlgo = document.getElementById("diff-algo");
  const algo = tab.diffAlgo || "hdiff";
  const normalArchive = document.getElementById("archive-format");
  const archiveFormat = tab.archiveFormat || "zip";
  const mode = tab.backupMode || "diff";

  if (normalAlgo) normalAlgo.value = algo;
  if (normalComp) {
    normalComp.value = compress;
    normalComp.disabled = locked;
  }
  if (compactComp) {
    compactComp.value = compress;
    compactComp.disabled = locked;
  }
  if (normalArchive) normalArchive.value = archiveFormat;
  const lockBtn = document.getElementById("lock-mode-btn");
  if (lockBtn) {
    lockBtn.textContent = locked ? "🔒" : "🔓";
    lockBtn.title = locked ? i18n.unlockMode : i18n.lockMode;
  }
  const isPass =
    mode === "archive" &&
    document.getElementById("archive-format")?.value === "zip-pass";
  const pwdArea = document.querySelector(".password-wrapper");
  if (pwdArea) {
    pwdArea.style.opacity = isPass ? "1" : "0.3";
    document.getElementById("archive-password").disabled = !isPass;
  }
  const cFileEl = document.getElementById("compact-selected-file");
  if (cFileEl)
    cFileEl.textContent = tab.workFile
      ? tab.workFile.split(/[\\/]/).pop()
      : i18n.selectedWorkFile || "No File Selected";
  const cSel = document.getElementById("compact-mode-select");
  if (cSel) cSel.disabled = locked;
  if (cSel && mode) cSel.value = mode;
}

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
            <button class="note-btn" data-path="${item.filePath}" style="background:none; border:none; cursor:pointer; font-size:14px; padding:4px;"></button>
          </div>
        </div>`;
      }),
    );
    list.innerHTML = itemsHtml.filter((html) => html !== null).join("");

    if (executeBtn) {
      if (isTargetArchivedGeneration) {
        executeBtn.setAttribute("disabled", "");
      } else if (!isExecuting) {
        executeBtn.removeAttribute("disabled");
      }
    }
    if (compactExecuteBtn) {
      if (isTargetArchivedGeneration) {
        compactExecuteBtn.setAttribute("disabled", "");
      } else if (!isExecuting) {
        compactExecuteBtn.removeAttribute("disabled");
      }
    }
    setupHistoryPopups();
  } catch (err) {
    console.error(err);
    list.innerHTML = `<div class="info-msg" style="color:red;">Error: ${err.message || "loading history"}</div>`;
  }
}

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

export async function showSettingsModal() {
  const modal = document.getElementById("settings-modal");
  if (!modal || !i18n) return;
  try {
    const config = await GetConfig();
    const lang = i18n.language || "ja";

    const categories = [
      ...new Set(settingsSchema.map((item) => item.category)),
    ];

    const modalContent = modal.querySelector(".modal-content");
    if (!modalContent) return;

    const tabsHtml = categories
      .map((cat, idx) => {
        const label = categoryLabels[cat]?.[lang] || cat;
        return `<button class="settings-tab-btn ${idx === 0 ? "active" : ""}" data-category="${cat}">${label}</button>`;
      })
      .join("");

    const pagesHtml = categories
      .map((cat, idx) => {
        const items = settingsSchema.filter((item) => item.category === cat);
        const itemsHtml = items
          .map((item) => {
            const labelText = i18n[item.label] || item.label;
            const hintText = item.hint ? i18n[item.hint] || "" : "";

            // tab スコープはアクティブタブから現在値を取得
            const currentValue =
              item.scope === "tab"
                ? (getActiveTab()?.[item.key] ?? [])
                : config[item.key];

            if (item.type === "boolean") {
              return `
            <div class="settings-item">
              <label style="display:flex; align-items:center; gap:8px; cursor:pointer;">
                <input type="checkbox" class="settings-input" data-key="${item.key}" ${currentValue ? "checked" : ""}>
                <span>${labelText}</span>
              </label>
              ${hintText ? `<div class="settings-hint">${hintText}</div>` : ""}
            </div>`;
            } else if (item.type === "number") {
              return `
            <div class="settings-item">
              <label>${labelText}</label>
              <input type="number" class="settings-input" data-key="${item.key}"
                value="${currentValue}"
                ${item.min !== null ? `min="${item.min}"` : ""}
                ${item.max !== null ? `max="${item.max}"` : ""}
                step="${item.step || 1}"
                style="width:80px; background:#252526; border:1px solid #444; color:#eee; padding:4px 8px; border-radius:4px;">
              ${hintText ? `<div class="settings-hint">${hintText}</div>` : ""}
            </div>`;
            } else if (item.type === "taglist") {
              // タグ（除外パターン）リストの表示。追加・削除をインタラクティブに行う。
              const tagsHtml = (Array.isArray(currentValue) ? currentValue : [])
                .map(
                  (tag) =>
                    `<span class="settings-tag" data-tag="${tag}">${tag}<button class="settings-tag-remove" data-key="${item.key}" data-tag="${tag}" title="Remove">×</button></span>`,
                )
                .join("");
              return `
            <div class="settings-item settings-taglist-item" data-key="${item.key}" data-scope="${item.scope || "config"}">
              <label>${labelText}</label>
              <div class="settings-tags" id="tags-${item.key}">${tagsHtml}</div>
              <div style="display:flex; gap:6px; margin-top:4px;">
                <input type="text" class="settings-tag-input" data-key="${item.key}"
                  placeholder="${i18n.hdiffIgnoreListPlaceholder || "e.g. *.tmp"}"
                  style="flex:1; background:#252526; border:1px solid #444; color:#eee; padding:4px 8px; border-radius:4px;">
                <button class="settings-tag-add modal-btn" data-key="${item.key}" style="padding:4px 10px;">${i18n.addBtn || "Add"}</button>
              </div>
              ${hintText ? `<div class="settings-hint">${hintText}</div>` : ""}
            </div>`;
            }
          })
          .join("");

        return `<div class="settings-page ${idx === 0 ? "" : "hidden"}" data-category="${cat}">${itemsHtml}</div>`;
      })
      .join("");

    modalContent.innerHTML = `
      <h3>${i18n.advancedSettingsTitle || "Advanced Settings"}</h3>
      <div class="settings-tabs">${tabsHtml}</div>
      <div class="settings-pages">${pagesHtml}</div>
      <div class="modal-buttons">
        <button id="settings-close-btn" class="modal-btn cancel">${i18n.closeBtn || "Close"}</button>
      </div>
    `;

    modalContent.querySelectorAll(".settings-tab-btn").forEach((btn) => {
      btn.onclick = (e) => {
        e.stopPropagation();
        modalContent
          .querySelectorAll(".settings-tab-btn")
          .forEach((b) => b.classList.remove("active"));
        modalContent
          .querySelectorAll(".settings-page")
          .forEach((p) => p.classList.add("hidden"));
        btn.classList.add("active");
        modalContent
          .querySelector(
            `.settings-page[data-category="${btn.dataset.category}"]`,
          )
          ?.classList.remove("hidden");
      };
    });

    // ---- [FIX #28] 設定変更ハンドラ ----
    // boolean / number を型に応じて正しく処理する
    modalContent.querySelectorAll(".settings-input").forEach((input) => {
      const handler = async () => {
        const key = input.dataset.key;
        const schema = settingsSchema.find((s) => s.key === key);
        if (!schema) return;

        let value;
        if (schema.type === "boolean") {
          // boolean はチェック状態をそのまま使う（数値バリデーションは不要）
          value = input.checked;
        } else {
          value = parseFloat(input.value);
          if (isNaN(value)) return;
          if (schema.min !== null && value < schema.min) return;
          if (schema.max !== null && value > schema.max) return;
        }

        await handleSettingChange(key, value);
      };
      input.addEventListener("change", handler);
    });

    // ---- taglist (tab スコープ) のイベントハンドラ ----
    // タグ追加: Add ボタン または Enter キー
    const setupTaglistHandlers = (container) => {
      container.querySelectorAll(".settings-tag-add").forEach((btn) => {
        btn.onclick = async () => {
          const key = btn.dataset.key;
          const textInput = container.querySelector(
            `.settings-tag-input[data-key="${key}"]`,
          );
          const pattern = textInput?.value.trim();
          if (!pattern) return;

          const tab = getActiveTab();
          if (!tab) return;
          const list = Array.isArray(tab[key]) ? [...tab[key]] : [];
          if (list.includes(pattern)) {
            textInput.value = "";
            return;
          }
          list.push(pattern);
          await handleTabSettingChange(tab, key, list);
          // タグDOM更新（再描画せずにインプレースで追加）
          const tagsContainer = container.querySelector(`#tags-${key}`);
          if (tagsContainer) {
            const span = document.createElement("span");
            span.className = "settings-tag";
            span.dataset.tag = pattern;
            span.innerHTML = `${pattern}<button class="settings-tag-remove" data-key="${key}" data-tag="${pattern}" title="Remove">×</button>`;
            tagsContainer.appendChild(span);
            span.querySelector(".settings-tag-remove").onclick = (e) =>
              removeTagHandler(e, container);
          }
          textInput.value = "";
        };
      });
      container.querySelectorAll(".settings-tag-input").forEach((input) => {
        input.addEventListener("keydown", (e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            container
              .querySelector(
                `.settings-tag-add[data-key="${input.dataset.key}"]`,
              )
              ?.click();
          }
        });
      });
      // 既存タグの削除ボタン
      container.querySelectorAll(".settings-tag-remove").forEach((btn) => {
        btn.onclick = (e) => removeTagHandler(e, container);
      });
    };

    const removeTagHandler = async (e, container) => {
      const btn = e.currentTarget;
      const key = btn.dataset.key;
      const tag = btn.dataset.tag;
      const tab = getActiveTab();
      if (!tab) return;
      const list = (Array.isArray(tab[key]) ? tab[key] : []).filter(
        (t) => t !== tag,
      );
      await handleTabSettingChange(tab, key, list);
      btn.closest(".settings-tag")?.remove();
    };

    setupTaglistHandlers(modalContent);

    modal.classList.remove("hidden");
  } catch (err) {
    console.error("Failed to load settings:", err);
    showFloatingError(i18n.errorLoadingHistory || "Failed to load settings");
  }
}

export async function handleSettingChange(key, value) {
  try {
    await UpdateConfigValue(key, value);
    showFloatingMessage(i18n.settingsSaved || "Settings saved");
  } catch (err) {
    console.error(`Failed to update ${key}:`, err);
    showFloatingError(i18n.settingsError || "Save failed");
  }
}

/// タブスコープの設定を session.json に保存し、インメモリのタブにも反映する
export async function handleTabSettingChange(tab, key, value) {
  try {
    tab[key] = value; // インメモリ更新
    const configDir = await GetConfigDir();
    const sessionPath = configDir + "/session.json";
    await UpdateSessionTabValue(sessionPath, tab.id, key, value);
    showFloatingMessage(i18n.settingsSaved || "Settings saved");
  } catch (err) {
    console.error(`Failed to update tab.${key}:`, err);
    showFloatingError(i18n.settingsError || "Save failed");
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

export function toggleProgress(show, text = "") {
  const displayMsg = text || (i18n ? i18n.processingMsg : "Processing...");
  const readyText = i18n ? i18n.readyStatus : "Ready";
  const container = document.getElementById("progress-container");
  const bar = document.getElementById("progress-bar");
  const status = document.getElementById("progress-status");
  const btn = document.getElementById("execute-backup-btn");
  const cBar = document.getElementById("compact-progress-bar");
  const cSts = document.getElementById("compact-status-label");
  const cBtn = document.getElementById("compact-execute-btn");

  if (show) {
    isExecuting = true;
    if (container) container.style.display = "block";
    if (status) {
      status.style.display = "block";
      status.textContent = displayMsg;
    }
    if (bar) bar.style.width = "0%";
    if (btn) btn.setAttribute("disabled", "");
    if (cSts) cSts.textContent = displayMsg;
    if (cBar) cBar.style.width = "0%";
    if (cBtn) cBtn.setAttribute("disabled", "");
  } else {
    if (bar) bar.style.width = "100%";
    if (cBar) cBar.style.width = "100%";
    setTimeout(() => {
      isExecuting = false;
      if (container) container.style.display = "none";
      if (status) status.style.display = "none";
      if (btn) btn.removeAttribute("disabled");
      if (cSts) cSts.textContent = readyText;
      if (cBar) cBar.style.width = "0%";
      if (cBtn) cBtn.removeAttribute("disabled");
    }, 500);
  }
}

export function showStartupOverlay() {
  const overlay = document.getElementById("startup-overlay");
  if (!overlay) return;

  const titleEl = document.getElementById("loader-title");
  const subEl = document.getElementById("loader-sub");

  if (titleEl) titleEl.textContent = i18n.loadingTitle || "Initializing...";
  if (subEl) subEl.textContent = i18n.pleaseWait || "Please wait...";

  overlay.style.display = "flex";
  overlay.style.opacity = "1";
}

export function updateStartupProgress(current, total) {
  const statusEl = document.getElementById("loader-status");
  if (!statusEl || !i18n.loadingStatus) return;

  statusEl.textContent = i18n.loadingStatus
    .replace("{current}", current)
    .replace("{total}", total);
}

export function hideStartupOverlay() {
  const overlay = document.getElementById("startup-overlay");
  if (!overlay) return;

  overlay.style.opacity = "0";

  setTimeout(() => {
    overlay.style.display = "none";
  }, 400);
}

export async function UpdateAllUI() {
  renderRecentFiles();
  renderTabs();
  await UpdateDisplay();
  UpdateHistory();
}
