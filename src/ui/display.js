import { FileExists, DirExists } from "../tauri_exports";
import { i18n, tabs, getActiveTab, formatSize } from "../state";

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
    const fullPath = tab.workFile || i18n.selectedWorkFile;
    const sizeText = tab.workFile ? ` (${formatSize(tab.workFileSize)})` : "";
    const missing = !workTargetExists && tab.workFile;
    fileEl.textContent = missing
      ? `${fullPath} (Not Found)`
      : `${fullPath}${sizeText}`;
    fileEl.title = tab.workFile || "";
    fileEl.classList.toggle("missing", !!missing);
  }
  if (dirEl) {
    const dirText = tab.backupDir || i18n.selectedBackupDir;
    const missing = !dirExists && tab.backupDir;
    dirEl.textContent = missing ? `${dirText} (Not Found)` : dirText;
    dirEl.title = tab.backupDir || "";
    dirEl.classList.toggle("missing", !!missing);
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
    lockBtn.textContent = locked ? i18n.unlockShort : i18n.lockShort;
    lockBtn.title = locked ? i18n.unlockMode : i18n.lockMode;
  }
  const isPass =
    mode === "archive" &&
    document.getElementById("archive-format")?.value === "zip-pass";
  // ツールバーのモード表示と、モード設定パネルの表示切替
  const modeBtn = document.getElementById("mode-btn");
  if (modeBtn) {
    const modeTitles = {
      copy: i18n.fullCopyTitle,
      archive: i18n.archiveTitle,
      diff: i18n.diffTitle,
    };
    const detail =
      mode === "diff"
        ? compress
        : mode === "archive"
          ? { zip: "ZIP", "zip-pass": "ZIP+PW", tar: "TAR.GZ" }[archiveFormat]
          : "";
    modeBtn.textContent = detail
      ? `${modeTitles[mode]} / ${detail}`
      : modeTitles[mode];
    modeBtn.title = i18n.modeButtonTip || "";
  }
  const modePanel = document.getElementById("mode-panel");
  if (modePanel) modePanel.dataset.mode = mode;
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
