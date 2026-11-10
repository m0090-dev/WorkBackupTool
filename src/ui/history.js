import { GetBackupList, ReadTextFile } from "../tauri_exports";
import { i18n, getActiveTab, formatSize } from "../state";
import { parseNoteContent } from "../memo.js";
import { isBackupExecuting, setCompactIdleStatus } from "./progress.js";
import { escapeHtml, escapeRegExp } from "./html.js";

// --- 選択状態(履歴テーブルの行) ---
const selected = new Set();
let selectionKey = "";

export function getSelectedPaths() {
  return Array.from(selected);
}

/** "2026-11-02 04:52:11" -> "11/02 04:52" */
function shortTime(ts) {
  const m = /^\d{4}-(\d{2})-(\d{2}) (\d{2}):(\d{2})/.exec(ts || "");
  return m ? `${m[1]}/${m[2]} ${m[3]}:${m[4]}` : ts || "";
}

function fillTemplate(text, values) {
  return Object.entries(values).reduce(
    (acc, [k, v]) => acc.replace(`{${k}}`, v),
    text || "",
  );
}

/** 選択状態に合わせて「復元」ボタンとステータスバーの表示を更新する */
export function updateSelectionUI() {
  const restoreBtn = document.getElementById("apply-selected-btn");
  const status = document.getElementById("selection-status");
  if (!i18n) return;
  const rows = Array.from(
    document.querySelectorAll("#diff-history-list tr.r.sel"),
  );
  const count = selected.size;
  const time = rows.length === 1 ? rows[0].dataset.time : "";

  let restoreText = i18n.restoreBtn;
  let statusText = i18n.statusNoSelection;
  if (count === 1) {
    restoreText = fillTemplate(i18n.restoreBtnWith, { time });
    statusText = fillTemplate(i18n.statusSelected, { time });
  } else if (count > 1) {
    restoreText = fillTemplate(i18n.restoreBtnCount, { count });
    statusText = fillTemplate(i18n.statusSelected, { time: count });
  }
  if (restoreBtn) {
    restoreBtn.textContent = restoreText;
    restoreBtn.disabled = count === 0;
  }
  if (status) status.textContent = statusText;

  // コンパクト画面の履歴表示
  const cRestore = document.getElementById("compact-restore-btn");
  if (cRestore) cRestore.disabled = count === 0;
  const cStatus = document.getElementById("compact-selection-status");
  if (cStatus) cStatus.textContent = statusText;
}

/** 行をクリックしたときの選択処理。additive=true(Ctrl/⌘クリック)で複数選択 */
export function selectHistoryRow(path, additive = false) {
  if (additive) {
    if (selected.has(path)) selected.delete(path);
    else selected.add(path);
  } else {
    selected.clear();
    selected.add(path);
  }
  document
    .querySelectorAll("#diff-history-list tr.r, #compact-history-list tr.r")
    .forEach((tr) => {
      tr.classList.toggle("sel", selected.has(tr.dataset.path));
    });
  updateSelectionUI();
}

function setMeta({ count = 0, genText = "", size = 0, name = "" }) {
  const countEl = document.getElementById("history-count");
  const genBtn = document.getElementById("gen-btn");
  const sizeEl = document.getElementById("history-size");
  const countText = fillTemplate(i18n.statusCount, { count });
  if (countEl) countEl.textContent = countText;
  if (genBtn) genBtn.textContent = genText;
  if (sizeEl) sizeEl.textContent = formatSize(size);

  // コンパクト画面: 「ファイル名 · N 件 · サイズ」+ 世代ボタン
  const cInfo = document.getElementById("compact-history-info");
  if (cInfo) {
    cInfo.textContent = [name, countText, formatSize(size)]
      .filter(Boolean)
      .join(" · ");
  }
  const cGen = document.getElementById("compact-gen-btn");
  if (cGen) cGen.textContent = genText;
}

/** 履歴の行を通常画面とコンパクト画面の両方に反映する */
function setRows(html) {
  const list = document.getElementById("diff-history-list");
  const cList = document.getElementById("compact-history-list");
  if (list) list.innerHTML = html;
  if (cList) cList.innerHTML = html;
}

function messageRow(text, isError = false) {
  return `<tr><td colspan="4" class="info-msg${isError ? " error" : ""}">${escapeHtml(text)}</td></tr>`;
}

export async function UpdateHistory() {
  const tab = getActiveTab();
  const list = document.getElementById("diff-history-list");
  const searchInput = document.getElementById("history-search");
  const searchTerm = (tab?.searchQuery || "").toLowerCase().trim();
  const clearBtn = document.getElementById("search-clear-btn");
  const executeBtn = document.getElementById("execute-backup-btn");
  const compactExecuteBtn = document.getElementById("compact-execute-btn");
  if (!list || !i18n) return;

  if (searchInput && document.activeElement !== searchInput) {
    searchInput.value = tab?.searchQuery || "";
  }
  if (clearBtn) clearBtn.classList.toggle("visible", searchTerm.length > 0);

  if (!tab?.workFile) {
    setRows(messageRow(i18n.selectFileFirst));
    setCompactIdleStatus(i18n.readyStatus);
    selected.clear();
    setMeta({ genText: i18n.noGeneration });
    updateSelectionUI();
    return;
  }

  try {
    const raw = (await GetBackupList(tab.workFile, tab.backupDir)) || [];

    // メモ(.note)を読み込む
    const items = await Promise.all(
      raw.map(async (item) => {
        const text = await ReadTextFile(item.filePath + ".note").catch(
          () => "",
        );
        const { text: note, meta } = parseNoteContent(text);
        return { ...item, note, mark: meta?.mark ?? 0 };
      }),
    );

    // --- 世代の整理 ---
    // generation = 0 は世代に属さない単体アーカイブ、1以上が世代
    const gens = [...new Set(items.map((i) => i.generation || 0))];
    const realGens = gens.filter((g) => g > 0).sort((a, b) => b - a);
    const latest = realGens.length > 0 ? realGens[0] : null;
    let view = tab.viewGeneration;
    if (view == null || !gens.includes(view)) {
      view = latest ?? (gens.includes(0) ? 0 : null);
    }

    // 世代メニュー
    const genList = document.getElementById("gen-list");
    if (genList) {
      const entries = [
        ...realGens.map((g) => ({
          gen: g,
          label: `${i18n.generationLabel} ${g}`,
          badge:
            g === latest
              ? i18n.genLatest
              : items.some((i) => i.generation === g && i.isArchived)
                ? i18n.genArchived
                : "",
        })),
        ...(gens.includes(0)
          ? [{ gen: 0, label: i18n.genStandalone, badge: "" }]
          : []),
      ];
      const genHtml = entries
        .map(
          (e) =>
            `<div class="gen-item${e.gen === view ? " on" : ""}" data-gen="${e.gen}" data-close>${escapeHtml(e.label)}${e.badge ? `<span class="badge">${escapeHtml(e.badge)}</span>` : ""}</div>`,
        )
        .join("");
      genList.innerHTML = genHtml;
      const cGenList = document.getElementById("compact-gen-list");
      if (cGenList) cGenList.innerHTML = genHtml;
    }

    // 検索中は全世代を対象にする。それ以外は選択中の世代だけ
    let visible = items.filter((i) =>
      searchTerm ? true : (i.generation || 0) === view,
    );
    if (searchTerm) {
      visible = visible.filter(
        (i) =>
          i.fileName.toLowerCase().includes(searchTerm) ||
          i.note.toLowerCase().includes(searchTerm),
      );
    }
    visible.sort(
      (a, b) =>
        b.timestamp.localeCompare(a.timestamp) ||
        b.fileName.localeCompare(a.fileName),
    );

    // --- 選択の保持(タブ/世代が変わったらクリア) ---
    const key = `${tab.id}|${view}`;
    if (key !== selectionKey) {
      selected.clear();
      selectionKey = key;
    }
    const visiblePaths = new Set(visible.map((i) => i.filePath));
    Array.from(selected).forEach((p) => {
      if (!visiblePaths.has(p)) selected.delete(p);
    });

    // --- ステータスバー ---
    let genText = i18n.noGeneration;
    if (view === 0) genText = i18n.genStandalone;
    else if (view != null)
      genText = `${i18n.generationLabel} ${view}${view === latest ? " ★" : ""}`;
    setMeta({
      name: tab.workFile.split(/[\\/]/).pop(),
      count: visible.length,
      genText,
      size: visible.reduce((sum, i) => sum + (i.fileSize || 0), 0),
    });

    // --- 行の描画 ---
    const markLabels = {
      1: i18n.priorityLow,
      2: i18n.priorityMid,
      3: i18n.priorityHigh,
    };
    const markClasses = { 1: "", 2: "mid", 3: "hi" };
    const highlight = (text) => {
      const safe = escapeHtml(text);
      if (!searchTerm) return safe;
      return safe.replace(
        new RegExp(`(${escapeRegExp(escapeHtml(searchTerm))})`, "gi"),
        "<mark>$1</mark>",
      );
    };

    if (visible.length === 0) {
      setRows(messageRow(i18n.noHistory));
    } else {
      setRows(
        visible
          .map((item) => {
            const time = shortTime(item.timestamp);
            const mark = item.mark || 0;
            const markCell =
              mark > 0
                ? `<td class="${markClasses[mark] || ""}">${escapeHtml(markLabels[mark] || "")}</td>`
                : `<td class="muted">-</td>`;
            const firstLine = (item.note || "").split("\n")[0];
            const genLabel =
              (item.generation || 0) === 0
                ? i18n.genStandalone
                : `${i18n.generationLabel} ${item.generation}${item.isArchived ? ` (${i18n.genArchived})` : ""}`;
            const tip =
              `<strong>${escapeHtml(item.fileName)}</strong><br>` +
              `${escapeHtml(item.timestamp)} · ${escapeHtml(genLabel)}<br>` +
              `<strong>Path:</strong> ${escapeHtml(item.filePath)}` +
              (mark > 0
                ? `<br><strong>${escapeHtml(i18n.priorityLabel)}:</strong> ${escapeHtml(markLabels[mark])}`
                : "") +
              (item.note
                ? `<hr><strong>${escapeHtml(i18n.backupMemo)}:</strong> ${escapeHtml(item.note)}`
                : "");
            return `<tr class="r${selected.has(item.filePath) ? " sel" : ""}" data-path="${escapeHtml(item.filePath)}" data-time="${escapeHtml(time)}" data-hover-content="${encodeURIComponent(tip)}"><td>${escapeHtml(time)}</td><td class="right">${formatSize(item.fileSize)}</td>${markCell}<td class="muted">${highlight(firstLine)}</td></tr>`;
          })
          .join(""),
      );
    }

    // コンパクト画面の「前回」表示(最新の履歴)
    const newest = [...items].sort((a, b) =>
      b.timestamp.localeCompare(a.timestamp),
    )[0];
    setCompactIdleStatus(
      newest
        ? fillTemplate(i18n.compactLast, {
            time: shortTime(newest.timestamp),
            size: formatSize(newest.fileSize),
          })
        : i18n.readyStatus,
    );
    updateSelectionUI();

    // 実行ボタン: 処理中でなければ有効化
    if (!isBackupExecuting()) {
      executeBtn?.removeAttribute("disabled");
      compactExecuteBtn?.removeAttribute("disabled");
    }
    setupHistoryPopups();
  } catch (err) {
    console.error(err);
    setRows(
      messageRow(`Error: ${err.message || err || "loading history"}`, true),
    );
  }
}

function setupHistoryPopups() {
  const tooltip =
    document.getElementById("history-tooltip") || createTooltipElement();
  const targets = document.querySelectorAll(
    "#diff-history-list tr.r, #compact-history-list tr.r",
  );

  targets.forEach((target) => {
    target.onmouseenter = () => {
      tooltip.innerHTML = decodeURIComponent(
        target.getAttribute("data-hover-content"),
      );
      tooltip.classList.remove("hidden");

      const rect = target.getBoundingClientRect();
      const tooltipHeight = tooltip.offsetHeight;
      const tooltipWidth = tooltip.offsetWidth;

      let top = rect.bottom + 2;
      if (top + tooltipHeight > window.innerHeight) {
        top = rect.top - tooltipHeight - 2;
      }
      const left = Math.max(
        4,
        Math.min(rect.left + 10, window.innerWidth - tooltipWidth - 4),
      );
      tooltip.style.left = `${left}px`;
      tooltip.style.top = `${top}px`;
    };

    target.onmouseleave = () => {
      tooltip.classList.add("hidden");
    };
  });
}

function createTooltipElement() {
  const el = document.createElement("div");
  el.id = "history-tooltip";
  el.className = "history-tooltip hidden";
  document.body.appendChild(el);
  return el;
}
