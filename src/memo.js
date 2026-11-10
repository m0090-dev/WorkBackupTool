import {
  i18n,
  tabs,
  recentFiles,
  getActiveTab,
  formatSize,
  saveCurrentSession,
  addToRecentFiles,
} from "./state";
import { GetConfigDir, ReadTextFile, WriteTextFile } from "./tauri_exports.js";

// .noteの読み込み（JSON/テキスト両対応）
export function parseNoteContent(raw) {
  if (!raw) return { text: "", meta: {} };
  try {
    const parsed = JSON.parse(raw);
    // JSONだった場合
    return {
      text: parsed.text ?? "",
      meta: parsed.meta ?? {},
    };
  } catch {
    // 素のテキストだった場合（後方互換）
    return { text: raw, meta: {} };
  }
}
// .noteの書き出し
export function serializeNote(text, meta) {
  return JSON.stringify({ text, meta });
}
// tags.json のパスを取得
export async function getTagsFilePath() {
  const configDir = await GetConfigDir();
  return configDir + "/tags.json"; // 簡易結合（filepath.Joinの代わり）
}

// 定型文を読み込む
export async function LoadTags() {
  const path = await getTagsFilePath();
  const content = await ReadTextFile(path);
  if (!content) return ["ラフ", "線画", "塗り", "修正"]; // 初期値
  try {
    return JSON.parse(content);
  } catch (e) {
    return ["ラフ", "線画", "塗り", "修正"];
  }
}

// 定型文を保存する
export async function SaveTags(tags) {
  const path = await getTagsFilePath();
  await WriteTextFile(path, JSON.stringify(tags));
}

const escapeRegExp = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** メモ欄に #タグ が入っているか(単語として) */
const tagPattern = (tag) => new RegExp(`(^|\\s)#${escapeRegExp(tag)}(?=\\s|$)`);

const escapeHtml = (text) =>
  String(text ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");

/**
 * 再利用可能なメモ入力ダイアログを表示する(mock2_popup_1.html 準拠)
 * options.time: 見出しに出すバックアップの時刻(例 "04:52")
 */
export async function showMemoDialog(
  initialText = "",
  initialMeta = {},
  onSave,
  options = {},
) {
  // 既存のダイアログがあれば削除
  const old = document.getElementById("memo-dialog-overlay");
  if (old) old.remove();

  const compact = document.body.classList.contains("compact-mode");
  const TAG_LIMIT = 3; // コンパクト画面で最初に見せるタグの数
  let expanded = false;

  // 定型文の初期読み込み
  let tags = await LoadTags();

  // i18n の安全な参照
  const t = {
    backupMemo: i18n?.backupMemo || "Note",
    addTag: i18n?.tagAddBtn || "+",
    addTagTitle: i18n?.addTagTitle || "Add Tag",
    memoPlaceholder: i18n?.memoPlaceholder || "...",
    cancel: i18n?.cancel || "Cancel",
    save: i18n?.save || "Save",
    enterNewTag: i18n?.enterNewTag || "Enter tag content",
    delete: i18n?.delete || "Delete",
    markLabel: i18n?.priorityLabel || "Priority",
    marks: [
      i18n?.priorityNone || "None",
      i18n?.priorityLow || "Low",
      i18n?.priorityMid || "Mid",
      i18n?.priorityHigh || "High",
    ],
    hint: i18n?.memoHint || "Ctrl+Enter to save",
  };
  const title =
    !compact && options.time && i18n?.memoDialogTitle
      ? i18n.memoDialogTitle.replace("{time}", options.time)
      : t.backupMemo;

  const overlay = document.createElement("div");
  overlay.id = "memo-dialog-overlay";
  overlay.className = "dialog-overlay";
  overlay.innerHTML = `
    <div class="dialog memo-dialog">
      <div class="d-head">${escapeHtml(title)}</div>
      <div class="d-body">
        ${compact ? "" : `<span class="label">${escapeHtml(t.markLabel)}</span>`}
        <div class="seg">
          ${t.marks
            .map(
              (label, i) =>
                `<div class="seg-item" data-mark="${i}">${escapeHtml(label)}</div>`,
            )
            .join("")}
        </div>
        <div id="dialog-tag-list" class="tags"></div>
        <div id="tag-inline-slot"></div>
        <textarea id="dialog-memo-input" class="memo-textarea" rows="${compact ? 2 : 3}" placeholder="${escapeHtml(t.memoPlaceholder)}"></textarea>
      </div>
      <div class="d-foot">
        ${compact ? "" : `<span class="hint">${escapeHtml(t.hint)}</span>`}
        <button id="memo-cancel-btn">${escapeHtml(t.cancel)}</button>
        <button id="memo-save-btn" class="primary">${escapeHtml(t.save)}</button>
      </div>
    </div>
  `;
  document.body.appendChild(overlay);

  const input = overlay.querySelector("#dialog-memo-input");
  const tagList = overlay.querySelector("#dialog-tag-list");
  const slot = overlay.querySelector("#tag-inline-slot");
  const segItems = overlay.querySelectorAll(".seg-item");

  input.value = initialText;

  // --- 優先度 ---
  const initialMark = initialMeta?.mark ?? 0;
  segItems.forEach((item) => {
    item.classList.toggle("on", parseInt(item.dataset.mark) === initialMark);
    item.onclick = (e) => {
      e.stopPropagation();
      removeContextMenu();
      segItems.forEach((o) => o.classList.remove("on"));
      item.classList.add("on");
    };
  });

  // --- ヘルパー:右クリックメニューの消去 ---
  function removeContextMenu() {
    const existing = document.getElementById("tag-context-menu");
    if (existing) existing.remove();
  }

  // --- タグの描画(クリックで #タグ をメモ欄に入れる/外す) ---
  function renderTags() {
    tagList.innerHTML = "";
    const limited = compact && !expanded && tags.length > TAG_LIMIT;
    const shown = limited ? tags.slice(0, TAG_LIMIT) : tags;

    shown.forEach((tag) => {
      const span = document.createElement("span");
      const re = tagPattern(tag);
      span.className = re.test(input.value) ? "on" : "";
      span.textContent = `#${tag}`;

      span.onclick = (e) => {
        e.stopPropagation();
        removeContextMenu();
        if (re.test(input.value)) {
          input.value = input.value
            .replace(re, "$1")
            .replace(/[ \t]{2,}/g, " ")
            .trim();
        } else {
          const val = input.value.trim();
          input.value = val ? `${val} #${tag}` : `#${tag}`;
        }
        input.focus();
        renderTags();
      };

      // 右クリック:タグの削除メニュー
      span.oncontextmenu = (e) => {
        e.preventDefault();
        e.stopPropagation();
        removeContextMenu();

        const menu = document.createElement("div");
        menu.id = "tag-context-menu";
        menu.className = "tab-context-menu";
        menu.style.left = `${e.clientX}px`;
        menu.style.top = `${e.clientY}px`;

        const delItem = document.createElement("div");
        delItem.className = "tab-menu-item danger";
        delItem.innerHTML = `<span>${escapeHtml(t.delete)}</span><span class="tab-menu-shortcut">#${escapeHtml(tag)}</span>`;
        delItem.onclick = async (ev) => {
          ev.stopPropagation();
          tags = tags.filter((x) => x !== tag);
          await SaveTags(tags);
          renderTags();
          removeContextMenu();
          input.focus();
        };
        menu.appendChild(delItem);
        document.body.appendChild(menu);

        const closeMenu = () => {
          removeContextMenu();
          document.removeEventListener("click", closeMenu);
        };
        setTimeout(() => document.addEventListener("click", closeMenu), 10);
      };

      tagList.appendChild(span);
    });

    // コンパクト画面: 残りのタグは「他 N 件」で展開
    if (limited) {
      const more = document.createElement("span");
      more.className = "more";
      more.textContent = (i18n?.tagsMore || "{count} more").replace(
        "{count}",
        tags.length - TAG_LIMIT,
      );
      more.onclick = (e) => {
        e.stopPropagation();
        expanded = true;
        renderTags();
      };
      tagList.appendChild(more);
    }

    // タグの追加
    const add = document.createElement("span");
    add.className = "more";
    add.textContent = t.addTag;
    add.title = t.addTagTitle;
    add.onclick = (e) => {
      e.stopPropagation();
      removeContextMenu();
      openInlineTagInput();
    };
    tagList.appendChild(add);
  }

  function openInlineTagInput() {
    if (slot.firstChild) {
      slot.innerHTML = "";
      return;
    }
    const box = document.createElement("div");
    box.className = "tag-inline-add";
    const tagInput = document.createElement("input");
    tagInput.type = "text";
    tagInput.placeholder = t.enterNewTag;
    const confirmBtn = document.createElement("button");
    confirmBtn.textContent = "OK";
    box.appendChild(tagInput);
    box.appendChild(confirmBtn);
    slot.appendChild(box);
    tagInput.focus();

    const addTag = async () => {
      const val = tagInput.value.replace(/^#/, "").trim();
      if (val && !tags.includes(val)) {
        tags.push(val);
        await SaveTags(tags);
        expanded = true;
        renderTags();
      }
      slot.innerHTML = "";
    };
    confirmBtn.onclick = (ev) => {
      ev.stopPropagation();
      addTag();
    };
    tagInput.addEventListener("keydown", (ev) => {
      ev.stopPropagation();
      if (ev.key === "Enter") {
        ev.preventDefault();
        addTag();
      }
      if (ev.key === "Escape") slot.innerHTML = "";
    });
  }

  renderTags();
  input.addEventListener("input", renderTags);
  input.addEventListener("contextmenu", (e) => {
    e.stopPropagation();
    removeContextMenu();
  });
  input.onclick = (e) => {
    e.stopPropagation();
    removeContextMenu();
  };

  // --- 保存 / キャンセル ---
  const close = () => {
    removeContextMenu();
    overlay.remove();
  };
  const save = () => {
    const mark = parseInt(
      overlay.querySelector(".seg-item.on")?.dataset.mark ?? "0",
    );
    if (onSave) onSave(input.value.trim(), { mark });
    close();
  };
  overlay.querySelector("#memo-save-btn").onclick = (e) => {
    e.stopPropagation();
    save();
  };
  overlay.querySelector("#memo-cancel-btn").onclick = (e) => {
    e.stopPropagation();
    close();
  };
  overlay.onclick = (e) => {
    if (e.target === overlay) close();
  };
  overlay.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      save();
    } else if (e.key === "Escape") {
      close();
    }
  });

  input.focus();
}
