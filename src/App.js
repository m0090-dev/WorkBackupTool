import {
  GetI18N,
  GetFileSize,
  OnFileDrop,
  RebuildArchiveCaches,
  GetRebuildCacheOnStartup,
  GetStartupCacheLimit,
  PregeneratePreviews,
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
import {
  chooseBackupDirMode,
  resolveEffectiveBackupDir,
} from "./backup-dir.js";
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
  // プレビューの先行生成は起動を妨げないよう、投げっぱなしで裏で進める
  // (オン/オフは Rust 側が設定を見て判断する)
  performStartupPreviewGeneration();

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
  const eligibleTabs = tabs.filter((t) => t.workFile || t.backupDir);
  let processedCount = 0;

  for (const tab of eligibleTabs) {
    if (limit > 0 && processedCount >= limit) break;

    // プログレスバーの表示更新（関数があれば実行）
    if (typeof updateStartupProgress === "function") {
      updateStartupProgress(processedCount + 1, eligibleTabs.length);
    }
    try {
      await RebuildArchiveCaches(
        tab.workFile,
        (await resolveEffectiveBackupDir(tab)) || "",
      );
    } catch (e) {
      console.error(`Cache rebuild failed for ${tab.workFile}:`, e);
    }
    processedCount++;
  }
}

/**
 * 起動時のプレビュー先行生成。
 * キャッシュ再構築の上限(startupCacheLimit)を流用し、タブごとに
 * 新しい順に上限件数までサムネイルを作っておく。
 */
async function performStartupPreviewGeneration() {
  try {
    const limit = await GetStartupCacheLimit();
    for (const tab of tabs) {
      if (!tab.workFile && !tab.backupDir) continue;
      await PregeneratePreviews(
        tab.workFile || "",
        (await resolveEffectiveBackupDir(tab)) || "",
        limit,
      );
    }
  } catch (e) {
    console.error("Preview pregeneration failed:", e);
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

  setText("execute-backup-btn", i18n.executeBtn);
  setText("apply-selected-btn", i18n.restoreBtn);
  setText("note-btn", i18n.backupMemo);
  setText("refresh-diff-btn", i18n.refreshBtn);
  setText("workfile-btn", i18n.changeBtn);
  setText("backupdir-btn", i18n.changeBtn);
  setText("label-target", i18n.labelWorkFile);
  setText("label-location", i18n.labelLocation);
  setText("label-recent", i18n.recentFilesTitle);
  setText("generation-archive-btn", i18n.generationArchive);
  setPlaceholder(
    "history-search",
    i18n.searchNotePlaceholder || i18n.searchPlaceholder || "Search...",
  );
  setText("progress-status", i18n.processingMsg);
  setText("col-date", i18n.colDate);
  setText("col-size", i18n.colSize);
  setText("col-priority", i18n.colPriority);
  setTitle("add-tab-btn", i18n.addTabBtn || "Add New Tab");
  setTitle("search-clear-btn", i18n.cancel || "Clear");

  // 作業対象メニュー
  const targetMenuItems = document.querySelectorAll("#target-menu .menu-item");
  if (targetMenuItems.length >= 2) {
    targetMenuItems[0].textContent = i18n.targetMenuFile;
    targetMenuItems[1].textContent = i18n.targetMenuFolder;
  }

  // バックアップモード設定パネル(通常画面 "" / コンパクト画面 "compact-")
  ["", "compact-"].forEach((pfx) => {
    const titles = document.querySelectorAll(
      `#${pfx}mode-panel .mode-option-title`,
    );
    const descs = document.querySelectorAll(
      `#${pfx}mode-panel .mode-option-desc`,
    );
    if (titles.length >= 3 && descs.length >= 3) {
      titles[0].textContent = i18n.fullCopyTitle;
      descs[0].textContent = i18n.fullCopyDesc;
      titles[1].textContent = i18n.archiveTitle;
      descs[1].textContent = i18n.archiveDesc;
      titles[2].textContent = i18n.diffTitle;
      descs[2].textContent = i18n.diffDesc;
    }
    setText(`${pfx}mode-panel-title`, i18n.backupMode);
    setText(`${pfx}mode-panel-close`, i18n.closeBtn);
    setText(`${pfx}label-archive-format`, i18n.settingFormat);
    setText(`${pfx}label-archive-password`, i18n.settingPassword);
    setText(`${pfx}label-diff-algo`, i18n.settingAlgorithm);
    setText(`${pfx}label-diff-compress`, i18n.settingCompression);
    setText(`${pfx}lock-mode-label`, i18n.lockModeLabel);
    setPlaceholder(`${pfx}archive-password`, i18n.passwordPlaceholder);
  });
  ["mode-btn", "compact-mode-btn"].forEach((id) =>
    setTitle(id, i18n.modeButtonTip),
  );

  setText("drop-modal-title", i18n.dropDialogTitle);
  setText("drop-modal-label", i18n.dropPathLabel);
  setText("drop-modal-instruction", i18n.dropSelectTarget);
  setText("drop-set-workfile", i18n.dropSetWorkFile);
  setText("drop-set-backupdir", i18n.dropSetBackupDir);
  setText("drop-cancel", i18n.dropCancel);

  // コンパクト画面
  setText("compact-execute-btn", i18n.executeBtn);
  setText("compact-history-btn", i18n.historyBtn);
  setText("compact-history-title", i18n.compactHistoryTitle);
  setText("compact-note-btn", i18n.backupMemo);
  setText("compact-restore-btn", i18n.restoreBtn);
  setText("compact-back-btn", i18n.compactBack);
  setText("compact-selection-status", i18n.statusNoSelection);
  setText("c-col-date", i18n.colDate);
  setText("c-col-size", i18n.colSize);
  setText("c-col-priority", i18n.colPriority);
  setText("c-col-note", i18n.backupMemo);
  setText("label-compact-tabs", i18n.compactTabsLabel);
  setText("label-compact-recent", i18n.recentFilesTitle);
  const compactMenuItems = document.querySelectorAll(
    "#compact-target-panel .menu-item",
  );
  if (compactMenuItems.length >= 2) {
    compactMenuItems[0].textContent = i18n.targetMenuFile;
    compactMenuItems[1].textContent = i18n.targetMenuFolder;
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
        delete tab.backupDirMode;
        tab.viewGeneration = null;

        addToRecentFiles(droppedPath);

        finishDrop(i18n.updatedWorkFile);
      };

      document.getElementById("drop-set-backupdir").onclick = async () => {
        const tab = getActiveTab();
        if (!isDir) {
          showFloatingError(
            i18n.dropErrorFileAsFolder ||
              "ファイルはフォルダとして設定できません",
          );
          return;
        }
        // 使い方(親フォルダ / 直接)を決める。キャンセル時はドロップ画面に戻る
        const mode = await chooseBackupDirMode(droppedPath, tab.workFile);
        if (!mode) return;
        tab.backupDir = droppedPath;
        tab.backupDirMode = mode;

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

// 通知ポップアップ用ウィンドウ(main.js が描画する)では、アプリ本体を初期化しない
if (!window.__WBT_NOTIFY__) {
  document.addEventListener("DOMContentLoaded", Initialize);
}
