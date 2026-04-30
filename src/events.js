import {
  SelectAnyFile,
  SelectAnyFolder,
  GetFileSize,
  WriteTextFile,
  ReadTextFile,
  RestoreBackup,
  EventsOn,
  OnFileDrop,
  ArchiveGeneration,
} from "./tauri_exports";

import {
  i18n,
  getActiveTab,
  addToRecentFiles,
  saveCurrentSession,
  recentFiles,
} from "./state";

import {
  renderTabs,
  UpdateDisplay,
  UpdateHistory,
  toggleProgress,
  showFloatingMessage,
  showFloatingError,
  renderRecentFiles,
  showArchiveModal,
  showSettingsModal,
  handleSettingChange,
} from "./ui";

import { addTab, OnExecute, switchTab } from "./actions";
import { ask } from "@tauri-apps/plugin-dialog";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

import { showMemoDialog, parseNoteContent, serializeNote } from "./memo.js";

const preventDefault = (e) => {
  e.preventDefault();
  e.stopPropagation();
};

export async function setupGlobalEvents() {
  window.addEventListener("dragenter", preventDefault, true);

  // ---- 作業ファイル/フォルダ選択 ----
  const handleSelectWorkFile = async () => {
    const tab = getActiveTab();
    const isCompact = document.body.classList.contains("compact-mode");
    const targetId = isCompact
      ? "compact-work-type-select"
      : "work-target-type-select";
    const sel = document.getElementById(targetId);
    const targetType = sel ? sel.value : "file";

    let res;
    if (targetType === "folder") {
      res = await SelectAnyFolder(i18n.workFileBtn);
    } else {
      res = await SelectAnyFile(i18n.workFileBtn, [
        { DisplayName: "Work file", Pattern: "*.*" },
      ]);
    }
    if (res) {
      tab.workFile = res;
      tab.workFileSize = await GetFileSize(res).catch(() => 0);
      tab.backupDir = "";
      tab.selectedTargetDir = "";
      addToRecentFiles(res);
      renderTabs();
      await UpdateDisplay();
      UpdateHistory();
      saveCurrentSession();
      showFloatingMessage(i18n.updatedWorkFile);
    }
  };

  // バックアップ先フォルダ選択
  const handleSelectBackupDir = async () => {
    const tab = getActiveTab();
    const res = await SelectAnyFolder(i18n.backupDirBtn);
    if (res) {
      tab.backupDir = res;
      await UpdateDisplay();
      UpdateHistory();
      saveCurrentSession();
      showFloatingMessage(i18n.updatedBackupDir);
    }
  };

  // ---- [FIX #1] recent-item 選択ロジックを独立関数に ----
  // GetFileSize の失敗を catch し、プルダウンの pointer-events 干渉も回避する
  const handleSelectRecentItem = async (path) => {
    const tab = getActiveTab();
    try {
      tab.workFile = path;
      tab.workFileSize = await GetFileSize(path).catch(() => 0);
      tab.backupDir = "";
      tab.selectedTargetDir = "";

      addToRecentFiles(path);
      saveCurrentSession();

      renderRecentFiles();
      renderTabs();
      await UpdateDisplay();
      UpdateHistory();

      showFloatingMessage(i18n.updatedWorkFile);
    } catch (err) {
      console.error("Failed to load recent item:", err);
      const idx = recentFiles.indexOf(path);
      if (idx > -1) recentFiles.splice(idx, 1);
      localStorage.setItem("recentFiles", JSON.stringify(recentFiles));
      renderRecentFiles();
    }
  };

  // --- クリックイベントリスナー ---
  window.addEventListener("click", async (e) => {
    const target = e.target.closest("button") || e.target;
    const id = target.id;
    const tab = getActiveTab();

    if (id === "add-tab-btn") {
      addTab();
      return;
    }

    // 世代バッジ
    const genBadge = e.target.closest(".gen-selector-badge");
    if (genBadge) {
      e.preventDefault();
      e.stopPropagation();
      tab.selectedTargetDir = genBadge.getAttribute("data-dir");
      saveCurrentSession();
      UpdateHistory();
      return;
    }

    // 履歴のメモボタン
    const historyNoteBtn = e.target.closest(".diff-item .note-btn");
    if (historyNoteBtn) {
      e.preventDefault();
      e.stopPropagation();
      const path = historyNoteBtn.getAttribute("data-path");
      const notePath = path + ".note";

      let currentText = "";
      let currentMeta = { mark: 0 };

      try {
        const raw = await ReadTextFile(notePath);
        if (raw) {
          const parsed = parseNoteContent(raw);
          currentText = parsed.text;
          currentMeta = parsed.meta;
        }
      } catch (err) {
        currentText = "";
        currentMeta = { mark: 0 };
      }
      showMemoDialog(
        currentText,
        { ...currentMeta },
        async (newText, newMeta) => {
          try {
            const finalMeta = { ...newMeta, target: path };
            await WriteTextFile(notePath, serializeNote(newText, finalMeta));
            showFloatingMessage(i18n.memoSaved);
            UpdateHistory();
          } catch (err) {
            console.error(err);
            showFloatingError(i18n.memoSaveError);
          }
        },
      );
      return;
    }

    // ---- [FIX #1] recent-item: closest() で確実に捕捉、stopPropagation 追加 ----
    const recentItem = e.target.closest(".recent-item");
    if (recentItem) {
      e.preventDefault();
      e.stopPropagation();
      const path = recentItem.getAttribute("data-path");
      await handleSelectRecentItem(path);
      return;
    }

    if (id === "workfile-btn" || id === "compact-workfile-btn") {
      await handleSelectWorkFile();
      return;
    } else if (id === "backupdir-btn") {
      await handleSelectBackupDir();
      return;
    } else if (id === "execute-backup-btn" || id === "compact-execute-btn") {
      if (target.disabled) return;
      await OnExecute();
      return;
    } else if (id === "refresh-diff-btn") {
      UpdateHistory();
      return;
    } else if (id === "select-all-btn") {
      const cbs = document.querySelectorAll(".diff-checkbox");
      const all = Array.from(cbs).every((cb) => cb.checked);
      cbs.forEach((cb) => (cb.checked = !all));
      return;
    } else if (id === "apply-selected-btn") {
      e.preventDefault();
      e.stopPropagation();
      const targets = Array.from(
        document.querySelectorAll(".diff-checkbox:checked"),
      ).map((el) => el.value);
      if (targets.length === 0) return;

      const isConfirmed = await ask(i18n.restoreConfirm, {
        title: "CG File Backup",
        type: "warning",
      });

      if (isConfirmed) {
        toggleProgress(true, "Restoring...");
        try {
          for (const p of targets) {
            await RestoreBackup(p, tab.workFile);
          }
          toggleProgress(false);
          showFloatingMessage(i18n.diffApplySuccess);
          UpdateHistory();
        } catch (err) {
          toggleProgress(false);
          alert(err);
        }
      }
      return;
    }

    if (id == "generation-archive-btn") {
      if (!tab || !tab.workFile) {
        showFloatingError(
          i18n?.selectFileFirst || "Please select a work file first.",
        );
        return;
      }
      showArchiveModal();
    }

    if (id == "archive-cancel-btn") {
      document.getElementById("archive-modal").classList.add("hidden");
    } else if (id == "archive-modal") {
      if (e.target.id === "archive-modal") e.target.classList.add("hidden");
    }

    if (id === "archive-execute-btn") {
      const selectedChecks = document.querySelectorAll(
        ".archive-gen-check:checked",
      );
      if (selectedChecks.length === 0) {
        showFloatingError("Please select at least one generation.");
        return;
      }

      const format = document.getElementById("archive-format-select").value;
      toggleProgress(true, "Archiving...");

      try {
        for (const cb of selectedChecks) {
          const genNum = parseInt(cb.getAttribute("data-gen"));
          await ArchiveGeneration(genNum, format, tab.workFile, tab.backupDir);
        }
        toggleProgress(false);
        showFloatingMessage("Archiving completed.");
        document.getElementById("archive-modal").classList.add("hidden");
        UpdateHistory();
      } catch (err) {
        toggleProgress(false);
        console.error(err);
        alert("Archive Error: " + err);
      }
      return;
    }

    if (id == "lock-mode-btn") {
      if (!tab) return;
      tab.isLocked = !tab.isLocked;
      await UpdateDisplay();
      saveCurrentSession();
    }

    if (id === "settings-close-btn") {
      document.getElementById("settings-modal").classList.add("hidden");
    } else if (id === "settings-modal") {
      if (e.target.id === "settings-modal") e.target.classList.add("hidden");
    }
  });

  // --- 変更イベントリスナー ---
  document.addEventListener("change", async (e) => {
    const id = e.target.id;
    const name = e.target.name;
    const value = e.target.value;
    const tab = getActiveTab();

    if (id === "compact-tab-select") {
      switchTab(Number(value));
      return;
    }
    if (name == "diff-algo") {
      if (tab) tab.diffAlgo = value;
    }
    if (name === "backupMode") {
      if (tab) tab.backupMode = value;
    }
    if (id === "hdiff-compress" || id === "compact-hdiff-compress") {
      if (tab) tab.compressMode = value;
    }
    if (id == "archive-format") {
      if (tab) tab.archiveFormat = value;
    }
    if (
      ["backupMode", "archive-format"].includes(name) ||
      id === "archive-format" ||
      id === "diff-algo" ||
      id === "hdiff-compress" ||
      id === "compact-hdiff-compress"
    ) {
      await UpdateDisplay();
      saveCurrentSession();
    }
    if (id === "compact-mode-select") {
      const radio = document.querySelector(
        `input[name="backupMode"][value="${value}"]`,
      );
      if (radio) {
        radio.checked = true;
        if (tab) tab.backupMode = value;
      }
    }
    if (id == "archive-select-all-check") {
      const checks = document.querySelectorAll(".archive-gen-check");
      checks.forEach((c) => (c.checked = e.target.checked));
    }
  });

  window.addEventListener("contextmenu", (e) => {
    const recentItem = e.target.closest(".recent-item");
    if (recentItem) {
      e.preventDefault();
      e.stopPropagation();
      const path = recentItem.getAttribute("data-path");
      const idx = recentFiles.indexOf(path);
      if (idx > -1) {
        recentFiles.splice(idx, 1);
        localStorage.setItem("recentFiles", JSON.stringify(recentFiles));
        renderRecentFiles();
        saveCurrentSession();
      }
    }
  });

  // --- Rust / Tray イベント ---
  EventsOn("tray-execute-clicked", async () => {
    const resultMsg = await OnExecute();
    if (!resultMsg) return;
    let permissionGranted = await isPermissionGranted();
    if (!permissionGranted) {
      const permission = await requestPermission();
      permissionGranted = permission === "granted";
    }
    if (permissionGranted) {
      sendNotification({ title: "cg-file-backup", body: resultMsg });
    }
  });

  EventsOn("tray-change-work-clicked", () => {
    handleSelectWorkFile();
  });
  EventsOn("tray-change-backup-clicked", () => {
    handleSelectBackupDir();
  });

  EventsOn("tray-mode-change", async (newMode) => {
    const radio = document.querySelector(
      `input[name="backupMode"][value="${newMode}"]`,
    );
    if (radio) {
      radio.checked = true;
      radio.dispatchEvent(new Event("change", { bubbles: true }));
      const tab = getActiveTab();
      if (tab) tab.backupMode = newMode;
      await UpdateDisplay();
      saveCurrentSession();
      showFloatingMessage(`${i18n.updatedBackupMode || "Mode"}: ${newMode}`);
    }
  });

  EventsOn("compact-mode-event", async (isCompact) => {
    const view = document.getElementById("compact-view");
    if (isCompact) {
      document.body.classList.add("compact-mode");
      if (view) view.classList.remove("hidden");
      if (typeof UpdateDisplay === "function") await UpdateDisplay();
    } else {
      document.body.classList.remove("compact-mode");
      if (view) view.classList.add("hidden");
    }
  });

  EventsOn("open-advanced-settings", async () => {
    await showSettingsModal();
  });

  // --- 検索窓の入力監視 ---
  const searchInput = document.getElementById("history-search");
  const clearBtn = document.getElementById("search-clear-btn");

  if (searchInput) {
    searchInput.addEventListener("input", (e) => {
      const tab = getActiveTab();
      if (tab) {
        tab.searchQuery = e.target.value;
        saveCurrentSession();
      }
      UpdateHistory();
    });
  }

  if (clearBtn) {
    clearBtn.addEventListener("click", () => {
      const tab = getActiveTab();
      if (tab) {
        tab.searchQuery = "";
        saveCurrentSession();
      }
      if (searchInput) {
        searchInput.value = "";
        searchInput.focus();
      }
      UpdateHistory();
    });
  }
}
