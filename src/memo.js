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

/**
 * 再利用可能なメモ入力ダイアログを表示する
 */
export async function showMemoDialog(initialText = "", onSave) {
  // 既存のダイアログがあれば削除
  const old = document.getElementById("memo-dialog-overlay");
  if (old) old.remove();

  // オーバーレイの作成
  const overlay = document.createElement("div");
  overlay.id = "memo-dialog-overlay";
  overlay.className = "memo-overlay";

  // 定型文の初期読み込み
  let tags = await LoadTags();

  // i18n の安全な参照
  const t = {
    backupMemo: i18n?.backupMemo || "Note",
    addTagTitle: i18n?.addTagTitle || "Add Tag",
    memoPlaceholder: i18n?.memoPlaceholder || "...",
    cancel: i18n?.cancel || "Cancel",
    save: i18n?.save || "Save",
    enterNewTag: i18n?.enterNewTag || "Enter tag content",
    delete: i18n?.delete || "Delete", // 削除ボタン用
  };

  // ダイアログのHTML構造
  overlay.innerHTML = `
        <div class="memo-dialog">
            <div class="memo-dialog-header"> ${t.backupMemo}</div>
            <div class="memo-tag-container">
                <div id="dialog-tag-list" style="display:inline-block;"></div>
                <button id="dialog-tag-add-btn" class="tag-add-btn" title="${t.addTagTitle}">+</button>
            </div>
            <textarea id="dialog-memo-input" class="memo-textarea" rows="3" placeholder="${t.memoPlaceholder}"></textarea>
            <div class="memo-dialog-footer">
                <button id="memo-cancel-btn" class="memo-btn-secondary">${t.cancel}</button>
                <button id="memo-save-btn" class="memo-btn-primary">${t.save}</button>
            </div>
        </div>
    `;

  document.body.appendChild(overlay);

  const input = overlay.querySelector("#dialog-memo-input");
  const tagList = overlay.querySelector("#dialog-tag-list");

  input.value = initialText;
  input.focus();

  // --- ヘルパー：既存のカスタムメニューを消去 ---
  const removeContextMenu = () => {
    const existing = document.getElementById("tag-context-menu");
    if (existing) existing.remove();
  };

  // --- タグリストの描画 ---
  const renderTags = () => {
    tagList.innerHTML = "";
    tags.forEach((tag, index) => {
      const span = document.createElement("span");
      span.className = "tag-item";
      span.innerText = `#${tag}`;

      // 左クリック：タグ挿入
      span.onclick = (e) => {
        e.stopPropagation();
        removeContextMenu();
        const val = input.value.trim();
        input.value = val ? `${val} #${tag}` : `#${tag}`;
        input.focus();
      };

      // 右クリック：カスタムコンテキストメニュー表示
      span.oncontextmenu = (e) => {
        e.preventDefault();
        e.stopPropagation();
        removeContextMenu();

        // メニュー生成
        const menu = document.createElement("div");
        menu.id = "tag-context-menu";
        menu.className = "tab-context-menu"; // ご提示のCSSクラスを使用
        menu.style.left = `${e.clientX}px`;
        menu.style.top = `${e.clientY}px`;

        // 削除アイテム生成
        const delItem = document.createElement("div");
        delItem.className = "tab-menu-item danger"; // 赤色強調スタイル
        delItem.innerHTML = `<span>${t.delete}</span><span class="tab-menu-shortcut">#${tag}</span>`;

        delItem.onclick = async (ev) => {
          ev.stopPropagation();
          tags.splice(index, 1);
          await SaveTags(tags);
          renderTags();
          removeContextMenu();
          if (input) input.focus();
        };

        menu.appendChild(delItem);
        document.body.appendChild(menu);

        // メニュー以外をクリックしたら閉じる
        const closeMenu = () => {
          removeContextMenu();
          document.removeEventListener("click", closeMenu);
        };
        setTimeout(() => document.addEventListener("click", closeMenu), 10);
      };

      tagList.appendChild(span);
    });
  };

  renderTags();

  // --- 入力欄などのイベント保護（メニュー消去を兼ねる） ---
  input.addEventListener("contextmenu", (e) => {
    e.stopPropagation();
    removeContextMenu();
    return true;
  });

  input.onclick = (e) => {
    e.stopPropagation();
    removeContextMenu();
  };

  // --- 定型文の新規追加ボタン ---
  overlay.querySelector("#dialog-tag-add-btn").onclick = async (e) => {
    e.stopPropagation();
    removeContextMenu();
    const newTag = prompt(t.enterNewTag);
    if (newTag && newTag.trim() !== "") {
      const cleanTag = newTag.replace(/^#/, "").trim();
      if (!tags.includes(cleanTag)) {
        tags.push(cleanTag);
        await SaveTags(tags);
        renderTags();
      }
    }
  };

  // --- ダイアログのボタン操作 ---
  overlay.querySelector("#memo-save-btn").onclick = (e) => {
    e.stopPropagation();
    if (onSave) onSave(input.value.trim());
    removeContextMenu();
    overlay.remove();
  };

  overlay.querySelector("#memo-cancel-btn").onclick = (e) => {
    e.stopPropagation();
    removeContextMenu();
    overlay.remove();
  };

  // 外側クリックで閉じる
  overlay.onclick = (e) => {
    if (e.target === overlay) {
      removeContextMenu();
      overlay.remove();
    }
  };
}
