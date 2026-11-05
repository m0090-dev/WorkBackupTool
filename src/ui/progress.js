import { i18n } from "../state";

let isExecuting = false;

/** バックアップ/復元などの処理中かどうか(履歴更新時に実行ボタンの有効化判定で使う) */
export function isBackupExecuting() {
  return isExecuting;
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
