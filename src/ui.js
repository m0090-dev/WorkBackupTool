import {
  GetBackupList,
  GetFileSize,
  WriteTextFile,
  ReadTextFile,
  GetConfigDir,
  GetGenerationFolders,
  UpdateConfigValue,
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

import { showMemoDialog,parseNoteContent } from "./memo.js";

import { switchTab, removeTab, reorderTabs } from "./actions";

let isExecuting = false;

// UI描画・メッセージ系（通常版）
export function showFloatingMessage(text) {
  const msgArea = document.getElementById("message-area");
  if (!msgArea) return;

  // --- 追加：前回の「赤」が残っていたら消す ---
  msgArea.classList.remove("error");

  msgArea.textContent = text;
  msgArea.classList.remove("hidden");

  // 既存のタイマーと競合しないよう、単純に3秒後に隠す
  setTimeout(() => msgArea.classList.add("hidden"), 3000);
}

// エラー版
export function showFloatingError(text) {
  const msgArea = document.getElementById("message-area");
  if (!msgArea) return;

  // 一旦リセットしてから赤を付ける
  msgArea.classList.add("error");
  msgArea.textContent = text;
  msgArea.classList.remove("hidden");

  setTimeout(() => {
    msgArea.classList.add("hidden");
    // 完全に隠れてから色を戻す
    setTimeout(() => {
      // まだ hidden 状態のときだけクラスを消す（連打対策）
      if (msgArea.classList.contains("hidden")) {
        msgArea.classList.remove("error");
      }
    }, 500);
  }, 3000);
}

export function renderRecentFiles() {
  const list = document.getElementById("recent-list");
  const section = document.getElementById("recent-files-section");
  if (!list) return;

  if (recentFiles.length === 0) {
    list.innerHTML = `<span class="recent-empty">No recent files</span>`;
    return;
  }
  list.innerHTML = recentFiles
    .map((path) => {
      const fileName = path.split(/[\\/]/).pop();
      return `<div class="recent-item" title="${path}" data-path="${path}"><i></i> ${fileName}</div>`;
    })
    .join("");
}

//TODO: Drag and drop関係はtauri v2が修整されたら正常動作するはず
export function renderTabs() {
  const list = document.getElementById("tabs-list");
  if (!list) return;

  // 初期化
  list.innerHTML = "";

  const clearGlobals = () => {
    const existingMenu = document.querySelector(".tab-context-menu");
    if (existingMenu) existingMenu.remove();
    const existingTooltips = document.querySelectorAll(".tab-tooltip");
    existingTooltips.forEach((t) => t.remove());
  };
  clearGlobals();

  let tooltip = null;

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

    // --- 右クリックメニュー（空表示防止版） ---
    el.oncontextmenu = (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (el._removeTooltip) el._removeTooltip();

      const existingMenu = document.querySelector(".tab-context-menu");
      if (existingMenu) existingMenu.remove();

      // 1. まずは一時的なフラグメントや配列で項目を準備する
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

      // 2. 項目が一つもなければメニュー自体を作らない
      if (menuItems.length === 0) return;

      // 3. 項目がある場合のみメニューを構築
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

/**
 * タブ専用のツールチップをセットアップします
 * @param {HTMLElement} el - 対象のタブ要素
 * @param {Object} tab - state.js のタブオブジェクト
 */
function setupTabTooltip(el, tab) {
  if (!el || !tab || !i18n) return;

  // ツールチップ実体への参照を管理するためのクロージャ用変数
  let tooltip = null;

  const removeTooltip = () => {
    if (tooltip) {
      tooltip.remove();
      tooltip = null;
    }
  };

  el.addEventListener("mouseenter", () => {
    // コンテキストメニューが表示されている時は出さない
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

    // 位置計算
    const rect = el.getBoundingClientRect();
    tooltip.style.left = `${rect.left}px`;
    tooltip.style.top = `${rect.bottom + 5}px`;
  });

  // 各種イベントで確実に消す
  el.addEventListener("mouseleave", removeTooltip);
  el.addEventListener("mousedown", removeTooltip);

  // ドラッグ開始時などにも消えるように el に関数を保持させておくと便利
  el._removeTooltip = removeTooltip;
}

function setupPathTooltip(el, fullPath) {
  if (!el || !fullPath) return;

  // 既存のセットアップ済みチェック
  if (el._pathTooltipSetup) {
    el._tooltipPath = fullPath;
    return;
  }

  el._pathTooltipSetup = true;
  el._tooltipPath = fullPath;

  el.addEventListener("mouseenter", () => {
    // 【修正】幅の判定を削除し、常に表示するように変更
    const tooltip = document.createElement("div");
    tooltip.className = "tab-tooltip";

    // 改行コード (\n) を <br> に変換して表示できるようにする
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
  let fileExists = true;
  let dirExists = true;
  const tab = getActiveTab();
  if (!i18n || !tab) return;

  if (tab.workFile) {
    fileExists = await FileExists(tab.workFile);
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
    if (!fileExists && tab.workFile) {
      // 存在しない場合：赤文字で (Not Found) を付加
      fileEl.innerHTML = `<span style="color: #ff4d4d; font-weight: bold;">${baseName} (Not Found)</span>`;
    } else {
      fileEl.textContent = `${baseName}${sizeText}`;
      fileEl.style.color = ""; // リセット
    }
  }
  if (dirEl) {
    const baseName = tab.backupDir
      ? tab.backupDir.split(/[\\/]/).pop()
      : i18n.selectedBackupDir;
    if (!dirExists && tab.backupDir) {
      // 存在しない場合：赤文字で (Not Found) を付加
      dirEl.innerHTML = `<span style="color: #ff4d4d; font-weight: bold;">${baseName} (Not Found)</span>`;
    } else {
      dirEl.textContent = baseName;
      dirEl.style.color = ""; // リセット
    }
  }

  const radio = document.querySelector(
    `input[name="backupMode"][value="${tab.backupMode}"]`,
  );
  if (radio) radio.checked = true;
  const compactModeSel = document.getElementById("compact-mode-select");
  if (compactModeSel) compactModeSel.value = tab.backupMode;

  // --- 各要素の同期 ---
  const locked = tab.isLocked || false;

  // ラジオボタン
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
  // Compact同期
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
  // --- 検索クリアボタンの表示制御 ---
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

    // --- ファイル名の降順でソート ---
    data.sort((a, b) => b.fileName.localeCompare(a.fileName));

    // 1. 本来の最新世代を取得
    const latestGenNumber = Math.max(
      ...data.map((item) => item.generation || 0),
    );

    // 2. 表示用のパスを決定する
    let activeDirPath = tab.selectedTargetDir;
    if (!activeDirPath) {
      const first = data[0];
      activeDirPath = first.filePath.replace(/[\\/][^\\/]+$/, "");
    }

    // --- ハイライト用のヘルパー関数 ---
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

        // --- 検索フィルタリング (ファイル名 または メモ に含まれるか) ---
        if (searchTerm) {
          const inFileName = item.fileName.toLowerCase().includes(searchTerm);
          const inNote = note.toLowerCase().includes(searchTerm);
          if (!inFileName && !inNote) return null; // ヒットしない場合はスキップ
        }

        const isDiffFile = item.fileName.toLowerCase().endsWith(".diff");
        const isArchive = !isDiffFile && item.generation === 0;

        const itemDir =
          item.filePath.substring(0, item.filePath.lastIndexOf("/")) ||
          item.filePath.substring(0, item.filePath.lastIndexOf("\\"));

        let statusHtml = "";
        let genBadge = "";
	const mark = noteMeta?.mark ?? 0;
const markBadge = mark > 0 ? (() => {
  const markColors = { 1: "#e6a817", 2: "#db741f", 3: "#c0392b" };
  const markLabels = { 1:  i18n?.priorityLow || "Low", 2: i18n?.priorityMid || "Mid", 3: i18n?.priorityHigh || "High" };
  return `<span style="font-size:10px; color:#fff; background:${markColors[mark]}; padding:1px 4px; border-radius:3px; margin-left:3px;">${markLabels[mark]}</span>`;
})() : "";

        if (isArchive) {
          const archiveText = i18n.fullArchive || " Full Archive";
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
          let statusIcon = isTarget ? "✅" : "";
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

        
  	const markLabels = { 1:  i18n?.priorityLow || "Low", 2: i18n?.priorityMid || "Mid", 3: i18n?.priorityHigh || "High" };
	const markText = mark > 0 ? `<br><strong>${i18n?.priorityLabel || "Priority"}:</strong> ${markLabels[mark]}` : "";

	const popupContent = `${statusHtml}<hr style="border:0; border-top:1px solid #eee; margin:5px 0;"><strong>Path:</strong> ${item.filePath}${markText}${note ? `<br><hr style="border:0; border-top:1px dashed #ccc; margin:5px 0;"><strong>${i18n.backupMemo}:</strong> ${note}` : ""}`;

        // ハイライト適用済みのテキストを作成
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
                ${note ? `<div style="font-size:10px; color:#2f8f5b; font-style:italic; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;"> ${displayedNote}</div>` : ""}
              </div>
            </label>
            <button class="note-btn" data-path="${item.filePath}" style="background:none; border:none; cursor:pointer; font-size:14px; padding:4px;"></button>
          </div>
        </div>`;
      }),
    );
    // フィルタで null になった要素を除外して結合
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

/**
 * 世代アーカイブ用モーダルのUIを更新して表示する
 * 専用の GetGenerationFolders を使用して、正確な世代フォルダリストを表示します。
 */
export async function showArchiveModal() {
  const tab = getActiveTab();
  const modal = document.getElementById("archive-modal");
  const listContainer = document.getElementById("archive-gen-list");

  if (!tab || !modal || !listContainer || !i18n) return;

  // i18nテキストの適用（タイトルやラベルの更新）
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
    // 1. 専用コマンドでアーカイブ候補（baseN_ フォルダ）を直接取得
    // ※ Rust側で「最新世代」は除外済みのリストが返ってきます
    const archiveCandidates = await GetGenerationFolders(
      tab.workFile,
      tab.backupDir,
    );

    // 2. 世代番号でソート（新しい順に表示）
    archiveCandidates.sort((a, b) => b.generation - a.generation);

    if (archiveCandidates.length === 0) {
      // 候補がない場合
      listContainer.innerHTML = `
        <div style="font-size:11px; color:#888; text-align:center; padding:15px;">
          ${i18n.noArchiveCandidates || "No folders available to archive."}
        </div>`;
      document.getElementById("archive-execute-btn").disabled = true;
    } else {
      // 3. リストの構築
      // data-gen 属性を付与して、実行時に Rust へ渡す targetN を特定しやすくします
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

    // 4. 全選択チェックボックスのリセット
    const selectAll = document.getElementById("archive-select-all-check");
    if (selectAll) selectAll.checked = false;

    // 5. モーダルを表示
    modal.classList.remove("hidden");
  } catch (err) {
    console.error("Failed to load archive candidates:", err);
    showFloatingError(
      i18n.errorLoadingHistory || "Failed to load generation folders",
    );
  }
}

/**
 * 詳細設定モーダルのUIを更新して表示する
 */
export async function showSettingsModal() {
  const modal = document.getElementById("settings-modal");
  if (!modal || !i18n) return;
  try {
    // 2. 最新の設定値をRust側から取得
    const config = await GetConfig();

    // 3. フォームに値をセット
    const cacheInput = document.getElementById("input-cache-limit");
    const thresholdInput = document.getElementById("input-threshold");

    if (cacheInput) cacheInput.value = config.startupCacheLimit;
    if (thresholdInput)
      thresholdInput.value = config.autoBaseGenerationThreshold;

    // 4. モーダルを表示
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
    showFloatingError(i18n.memoSaveError || "Save failed");
  }
}

function setupHistoryPopups() {
  // IDを history-tooltip に変更
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

      // 位置計算（ロジックは維持）
      const rect = target.getBoundingClientRect();
      tooltip.style.left = `${rect.left}px`;
      tooltip.style.top = `${rect.bottom + 5}px`;

      // 1. 高さと画面端をチェックするための変数を追加
      const tooltipHeight = tooltip.offsetHeight;
      const windowHeight = window.innerHeight;

      // 2. 位置計算を「入り切らないなら上」という条件分岐に変更
      let topPosition = rect.bottom + 2;
      if (topPosition + tooltipHeight > windowHeight) {
        topPosition = rect.top - tooltipHeight - 2;
      }

      // 3. 計算した値を代入
      tooltip.style.top = `${topPosition}px`;
    };

    target.onmouseleave = () => {
      tooltip.classList.add("hidden");
    };
  });
}

function createTooltipElement() {
  const el = document.createElement("div");
  // IDとクラス名を history-tooltip に変更
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

/**
 * 起動時の全画面オーバーレイを表示
 */
export function showStartupOverlay() {
  const overlay = document.getElementById("startup-overlay");
  if (!overlay) return;

  // i18nからテキストを取得して反映
  const titleEl = document.getElementById("loader-title");
  const subEl = document.getElementById("loader-sub");

  if (titleEl) titleEl.textContent = i18n.loadingTitle || "Initializing...";
  if (subEl) subEl.textContent = i18n.pleaseWait || "Please wait...";

  // 初期状態をセット
  overlay.style.display = "flex";
  overlay.style.opacity = "1";
}

/**
 * キャッシュ生成などの進捗状況を更新する
 * @param {number} current - 現在の処理数
 * @param {number} total - 総数
 */
export function updateStartupProgress(current, total) {
  const statusEl = document.getElementById("loader-status");
  if (!statusEl || !i18n.loadingStatus) return;

  // i18nの "アーカイブキャッシュを処理しています... ({current}/{total})" を置換
  statusEl.textContent = i18n.loadingStatus
    .replace("{current}", current)
    .replace("{total}", total);
}

/**
 * 起動時のオーバーレイをフェードアウトさせて非表示にする
 */
export function hideStartupOverlay() {
  const overlay = document.getElementById("startup-overlay");
  if (!overlay) return;

  // フェードアウト
  overlay.style.opacity = "0";

  // アニメーションが終わるのを待ってから完全に消す（CSSのtransition時間に合わせる）
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
