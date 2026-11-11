import { i18n, tabs } from "../state";
import { escapeHtml } from "./html.js";
import { switchTab, removeTab, reorderTabs } from "../actions";

//TODO: Drag and drop関係はtauri v2が修整されたら正常動作するはず
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

    el.draggable = true;
    el.dataset.id = tab.id;
    el.ondragstart = (e) => {
      if (el._removeTooltip) el._removeTooltip();
      el.classList.add("dragging");
      e.dataTransfer.setData("text/plain", tab.id);
    };
    el.ondragover = (e) => {
      e.preventDefault();
      e.stopPropagation();
      el.classList.add("drag-over");
    };
    el.ondragleave = () => el.classList.remove("drag-over");
    el.ondragend = () => {
      el.classList.remove("dragging");
      list
        .querySelectorAll(".tab-item")
        .forEach((i) => i.classList.remove("drag-over"));
    };
    el.ondrop = async (e) => {
      e.preventDefault();
      const dId = e.dataTransfer.getData("text/plain");
      if (dId && dId !== el.dataset.id) await reorderTabs(dId, el.dataset.id);
    };

    el.onclick = async () => {
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
