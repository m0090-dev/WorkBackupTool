import {
  GetI18N,
  GetFileSize,
  OnFileDrop,
  RebuildArchiveCaches,
  GetRebuildCacheOnStartup,
  GetStartupCacheLimit,
  DirExists,
} from "./tauri_exports";

import {
  i18n,
  setI18N,
  tabs,
  getActiveTab,
  addToRecentFiles,
  restoreSession,
  saveCurrentSession,
} from "./state";

import {
  renderRecentFiles,
  renderTabs,
  UpdateDisplay,
  UpdateHistory,
  showFloatingMessage,
  showFloatingError,
  UpdateAllUI,
  updateStartupProgress,
  showStartupOverlay,
  hideStartupOverlay,
} from "./ui";

import { setupGlobalEvents } from "./events";
import { switchTab } from "./actions";

// --- 初期化ロジック ---
async function Initialize() {
  const data = await GetI18N();

  if (!data) return;

  // stateにi18nデータをセット
  setI18N(data);

  await restoreSession();
  showStartupOverlay();
  await performStartupCacheRebuild();

  setupInitialUI();

  setupDragAndDrop();

  await setupGlobalEvents();

  // 先頭タブを必ずアクティブにする
  if (tabs.length > 0 && !tabs.some((t) => t.active)) {
    switchTab(tabs[0].id);
  } else {
    await UpdateAllUI();
  }
  hideStartupOverlay();
}

/**
 * 起動時のキャッシュ再構築処理
 */
async function performStartupCacheRebuild() {
  const shouldRebuild = await GetRebuildCacheOnStartup();
  if (!shouldRebuild) return;

  const limit = await GetStartupCacheLimit();
  const eligibleTabs = tabs.filter((t) => t.workFile);
  let processedCount = 0;

  for (const tab of eligibleTabs) {
    if (limit > 0 && processedCount >= limit) break;

    // プログレスバーの表示更新（関数があれば実行）
    if (typeof updateStartupProgress === "function") {
      updateStartupProgress(processedCount + 1, eligibleTabs.length);
    }
    try {
      await RebuildArchiveCaches(tab.workFile, tab.backupDir || "");
    } catch (e) {
      console.error(`Cache rebuild failed for ${tab.workFile}:`, e);
    }
    processedCount++;
  }
}

function setupInitialUI() {
  const setTitle = (id, text) => {
    const el = document.getElementById(id);
    if (el) el.title = text || "";
  };

  const setText = (id, text) => {
    const el = document.getElementById(id);
    if (el) el.textContent = text || "";
  };
  const setPlaceholder = (id, text) => {
    const el = document.getElementById(id);
    if (el) el.placeholder = text || "";
  };

  const setQueryText = (sel, text) => {
    const el = document.querySelector(sel);
    if (el) el.textContent = text || "";
  };

  setQueryText(".action-section h3", i18n.newBackupTitle);

  setQueryText(".history-section h3", i18n.historyTitle);
  setQueryText(".recent-title", i18n.recentFilesTitle);
  setText("workfile-btn", i18n.workFileBtn);

  setText("backupdir-btn", i18n.backupDirBtn);

  setText("label-target", i18n.labelWorkFile);
  setText("label-location", i18n.labelLocation);
  setPlaceholder("history-search", i18n.searchPlaceholder || "Search...");
  setText("progress-status", i18n.readyStatus || "Ready");

  const titles = document.querySelectorAll(".mode-title");

  const descs = document.querySelectorAll(".mode-desc");

  if (titles.length >= 3) {
    titles[0].textContent = i18n.fullCopyTitle;
    descs[0].textContent = i18n.fullCopyDesc;

    titles[1].textContent = i18n.archiveTitle;
    descs[1].textContent = i18n.archiveDesc;

    titles[2].textContent = i18n.diffTitle;
    descs[2].textContent = i18n.diffDesc;
  }

  setText("execute-backup-btn", i18n.executeBtn);
  const lockBtn = document.getElementById("lock-mode-btn");
  if (lockBtn) lockBtn.title = i18n.lockMode || "Lock backup mode";

  setText("refresh-diff-btn", i18n.refreshBtn);

  setText("apply-selected-btn", i18n.applyBtn);

  setText("select-all-btn", i18n.selectAllBtn);

  setText("drop-modal-title", i18n.dropModalTitle);

  setText("drop-set-workfile", i18n.dropSetWorkFile);

  setText("drop-set-backupdir", i18n.dropSetBackupDir);

  setText("drop-cancel", i18n.dropCancel);

  setTitle("add-tab-btn", i18n.addTabBtn || "Add New Tab");

  // --- ここに追記 ---
  const typeSelect = document.getElementById("work-target-type-select");
  if (typeSelect) {
    if (typeSelect.options[0])
      typeSelect.options[0].textContent = i18n.workTargetTypeFile || "File";
    if (typeSelect.options[1])
      typeSelect.options[1].textContent = i18n.workTargetTypeFolder || "Folder";
  }

  // Compact用テキスト
  const typeCompactSelect = document.getElementById("compact-work-type-select");
  if (typeCompactSelect) {
    if (typeCompactSelect.options[0])
      typeCompactSelect.options[0].textContent =
        i18n.workTargetTypeFile || "File";
    if (typeCompactSelect.options[1])
      typeCompactSelect.options[1].textContent =
        i18n.workTargetTypeFolder || "Folder";
  }

  setQueryText(".compact-title-text", i18n.compactMode || "Compact");

  setText("compact-workfile-btn", i18n.workFileBtn);

  setText("compact-execute-btn", i18n.executeBtn);

  const cSel = document.getElementById("compact-mode-select");

  if (cSel && cSel.options.length >= 3) {
    cSel.options[0].text = i18n.fullCopyTitle;

    cSel.options[1].text = i18n.archiveTitle;

    cSel.options[2].text = i18n.diffTitle;
  }

  const workBtn = document.getElementById("workfile-btn");

  const recentSec = document.querySelector(".recent-files-section");

  if (workBtn && recentSec) {
    workBtn.addEventListener("mouseenter", () => {
      recentSec.style.display = "block";
      setTimeout(() => (recentSec.style.opacity = "1"), 10);
    });

    workBtn.addEventListener("mouseleave", () => {
      setTimeout(() => {
        if (!recentSec.matches(":hover")) {
          recentSec.style.display = "none";
          recentSec.style.opacity = "0";
        }
      }, 300);
    });

    recentSec.addEventListener("mouseleave", () => {
      recentSec.style.display = "none";
      recentSec.style.opacity = "0";
    });
  }
}

// --- ドラッグアンドドロップ設定 ---

function setupDragAndDrop() {
  OnFileDrop(async (x, y, paths) => {
    if (!paths || paths.length === 0) return;

    const droppedPath = paths[0];

    const modal = document.getElementById("drop-modal");

    const pathText = document.getElementById("drop-modal-path");

    (async () => {
      const isDir = await DirExists(droppedPath).catch(() => false);

      pathText.textContent = droppedPath;

      modal.classList.remove("hidden");

      document.getElementById("drop-set-workfile").onclick = async () => {
        const tab = getActiveTab();

        tab.workFile = droppedPath;

        tab.workFileSize = await GetFileSize(droppedPath);

        tab.backupDir = "";
        tab.selectedTargetDir = "";

        addToRecentFiles(droppedPath);

        finishDrop(i18n.updatedWorkFile);
      };

      document.getElementById("drop-set-backupdir").onclick = () => {
        const tab = getActiveTab();
        if (!isDir) {
          showFloatingError(
            i18n.dropErrorFileAsFolder ||
              "ファイルはフォルダとして設定できません",
          );
          return;
        }
        tab.backupDir = droppedPath;

        finishDrop(i18n.updatedBackupDir);
      };

      document.getElementById("drop-cancel").onclick = () => {
        modal.classList.add("hidden");
      };

      async function finishDrop(msg) {
        modal.classList.add("hidden");

        showFloatingMessage(msg);

        await UpdateAllUI();

        saveCurrentSession();
      }
    })();
  }, true);
}

// アプリケーション開始

document.addEventListener("DOMContentLoaded", Initialize);
