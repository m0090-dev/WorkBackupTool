import {
  i18n,
  tabs,
  recentFiles,
  getActiveTab,
  formatSize,
  saveCurrentSession,
  addToRecentFiles,
} from "./state";
import { GetConfigDir, ReadTextFile } from "./tauri_exports.js";

// tags.json のパスを取得
export async function getTagsFilePath() {
  const configDir = GetConfigDir();
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
 * すべてのテキストはグローバルの i18n オブジェクトを参照します
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

  // i18n の安全な参照（Undefined 対策）
  const t = {
    backupMemo: i18n?.backupMemo || "Note",
    addTagTitle: i18n?.addTagTitle || "Add Tag",
    memoPlaceholder: i18n?.memoPlaceholder || "...",
    cancel: i18n?.cancel || "Cancel",
    save: i18n?.save || "Save",
    enterNewTag: i18n?.enterNewTag || "Enter tag content",
    confirmDeleteTag: i18n?.confirmDeleteTag || "Delete #{tag}?",
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

  // --- マウス操作（コピー・貼り付け）を有効にするための処理 ---
  // 入力欄での右クリックメニューをオーバーレイの onclick から保護する
  input.addEventListener("contextmenu", (e) => {
    e.stopPropagation();
    return true;
  });
  // 入力欄でのクリックも念のため保護
  input.onclick = (e) => e.stopPropagation();

  // --- タグリストの描画と保存ロジック ---
  const renderTags = () => {
    tagList.innerHTML = "";
    tags.forEach((tag, index) => {
      const span = document.createElement("span");
      span.className = "tag-item";
      span.innerText = `#${tag}`;

      // タグクリック：入力欄に追記
      span.onclick = (e) => {
        e.stopPropagation();
        const val = input.value.trim();
        input.value = val ? `${val} #${tag}` : `#${tag}`;
        input.focus();
      };

      // 右クリック：タグの削除
      span.oncontextmenu = async (e) => {
        e.preventDefault();
        e.stopPropagation();
        const confirmMsg = t.confirmDeleteTag.replace("{tag}", tag);
        if (confirm(confirmMsg)) {
          tags.splice(index, 1);
          await SaveTags(tags);
          renderTags();
        }
      };
      tagList.appendChild(span);
    });
  };

  renderTags();

  // --- 定型文の新規追加ボタン ---
  overlay.querySelector("#dialog-tag-add-btn").onclick = async (e) => {
    e.stopPropagation();
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
    overlay.remove();
  };

  overlay.querySelector("#memo-cancel-btn").onclick = (e) => {
    e.stopPropagation();
    overlay.remove();
  };

  // 外側クリックで閉じる（targetが自分自身＝背景の時のみ実行）
  overlay.onclick = (e) => {
    if (e.target === overlay) {
      overlay.remove();
    }
  };
}
