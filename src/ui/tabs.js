import { i18n, tabs } from "../state";
import { escapeHtml } from "./html.js";
import { switchTab, removeTab, reorderTabs } from "../actions";

// タブの並び替えは HTML5 DnD ではなく Pointer Events で実装している。
// 理由: Tauri v2 (Windows/WebView2) では dragDropEnabled: true の間、
// WebView2 の IDropTarget を Tauri が差し替えるため HTML5 の
// dragstart/dragover/drop が一切発火しない。一方、アプリ内で始まる
// マウス操作は OLE ドラッグではないので Pointer Events は干渉されない。
// これにより、ウィンドウへのファイル/フォルダドロップ(ネイティブ、
// パス取得)は dragDropEnabled: true のまま両立できる。
export function renderTabs() {
  const list = document.getElementById("tabs-list");
  if (!list) return;

  list.innerHTML = "";

  const clearGlobals = () => {
    const existingMenu = document.querySelector(".tab-context-menu");
    if (existingMenu) existingMenu.remove();
    const existingTooltips = document.querySelectorAll(".tab-tooltip");
    existingTooltips.forEach((t) => t.remove());
  };
  clearGlobals();

  tabs.forEach((tab, index) => {
    const el = document.createElement("div");
    el.className = `tab-item ${tab.active ? "active" : ""}`;

    const fileName = tab.workFile
      ? tab.workFile.split(/[\\/]/).pop()
      : i18n?.selectedWorkFile || "No file selected";
    el.textContent = fileName;

    setupTabTooltip(el, tab);

    el.dataset.id = tab.id;
    el.style.touchAction = "none";
    setupTabPointerDrag(el, list);

    el.onclick = async () => {
      // ドラッグ直後の click でタブ切り替えが走らないようにする
      if (el._suppressNextClick) {
        el._suppressNextClick = false;
        return;
      }
      if (el._removeTooltip) el._removeTooltip();
      await switchTab(tab.id);
    };

    el.oncontextmenu = (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (el._removeTooltip) el._removeTooltip();

      const existingMenu = document.querySelector(".tab-context-menu");
      if (existingMenu) existingMenu.remove();

      const menuItems = [];

      if (index > 0) {
        const item = document.createElement("div");
        item.className = "tab-menu-item";
        item.innerHTML = `<span>${i18n.tabMenuMoveLeft}</span><span class="tab-menu-shortcut">◀</span>`;
        item.onclick = async (ev) => {
          ev.stopPropagation();
          await reorderTabs(tab.id, tabs[index - 1].id);
          menu.remove();
        };
        menuItems.push(item);
      }

      if (index < tabs.length - 1) {
        const item = document.createElement("div");
        item.className = "tab-menu-item";
        item.innerHTML = `<span>${i18n.tabMenuMoveRight}</span><span class="tab-menu-shortcut">▶</span>`;
        item.onclick = async (ev) => {
          ev.stopPropagation();
          await reorderTabs(tab.id, tabs[index + 1].id);
          menu.remove();
        };
        menuItems.push(item);
      }

      if (tabs.length > 1) {
        const sep = document.createElement("div");
        sep.className = "tab-menu-separator";
        menuItems.push(sep);

        const del = document.createElement("div");
        del.className = "tab-menu-item danger";
        del.innerHTML = `<span>${i18n.tabMenuClose}</span><span class="tab-menu-shortcut">×</span>`;
        del.onclick = async (ev) => {
          ev.stopPropagation();
          await removeTab(tab.id);
          menu.remove();
        };
        menuItems.push(del);
      }

      if (menuItems.length === 0) return;

      const menu = document.createElement("div");
      menu.className = "tab-context-menu";
      menuItems.forEach((item) => menu.appendChild(item));

      document.body.appendChild(menu);

      const menuRect = menu.getBoundingClientRect();
      let left = e.clientX;
      let top = e.clientY;
      if (left + menuRect.width > window.innerWidth) left -= menuRect.width;
      if (top + menuRect.height > window.innerHeight) top -= menuRect.height;

      menu.style.left = `${left}px`;
      menu.style.top = `${top}px`;

      const closeMenu = (ev) => {
        if (!menu.contains(ev.target)) {
          menu.remove();
          document.removeEventListener("mousedown", closeMenu);
        }
      };
      setTimeout(() => document.addEventListener("mousedown", closeMenu), 50);
    };

    list.appendChild(el);
  });
}


// ドラッグ開始とみなす移動量(px)。これ未満の動きは通常のクリック(タブ切り替え)
const TAB_DRAG_THRESHOLD = 6;

function setupTabPointerDrag(el, list) {
  el.addEventListener("pointerdown", (e) => {
    // 左ボタンのみ対象。右ボタンはコンテキストメニューに譲る
    if (e.button !== 0) return;
    // タブが1つだけなら並び替える意味がない
    if (tabs.length < 2) return;

    const pointerId = e.pointerId;
    const startX = e.clientX;
    const startY = e.clientY;
    let dragging = false;
    let overEl = null;
    // ドラッグ開始時点での他タブの矩形。ドラッグ中はレイアウトが変わらないので使い回せる。
    // ドラッグ中の要素は transform で動くため、elementFromPoint では判定できないことへの対策でもある
    let otherRects = null;

    const clearOver = () => {
      if (overEl) overEl.classList.remove("drag-over");
      overEl = null;
    };

    const onMove = (ev) => {
      if (ev.pointerId !== pointerId) return;
      if (!dragging) {
        if (
          Math.hypot(ev.clientX - startX, ev.clientY - startY) <
          TAB_DRAG_THRESHOLD
        )
          return;
        dragging = true;
        if (el._removeTooltip) el._removeTooltip();
        el.classList.add("dragging");
        el.style.zIndex = "10";
        el.style.cursor = "grabbing";
        document.body.style.userSelect = "none";
        otherRects = [...list.querySelectorAll(".tab-item")]
          .filter((i) => i !== el)
          .map((i) => ({ el: i, rect: i.getBoundingClientRect() }));
      }
      el.style.transform = `translateX(${ev.clientX - startX}px)`;
      const hit = otherRects.find(
        ({ rect }) =>
          ev.clientX >= rect.left &&
          ev.clientX <= rect.right &&
          ev.clientY >= rect.top - 10 &&
          ev.clientY <= rect.bottom + 10,
      );
      const next = hit ? hit.el : null;
      if (next !== overEl) {
        clearOver();
        if (next) {
          next.classList.add("drag-over");
          overEl = next;
        }
      }
    };

    const cleanup = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onCancel);
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("blur", onCancel);
      el.classList.remove("dragging");
      el.style.transform = "";
      el.style.zIndex = "";
      el.style.cursor = "";
      document.body.style.userSelect = "";
      clearOver();
    };

    const suppressClick = () => {
      el._suppressNextClick = true;
      setTimeout(() => (el._suppressNextClick = false), 200);
    };

    const onUp = async (ev) => {
      if (ev.pointerId !== pointerId) return;
      const target = dragging ? overEl : null;
      const wasDragging = dragging;
      cleanup();
      if (!wasDragging) return;
      suppressClick();
      if (target && target.dataset.id !== el.dataset.id) {
        // 並び替えの意味は従来の HTML5 版 ondrop と同じ
        // (reorderTabs: ドロップ先タブの位置へ挿入)
        await reorderTabs(el.dataset.id, target.dataset.id);
      }
    };

    const onCancel = () => {
      if (dragging) suppressClick();
      cleanup();
    };

    const onKeyDown = (ev) => {
      if (ev.key === "Escape") onCancel();
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onCancel);
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("blur", onCancel);
  });
}

function setupTabTooltip(el, tab) {
  if (!el || !tab || !i18n) return;

  let tooltip = null;

  const removeTooltip = () => {
    if (tooltip) {
      tooltip.remove();
      tooltip = null;
    }
  };

  el.addEventListener("mouseenter", () => {
    if (document.querySelector(".tab-context-menu")) return;

    removeTooltip();

    const fileName = tab.workFile
      ? tab.workFile.split(/[\\/]/).pop()
      : i18n?.selectedWorkFile || "No file selected";
    const workPath =
      tab.workFile || i18n?.selectedWorkFile || "No file selected";
    const backupPath =
      tab.backupDir || i18n?.selectedBackupDir || "Default location";
    const mode = tab.backupMode || "diff";

    function formatFileSize(bytes) {
      if (bytes < 1024) return bytes + " B";

      const units = ["KB", "MB", "GB", "TB"];
      let size = bytes;
      let index = -1;

      do {
        size /= 1024;
        index++;
      } while (size >= 1024 && index < units.length - 1);

      return size.toFixed(2) + " " + units[index];
    }
    const workSizeMB = formatFileSize(tab.workFileSize);
    tooltip = document.createElement("div");
    tooltip.className = "tab-tooltip";
    tooltip.innerHTML = `
      <span class="tooltip-title">${escapeHtml(fileName)}</span>
      <b>${escapeHtml(i18n.labelWorkFile)}</b> <code>${escapeHtml(workPath)}(${escapeHtml(workSizeMB)})</code>
      <b>${escapeHtml(i18n.labelLocation)}</b> <code>${escapeHtml(backupPath)}</code>
      <b>${escapeHtml(i18n.backupMode)}:</b> <span class="tooltip-mode">${escapeHtml(mode.toUpperCase())}</span>
    `;

    document.body.appendChild(tooltip);

    const rect = el.getBoundingClientRect();
    tooltip.style.left = `${rect.left}px`;
    tooltip.style.top = `${rect.bottom + 5}px`;
  });

  el.addEventListener("mouseleave", removeTooltip);
  el.addEventListener("mousedown", removeTooltip);

  el._removeTooltip = removeTooltip;
}
