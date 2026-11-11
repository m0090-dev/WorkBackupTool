/**
 * コンパクト画面(幅 320px)の描画と、ウィンドウ高さの自動調整。
 * 高さは中身に合わせて変わる(通常表示は小さく、履歴・パネル・ダイアログを開くと伸びる)。
 */
import { SetWindowHeight } from "../tauri_exports";
import { i18n, tabs, getActiveTab } from "../state";
import { escapeHtml } from "./html.js";
import { closePopovers } from "./popover.js";

const baseName = (path) => (path ? path.split(/[\\/]/).pop() : "");

/** ファイル名・タブ一覧など、コンパクト画面の状態をアクティブタブに合わせる */
export function UpdateCompact() {
  const tab = getActiveTab();
  if (!tab || !i18n) return;

  const name = document.getElementById("compact-filename");
  if (name) name.textContent = baseName(tab.workFile) || i18n.selectedWorkFile;
  const btn = document.getElementById("compact-target-btn");
  if (btn) btn.title = tab.workFile || "";

  const list = document.getElementById("compact-tab-list");
  if (list) {
    list.innerHTML = tabs
      .map((t) => {
        const label = escapeHtml(baseName(t.workFile) || i18n.selectedWorkFile);
        return `<div class="compact-tab-item${t.active ? " on" : ""}" data-tab-id="${t.id}" title="${escapeHtml(t.workFile)}" data-close>${label}</div>`;
      })
      .join("");
  }
}

/** 「履歴」ボタン/「戻る」ボタンでの表示切り替え */
export function setCompactHistoryMode(on) {
  const view = document.getElementById("compact-view");
  if (view) view.classList.toggle("history-mode", on);
  closePopovers();
}

// --- ウィンドウ高さの自動調整 ---
let lastHeight = 0;
let scheduled = false;

function measure() {
  const view = document.getElementById("compact-view");
  if (!view || !document.body.classList.contains("compact-mode")) return 0;

  let height = 0;
  for (const child of view.children) {
    const style = window.getComputedStyle(child);
    if (style.display === "none" || style.visibility === "hidden") continue;
    height += child.getBoundingClientRect().height;
  }
  if (height < 1) height = view.scrollHeight;
  height = Math.ceil(height);

  document
    .querySelectorAll(".dialog-overlay:not(.hidden) .dialog")
    .forEach((d) => {
      height = Math.max(height, d.offsetHeight + 20);
    });
  return height;
}

export function syncCompactHeight() {
  if (scheduled) return;
  scheduled = true;
  const run = () => {
    if (!scheduled) return;
    scheduled = false;
    const height = measure();
    // 2px 未満の差は無視（無駄なリサイズを減らす）
    if (height > 0 && Math.abs(height - lastHeight) > 2) {
      lastHeight = height;
      SetWindowHeight(height).catch(() => {});
    }
  };
  requestAnimationFrame(run);
  setTimeout(run, 80); // 保険は短めに1回
}

/**
 * コンパクト切替・表示時に高さを送り直す。
 * トレイは Rust 側で近い初期サイズにしているので、再送は最小限にする。
 */
export function resetCompactHeight() {
  lastHeight = 0;
  syncCompactHeight();
  // 1回だけ（連打しない）
  setTimeout(() => {
    lastHeight = 0;
    syncCompactHeight();
  }, 120);
}

export function initCompactAutoSize() {
  const view = document.getElementById("compact-view");
  if (!view) return;
  if (typeof ResizeObserver !== "undefined") {
    new ResizeObserver(syncCompactHeight).observe(view);
  }
  // focus のたびにフルリセットするとトレイ再表示でちらつくので、軽く同期だけ
  window.addEventListener("focus", () => {
    if (document.body.classList.contains("compact-mode")) {
      syncCompactHeight();
    }
  });
  document.addEventListener("visibilitychange", () => {
    if (
      document.visibilityState === "visible" &&
      document.body.classList.contains("compact-mode")
    ) {
      resetCompactHeight();
    }
  });
  new MutationObserver(syncCompactHeight).observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ["class"],
  });
}
