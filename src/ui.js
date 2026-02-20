import {
  GetBackupList,
  GetFileSize,
  WriteTextFile,
  ReadTextFile,
  GetConfigDir,
  GetGenerationFolders,
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

import { showMemoDialog } from "./memo.js";

import { switchTab, removeTab, reorderTabs } from "./actions";

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

    const removeTooltip = () => {
      if (tooltip) {
        tooltip.remove();
        tooltip = null;
      }
    };

    // --- ツールチップ・ドラッグ設定（既存ロジック維持） ---
    el.addEventListener("mouseenter", () => {
      if (document.querySelector(".tab-context-menu")) return;
      removeTooltip();
      tooltip = document.createElement("div");
      tooltip.className = "tab-tooltip";
      const fullPath = tab.workFile || "No file selected";
      tooltip.innerHTML = `<b>${fileName}</b><code>${fullPath}</code>`;
      document.body.appendChild(tooltip);
      const rect = el.getBoundingClientRect();
      tooltip.style.left = `${rect.left}px`;
      tooltip.style.top = `${rect.bottom + 5}px`;
    });
    el.addEventListener("mouseleave", removeTooltip);
    el.addEventListener("mousedown", removeTooltip);

    el.draggable = true;
    el.dataset.id = tab.id;
    el.ondragstart = (e) => {
      removeTooltip();
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
    el.ondrop = (e) => {
      e.preventDefault();
      const dId = e.dataTransfer.getData("text/plain");
      if (dId && dId !== el.dataset.id) reorderTabs(dId, el.dataset.id);
    };

    el.onclick = () => {
      removeTooltip();
      switchTab(tab.id);
    };

    // --- 右クリックメニュー（空表示防止版） ---
    el.oncontextmenu = (e) => {
      e.preventDefault();
      e.stopPropagation();
      removeTooltip();

      const existingMenu = document.querySelector(".tab-context-menu");
      if (existingMenu) existingMenu.remove();

      // 1. まずは一時的なフラグメントや配列で項目を準備する
      const menuItems = [];

      if (index > 0) {
        const item = document.createElement("div");
        item.className = "tab-menu-item";
        item.innerHTML = `<span>${i18n.tabMenuMoveLeft}</span><span class="tab-menu-shortcut">◀</span>`;
        item.onclick = (ev) => {
          ev.stopPropagation();
          reorderTabs(tab.id, tabs[index - 1].id);
          menu.remove();
        };
        menuItems.push(item);
      }

      if (index < tabs.length - 1) {
        const item = document.createElement("div");
        item.className = "tab-menu-item";
        item.innerHTML = `<span>${i18n.tabMenuMoveRight}</span><span class="tab-menu-shortcut">▶</span>`;
        item.onclick = (ev) => {
          ev.stopPropagation();
          reorderTabs(tab.id, tabs[index + 1].id);
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
        del.onclick = (ev) => {
          ev.stopPropagation();
          removeTab(tab.id);
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

// 全体のUI更新
export function UpdateDisplay() {
  const tab = getActiveTab();
  if (!i18n || !tab) return;
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
  if (fileEl)
    fileEl.textContent =
      (tab.workFile
        ? tab.workFile.split(/[\\/]/).pop()
        : i18n.selectedWorkFile) +
      (tab.workFile ? ` [${formatSize(tab.workFileSize)}]` : "");
  if (dirEl) dirEl.textContent = tab.backupDir || i18n.selectedBackupDir;

  const radio = document.querySelector(
    `input[name="backupMode"][value="${tab.backupMode}"]`,
  );
  if (radio) radio.checked = true;
  const compactModeSel = document.getElementById("compact-mode-select");
  if (compactModeSel) compactModeSel.value = tab.backupMode;

  // --- 各要素の同期 ---
  const normalComp = document.getElementById("hdiff-compress");
  const compactComp = document.getElementById("compact-hdiff-compress");
  const compress = tab.compressMode || "zstd";
  const normalAlgo = document.getElementById("diff-algo");
  const algo = tab.diffAlgo || "hdiff";
  const normalArchive = document.getElementById("archive-format");
  const archiveFormat = tab.archiveFormat || "zip";
  const mode = tab.backupMode || "diff";

  if (normalAlgo) normalAlgo.value = algo;
  if (normalComp) normalComp.value = compress;
  if (compactComp) compactComp.value = compress;
  if (normalArchive) normalArchive.value = archiveFormat;

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
  if (cSel && mode) cSel.value = mode;
}

export async function UpdateHistory() {
  const tab = getActiveTab();
  const list = document.getElementById("diff-history-list");
  const searchInput = document.getElementById("history-search");
  const searchTerm = (tab?.searchQuery || "").toLowerCase().trim();
  const clearBtn = document.getElementById("search-clear-btn");

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

    const itemsHtml = await Promise.all(
      data.map(async (item) => {
        const note = await ReadTextFile(item.filePath + ".note").catch(
          () => "",
        );

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

        if (isArchive) {
          const archiveText = i18n.fullArchive || " Full Archive";
          statusHtml = `<div style="color:#2f8f5b; font-weight:bold;">${archiveText}</div>`;
          genBadge = `<span style="font-size:10px; color:#fff; background:#2f8f5b; padding:1px 4px; border-radius:3px; margin-left:5px;">Archive</span>`;
        } else {
          const currentGen = item.generation || 1;
          const isTarget = itemDir === activeDirPath;
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
            statusText = i18n.archived_generation_extracting || "世代アーカイブ (一時展開中)";
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

        const popupContent = `${statusHtml}<hr style="border:0; border-top:1px solid #eee; margin:5px 0;"><strong>Path:</strong> ${item.filePath}${note ? `<br><hr style="border:0; border-top:1px dashed #ccc; margin:5px 0;"><strong>${i18n.backupMemo}:</strong> ${note}` : ""}`;

        // ハイライト適用済みのテキストを作成
        const displayedFileName = highlight(item.fileName, searchTerm);
        const displayedNote = highlight(note, searchTerm);

        return `<div class="diff-item" style="${itemDir === activeDirPath ? "border-left: 4px solid #2f8f5b; background: #f0fff4;" : ""}">
          <div style="display:flex; align-items:center; width:100%;">
            <label style="display:flex; align-items:center; cursor:pointer; flex:1; min-width:0;">
              <input type="checkbox" class="diff-checkbox" value="${item.filePath}" style="margin-right:10px;">
              <div style="display:flex; flex-direction:column; flex:1; min-width:0;">
                <span class="diff-name" data-hover-content="${encodeURIComponent(popupContent)}" style="font-weight:bold; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                  ${displayedFileName} ${genBadge} <span style="font-size:10px; color:#3B5998;">(${formatSize(item.fileSize)})</span>
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
            <span style="font-weight:bold; color:#fff;">Gen.${c.generation}</span>
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
    if (container) container.style.display = "block";
    if (status) {
      status.style.display = "block";
      status.textContent = displayMsg;
    }
    if (bar) bar.style.width = "0%";
    if (btn) btn.disabled = true;
    if (cSts) cSts.textContent = displayMsg;
    if (cBar) cBar.style.width = "0%";
    if (cBtn) cBtn.disabled = true;
  } else {
    if (bar) bar.style.width = "100%";
    if (cBar) cBar.style.width = "100%";
    setTimeout(() => {
      if (container) container.style.display = "none";
      if (status) status.style.display = "none";
      if (btn) btn.disabled = false;
      if (cSts) cSts.textContent = readyText;
      if (cBar) cBar.style.width = "0%";
      if (cBtn) cBtn.disabled = false;
    }, 500);
  }
}

export function UpdateAllUI() {
  renderRecentFiles();
  renderTabs();
  UpdateDisplay();
  UpdateHistory();
}
