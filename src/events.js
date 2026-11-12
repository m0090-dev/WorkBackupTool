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
  QuitApp,
  ShowAbout,
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
  getSelectedPaths,
  selectHistoryRow,
  initPopovers,
  closePopovers,
  setCompactHistoryMode,
  resetCompactHeight,
  initCompactAutoSize,
  notifyResult,
  UpdateAllUI,
} from "./ui";

import { addTab, OnExecute, switchTab } from "./actions";
import { ask } from "@tauri-apps/plugin-dialog";

import { showMemoDialog, parseNoteContent, serializeNote } from "./memo.js";

const preventDefault = (e) => {
  e.preventDefault();
  e.stopPropagation();
};

export async function setupGlobalEvents() {
  window.addEventListener("dragenter", preventDefault, true);
  initPopovers();
  initCompactAutoSize();

  // 直近に選んだ作業対象の種別(トレイメニューからの変更時に使う)
  let lastWorkTargetType = "file";

  // ---- 作業ファイル/フォルダ選択 ----
  // typeOverride: "file" | "folder"(通常画面のメニューから)。未指定ならコンパクト画面の選択か直近の種別
  const handleSelectWorkFile = async (typeOverride) => {
    const tab = getActiveTab();
    const targetType = typeOverride || lastWorkTargetType;
    lastWorkTargetType = targetType;

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
      tab.viewGeneration = null;
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
      tab.viewGeneration = null;

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

  // 履歴のメモ編集(ツールバーの「メモ」/行のダブルクリック)
  const openNoteEditor = async (path) => {
    const notePath = path + ".note";
    // 見出しに出すバックアップの時刻(履歴の行の「MM/DD HH:mm」の時刻部分)
    const row = Array.from(
      document.querySelectorAll("#diff-history-list tr.r"),
    ).find((tr) => tr.getAttribute("data-path") === path);
    const time = (row?.getAttribute("data-time") || "").split(" ").pop();
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
      { time },
    );
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

    // 世代メニューの項目(履歴に表示する世代を切り替える)
    const genItem = e.target.closest(".gen-item");
    if (genItem) {
      tab.viewGeneration = Number(genItem.getAttribute("data-gen"));
      saveCurrentSession();
      UpdateHistory();
      return;
    }

    // 履歴の行(Ctrl/⌘ クリックで複数選択)
    const historyRow = e.target.closest(
      "#diff-history-list tr.r, #compact-history-list tr.r",
    );
    if (historyRow) {
      selectHistoryRow(
        historyRow.getAttribute("data-path"),
        e.ctrlKey || e.metaKey,
      );
      return;
    }

    // 作業対象メニュー(ファイル/フォルダを選ぶ)
    const actionEl = e.target.closest("[data-action]");
    if (actionEl) {
      const action = actionEl.getAttribute("data-action");
      if (action === "pick-file") await handleSelectWorkFile("file");
      else if (action === "pick-folder") await handleSelectWorkFile("folder");
      return;
    }

    // ツールバーの「メモ」
    if (id === "note-btn" || id === "compact-note-btn") {
      const paths = getSelectedPaths();
      if (paths.length !== 1) {
        showFloatingError(i18n.selectHistoryFirst);
        return;
      }
      await openNoteEditor(paths[0]);
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

    // コンパクト画面: タブの切り替え / 履歴表示の切り替え
    const compactTab = e.target.closest(".compact-tab-item");
    if (compactTab) {
      await switchTab(Number(compactTab.getAttribute("data-tab-id")));
      return;
    }
    if (id === "compact-history-btn") {
      setCompactHistoryMode(true);
      return;
    }
    if (id === "compact-back-btn") {
      setCompactHistoryMode(false);
      return;
    }

    if (id === "backupdir-btn") {
      await handleSelectBackupDir();
      return;
    } else if (id === "execute-backup-btn" || id === "compact-execute-btn") {
      if (target.disabled) return;
      await OnExecute();
      return;
    } else if (id === "refresh-diff-btn") {
      UpdateHistory();
      return;
    } else if (id === "apply-selected-btn" || id === "compact-restore-btn") {
      e.preventDefault();
      e.stopPropagation();
      const targets = getSelectedPaths();
      if (targets.length === 0) return;

      const isConfirmed = await ask(i18n.restoreConfirm, {
        title: "WorkBackupTool",
        type: "warning",
      });

      if (isConfirmed) {
        toggleProgress(true, "Restoring...");
        try {
          for (const p of targets) {
            await RestoreBackup(p, tab.workFile);
          }
          toggleProgress(false);
          const notified = await notifyResult(true, i18n.diffApplySuccess);
          if (!notified) showFloatingMessage(i18n.diffApplySuccess);
          UpdateHistory();
        } catch (err) {
          toggleProgress(false);
          const message = `${i18n.restoreError}: ${err}`;
          const notified = await notifyResult(false, message);
          if (notified) showFloatingError(message);
          else alert(err);
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

    if (id === "lock-mode-check" || id === "compact-lock-mode-check") {
      if (!tab) return;
      tab.isLocked = target.checked;
      await UpdateDisplay();
      saveCurrentSession();
    }

    if (id === "settings-close-btn") {
      document.getElementById("settings-modal").classList.add("hidden");
    } else if (id === "settings-modal") {
      if (e.target.id === "settings-modal") e.target.classList.add("hidden");
    }
  });

  // 履歴の行をダブルクリックでメモ編集
  window.addEventListener("dblclick", async (e) => {
    const row = e.target.closest(
      "#diff-history-list tr.r, #compact-history-list tr.r",
    );
    if (!row) return;
    const path = row.getAttribute("data-path");
    selectHistoryRow(path, false);
    await openNoteEditor(path);
  });

  // --- 変更イベントリスナー ---
  document.addEventListener("change", async (e) => {
    // コンパクト画面の設定部品は、通常画面と同じ扱いにそろえる("compact-" を外す)
    const id = (e.target.id || "").replace(/^compact-/, "");
    const name =
      e.target.name === "compactBackupMode" ? "backupMode" : e.target.name;
    const value = e.target.value;
    const tab = getActiveTab();

    if (name == "diff-algo") {
      if (tab) tab.diffAlgo = value;
    }
    if (name === "backupMode") {
      if (tab) tab.backupMode = value;
    }
    if (id === "hdiff-compress") {
      if (tab) tab.compressMode = value;
    }
    if (id == "archive-format") {
      if (tab) tab.archiveFormat = value;
    }
    if (
      ["backupMode", "archive-format"].includes(name) ||
      id === "archive-format" ||
      id === "diff-algo" ||
      id === "hdiff-compress"
    ) {
      await UpdateDisplay();
      saveCurrentSession();
    }
    if (id == "archive-select-all-check") {
      const checks = document.querySelectorAll(".archive-gen-check");
      checks.forEach((c) => (c.checked = e.target.checked));
    }
  });

  // ZIPパスワード欄は通常画面とコンパクト画面で同じ値を共有する
  document.addEventListener("input", (e) => {
    const pair = {
      "archive-password": "compact-archive-password",
      "compact-archive-password": "archive-password",
    }[e.target.id];
    const other = pair && document.getElementById(pair);
    if (other) other.value = e.target.value;
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
  // トレイからの実行も、通常の実行と同じ OnExecute を通す。
  // 完了の通知(設定に従う)は OnExecute の中で通知基盤に渡すので、ここでは何もしない。
  EventsOn("tray-execute-clicked", async () => {
    await OnExecute();
  });

  EventsOn("tray-change-work-clicked", () => {
    handleSelectWorkFile();
  });
  EventsOn("tray-change-backup-clicked", () => {
    handleSelectBackupDir();
  });

  EventsOn("tray-mode-change", async (newMode) => {
    const tab = getActiveTab();
    const radio = document.querySelector(
      `input[name="backupMode"][value="${newMode}"]`,
    );
    if (radio && !tab.isLocked) {
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
      await UpdateAllUI();
      resetCompactHeight();
    } else {
      document.body.classList.remove("compact-mode");
      document.body.classList.remove("frameless");
      setCompactHistoryMode(false);
      if (view) view.classList.add("hidden");
    }
  });

  // トレイから開いた枠なしウィンドウでは、移動用のバーを出す
  EventsOn("compact-frameless", (isFrameless) => {
    document.body.classList.toggle("frameless", !!isFrameless);
    resetCompactHeight();
  });

  EventsOn("open-advanced-settings", async () => {
    await showSettingsModal();
  });

  // ファイルメニュー「世代の圧縮」
  EventsOn("open-generation-archive", () => {
    const tab = getActiveTab();
    if (!tab || !tab.workFile) {
      showFloatingError(
        i18n?.selectFileFirst || "Please select a work file first.",
      );
      return;
    }
    showArchiveModal();
  });

  // メニューの Ctrl+Q / F1 相当（このウィンドウにフォーカスがあるときだけ。他アプリには効かない）
  window.addEventListener(
    "keydown",
    (e) => {
      const el = e.target;
      const typing =
        el &&
        (el.tagName === "INPUT" ||
          el.tagName === "TEXTAREA" ||
          el.tagName === "SELECT" ||
          el.isContentEditable);
      if (typing) return;

      // Ctrl+Q → 終了
      if (
        e.ctrlKey &&
        !e.altKey &&
        !e.shiftKey &&
        !e.metaKey &&
        e.code === "KeyQ"
      ) {
        e.preventDefault();
        QuitApp();
        return;
      }

      // F1 → About
      if (
        !e.ctrlKey &&
        !e.altKey &&
        !e.shiftKey &&
        !e.metaKey &&
        e.key === "F1"
      ) {
        e.preventDefault();
        ShowAbout();
      }
    },
    true,
  );

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
