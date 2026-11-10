import { FileExists, DirExists } from "../tauri_exports";
import { i18n, getActiveTab, formatSize } from "../state";
import { UpdateCompact } from "./compact.js";

const $ = (id) => document.getElementById(id);

/**
 * バックアップモードの設定パネル(通常画面 pfx="" / コンパクト画面 pfx="compact-")を
 * アクティブタブの状態に合わせる。
 */
function applyModeControls(pfx, tab) {
  const mode = tab.backupMode || "diff";
  const locked = tab.isLocked || false;
  const compress = tab.compressMode || "zstd";
  const archiveFormat = tab.archiveFormat || "zip";
  const radioName = pfx ? "compactBackupMode" : "backupMode";

  document.querySelectorAll(`input[name="${radioName}"]`).forEach((r) => {
    r.checked = r.value === mode;
    r.disabled = locked;
  });

  const algo = $(`${pfx}diff-algo`);
  if (algo) algo.value = tab.diffAlgo || "hdiff";
  const comp = $(`${pfx}hdiff-compress`);
  if (comp) {
    comp.value = compress;
    comp.disabled = locked;
  }
  const format = $(`${pfx}archive-format`);
  if (format) format.value = archiveFormat;

  // ロック(パネル内のチェックボックス)
  const lockCheck = $(`${pfx}lock-mode-check`);
  if (lockCheck) lockCheck.checked = locked;
  const lockHint = $(`${pfx}lock-mode-hint`);
  if (lockHint) {
    lockHint.textContent = locked ? i18n.lockHintOn : i18n.lockHintOff;
  }

  const panel = $(`${pfx}mode-panel`);
  if (panel) panel.dataset.mode = mode;

  // パスワード欄は「ZIP (Password)」のときだけ表示
  const isPass = mode === "archive" && archiveFormat === "zip-pass";
  const pwdArea = $(`${pfx}password-area`);
  if (pwdArea) {
    pwdArea.style.display = isPass ? "grid" : "none";
    const pwd = $(`${pfx}archive-password`);
    if (pwd) pwd.disabled = !isPass;
  }
}

/** モードの表示名(ツールバー/ステータスの要約) */
function modeSummary(tab) {
  const mode = tab.backupMode || "diff";
  const titles = {
    copy: i18n.fullCopyTitle,
    archive: i18n.archiveTitle,
    diff: i18n.diffTitle,
  };
  const detail =
    mode === "diff"
      ? tab.compressMode || "zstd"
      : mode === "archive"
        ? { zip: "ZIP", "zip-pass": "ZIP / Password", tar: "TAR.GZ" }[
            tab.archiveFormat || "zip"
          ]
        : "";
  return detail ? `${titles[mode]} / ${detail}` : titles[mode];
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

  const fileEl = $("selected-workfile");
  const dirEl = $("selected-backupdir");
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

  // モード設定(通常画面・コンパクト画面)
  applyModeControls("", tab);
  applyModeControls("compact-", tab);
  const summary = modeSummary(tab);
  ["mode-summary", "compact-mode-summary"].forEach((id) => {
    const el = $(id);
    if (el) el.textContent = summary;
  });
  const cMode = $("compact-history-mode");
  if (cMode) cMode.textContent = summary;

  UpdateCompact();
}
