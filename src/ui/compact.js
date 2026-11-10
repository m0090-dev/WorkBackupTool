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
  let height = Math.ceil(view.getBoundingClientRect().height);
  // ダイアログが開いているときは、ダイアログが収まる高さを確保する
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
    if (!scheduled) return; // rAF とタイマーのどちらか早い方だけ実行する
    scheduled = false;
    const height = measure();
    if (height > 0 && Math.abs(height - lastHeight) > 1) {
      lastHeight = height;
      SetWindowHeight(height).catch(() => {});
    }
  };
  requestAnimationFrame(run);
  // ウィンドウが非表示の間は rAF が止まるため、タイマーでも保険をかける
  setTimeout(run, 120);
}

/**
 * コンパクト表示に切り替わったとき/ウィンドウが表示されたときに、高さを必ず送り直す。
 * (トレイから開く枠なしウィンドウは、Rust側が開くたびに初期サイズへ戻すため、
 *  前回と同じ高さでも再送しないと、下に余白が残ることがある)
 */
export function resetCompactHeight() {
  lastHeight = 0;
  syncCompactHeight();
  // 表示が完了してからもう一度(表示直後はレイアウトが落ち着いていないことがある)
  [150, 450].forEach((ms) =>
    setTimeout(() => {
      lastHeight = 0;
      syncCompactHeight();
    }, ms),
  );
}

export function initCompactAutoSize() {
  const view = document.getElementById("compact-view");
  if (!view) return;
  if (typeof ResizeObserver !== "undefined") {
    new ResizeObserver(syncCompactHeight).observe(view);
  }
  // ウィンドウが再び表示/フォーカスされたとき(トレイから開き直したとき)も送り直す
  window.addEventListener("focus", resetCompactHeight);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") resetCompactHeight();
  });
  // ダイアログの開閉(クラス/要素の変化)でも再計算する
  new MutationObserver(syncCompactHeight).observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ["class"],
  });
}
