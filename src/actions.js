import {
  CopyBackupFile,
  ArchiveBackupFile,
  BackupOrDiff,
  RestoreBackup,
  GetFileSize,
  DirExists,
  GetShowMemoAfterBackup,
  GetBackupList,
  ReadTextFile,
  WriteTextFile,
} from "./tauri_exports";

import {
  i18n,
  tabs,
  getActiveTab,
  addToRecentFiles,
  saveCurrentSession,
} from "./state";

import {
  renderTabs,
  UpdateDisplay,
  UpdateHistory,
  toggleProgress,
  showFloatingMessage,
  UpdateAllUI,
} from "./ui";
import { showMemoDialog } from "./memo.js";

// --- タブ操作ロジック ---
export async function switchTab(id) {
  tabs.forEach((t) => (t.active = t.id == id));
  // DEBUG
  const activeTab = tabs.find((t) => t.active);
  console.log(
    "DEBUG JS Switched to:",
    activeTab ? activeTab.id : "NONE",
    activeTab?.workFile,
  );
  await UpdateAllUI();
  saveCurrentSession();
}

export async function addTab() {
  tabs.forEach((t) => (t.active = false));
  tabs.push({
    id: Date.now(),
    workFile: "",
    workFileSize: 0,
    backupDir: "",
    active: true,
    backupMode: "diff",
    compressMode: "zstd",
  });
  await UpdateAllUI();
  saveCurrentSession();
}

export async function removeTab(id) {
  const index = tabs.findIndex((t) => t.id === id);
  const wasActive = tabs[index].active;
  tabs.splice(index, 1);
  if (wasActive) tabs[Math.max(0, index - 1)].active = true;
  await UpdateAllUI();
  saveCurrentSession();
}

export async function reorderTabs(draggedId, targetId) {
  // 文字列IDを比較するために型を合わせる（念のため）
  const draggedIndex = tabs.findIndex(
    (t) => String(t.id) === String(draggedId),
  );
  const targetIndex = tabs.findIndex((t) => String(t.id) === String(targetId));

  if (
    draggedIndex !== -1 &&
    targetIndex !== -1 &&
    draggedIndex !== targetIndex
  ) {
    // 1. 配列のコピーを作成して操作する（リアクティブな問題を避けるため）
    const [removed] = tabs.splice(draggedIndex, 1);
    tabs.splice(targetIndex, 0, removed);

    // 2. セッションを強制保存（ここで localStorage などに書き込まれる）
    saveCurrentSession();

    await UpdateAllUI();
  }
}

export async function OnExecute() {
  const tab = getActiveTab();
  if (!tab?.workFile) {
    alert(i18n.selectFileFirst);
    return;
  }
  const mode = tab.backupMode;
  // --- 2. 差分設定の取得 (既存ロジック維持 + 圧縮設定追加) ---
  let algo = tab.diffAlgo || "hdiff";
  const compress = tab.compressMode || "zstd";
  const archiveFormat = tab.archiveFormat || "zip";
  const pwdEl = document.getElementById("archive-password");
  const pwdValue = pwdEl ? pwdEl.value : "";
  await UpdateAllUI();
  toggleProgress(true, i18n.processingMsg);
  try {
    let successText = "";

    // --- A. 単純コピーモード ---
    if (mode === "copy") {
      await CopyBackupFile(tab.workFile, tab.backupDir);
      successText = i18n.copyBackupSuccess;
    }
    // --- B. アーカイブモード ---
    else if (mode === "archive") {
      let fmt = archiveFormat;
      let pwd = fmt === "zip-pass" ? pwdValue : "";
      if (fmt === "zip-pass") fmt = "zip";
      await ArchiveBackupFile(tab.workFile, tab.backupDir, fmt, pwd);
      successText = i18n.archiveBackupSuccess.replace(
        "{format}",
        fmt.toUpperCase(),
      );
    }
    // --- C. 差分バックアップモード (既存ロジック完全維持 + 引数拡張) ---
    else if (mode === "diff") {
      console.log("DEBUG JS: tab.selectedTargetDir =", tab.selectedTargetDir);
      console.log("DEBUG JS: tab.backupDir =", tab.backupDir);
      // フォルダの存在確認ロジック
      if (tab.selectedTargetDir) {
        const exists = await DirExists(tab.selectedTargetDir);
        if (!exists) {
          console.log(
            "Selected directory no longer exists. Reverting to auto-discovery.",
          );
          tab.selectedTargetDir = "";
        }
      }

      const targetPath = tab.selectedTargetDir || tab.backupDir || "";
      console.log("DEBUG JS: Final targetPath sent to Rust =", targetPath);

      // Rust側(またはGo側)の関数を呼び出し
      // 引数に新しく compress を追加。algoがbsdiffの場合は内部で無視される設計
      await BackupOrDiff(tab.workFile, targetPath, algo, compress);

      successText = `${i18n.diffBackupSuccess} (${algo.toUpperCase()}${algo === "hdiff" ? ":" + compress : ""})`;
    }

    toggleProgress(false);
    showFloatingMessage(successText);

    // メモダイアログをオプションで表示
    const showMemo = await GetShowMemoAfterBackup();
    if (showMemo) {
      const data = await GetBackupList(tab.workFile, tab.backupDir);
      if (data && data.length > 0) {
        data.sort((a, b) => b.fileName.localeCompare(a.fileName));
        const latest = data[0];
        const notePath = latest.filePath + ".note";
        const currentNote = await ReadTextFile(notePath).catch(() => "");
        showMemoDialog(currentNote, async (newText) => {
          await WriteTextFile(notePath, newText);
          UpdateHistory();
        });
      }
    }
    await UpdateAllUI();
    return successText;
  } catch (err) {
    toggleProgress(false);
    alert(err);
    return null;
  }
}

// --- 復元・適用ロジック ---
export async function applySelectedBackups() {
  const tab = getActiveTab();
  const targets = Array.from(
    document.querySelectorAll(".diff-checkbox:checked"),
  ).map((el) => el.value);
  if (targets.length > 0 && confirm(i18n.restoreConfirm)) {
    toggleProgress(true, "Restoring...");
    try {
      for (const p of targets) {
        await RestoreBackup(p, tab.workFile);
      }
      toggleProgress(false);
      showFloatingMessage(i18n.diffApplySuccess);
      await UpdateAllUI();
    } catch (err) {
      toggleProgress(false);
      alert(err);
    }
  }
}
