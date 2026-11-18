import { i18n } from "../state";
import { escapeHtml } from "./html.js";

/**
 * カスタム保存先フォルダの使い方を選ぶダイアログ。
 * (パスワードダイアログ askArchivePassword と同じ作りのモーダル)
 * @param {{ dir: string, initialMode: "parent"|"direct", subfolderName: string }} options
 *   dir: 指定されたフォルダ / initialMode: 初期選択 / subfolderName: 親モードで作るフォルダ名
 * @returns {Promise<"parent"|"direct"|null>} キャンセルしたら null
 */
export function askBackupDirMode({ dir, initialMode, subfolderName }) {
  return new Promise((resolve) => {
    document.getElementById("backup-dir-dialog-overlay")?.remove();

    const overlay = document.createElement("div");
    overlay.id = "backup-dir-dialog-overlay";
    overlay.className = "dialog-overlay";

    const parentDesc = (
      i18n?.backupDirModeParentDesc ||
      "Create {name} inside it and store backups there."
    ).replace("{name}", subfolderName || "wbt_backup_...");
    const directDesc =
      i18n?.backupDirModeDirectDesc || "Store backups directly in this folder.";

    const dialog = document.createElement("div");
    dialog.className = "dialog";
    dialog.innerHTML = `
      <div class="d-head">${escapeHtml(i18n?.backupDirModeTitle || "How should this folder be used?")}</div>
      <div class="d-body">
        <span class="label">${escapeHtml(i18n?.backupDirBtn || "Backup folder")}</span>
        <div class="path">${escapeHtml(dir)}</div>
        <div class="backup-dir-options">
          <label class="backup-dir-option">
            <input type="radio" name="backup-dir-mode" value="parent">
            <span class="backup-dir-option-copy">
              <span class="backup-dir-option-title">${escapeHtml(i18n?.backupDirModeParentLabel || "Use as a parent folder")}</span>
              <span class="backup-dir-option-desc">${escapeHtml(parentDesc)}</span>
            </span>
          </label>
          <label class="backup-dir-option">
            <input type="radio" name="backup-dir-mode" value="direct">
            <span class="backup-dir-option-copy">
              <span class="backup-dir-option-title">${escapeHtml(i18n?.backupDirModeDirectLabel || "Store directly in this folder")}</span>
              <span class="backup-dir-option-desc">${escapeHtml(directDesc)}</span>
            </span>
          </label>
        </div>
      </div>
      <div class="d-foot">
        <button data-role="cancel">${escapeHtml(i18n?.cancel || "Cancel")}</button>
        <button data-role="ok" class="primary">${escapeHtml(i18n?.okBtn || "OK")}</button>
      </div>
    `;
    overlay.appendChild(dialog);
    document.body.appendChild(overlay);

    const radios = dialog.querySelectorAll('input[name="backup-dir-mode"]');
    radios.forEach((r) => {
      r.checked = r.value === (initialMode === "direct" ? "direct" : "parent");
    });

    const finish = (value) => {
      overlay.remove();
      resolve(value);
    };
    dialog.querySelector('[data-role="ok"]').onclick = (e) => {
      e.stopPropagation();
      const checked = dialog.querySelector(
        'input[name="backup-dir-mode"]:checked',
      );
      finish(checked ? checked.value : null);
    };
    dialog.querySelector('[data-role="cancel"]').onclick = (e) => {
      e.stopPropagation();
      finish(null);
    };
    overlay.onclick = (e) => {
      if (e.target === overlay) finish(null);
    };
    overlay.addEventListener("keydown", (e) => {
      e.stopPropagation();
      if (e.key === "Escape") finish(null);
    });

    dialog.querySelector('[data-role="ok"]').focus();
  });
}
