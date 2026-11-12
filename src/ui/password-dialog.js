import { i18n } from "../state";

/**
 * パスワード付きアーカイブの復元で、パスワードを尋ねるダイアログ。
 * @param {{ prefill?: string, wrong?: boolean }} options
 *   prefill: 最初から入れておく値 / wrong: 直前のパスワードが違っていたときに true
 * @returns {Promise<string|null>} 入力したパスワード。キャンセルしたら null
 */
export function askArchivePassword({ prefill = "", wrong = false } = {}) {
  return new Promise((resolve) => {
    document.getElementById("password-dialog-overlay")?.remove();

    const overlay = document.createElement("div");
    overlay.id = "password-dialog-overlay";
    overlay.className = "dialog-overlay";

    const dialog = document.createElement("div");
    dialog.className = "dialog";

    const head = document.createElement("div");
    head.className = "d-head";
    head.textContent = i18n?.passwordPromptTitle || "Password required";

    const body = document.createElement("div");
    body.className = "d-body";

    const text = document.createElement("div");
    text.className = "password-prompt-text";
    text.textContent =
      i18n?.passwordPromptBody ||
      "This archive is protected with a password. Enter it to restore.";
    body.appendChild(text);

    if (wrong) {
      const notice = document.createElement("div");
      notice.className = "notice password-prompt-wrong";
      notice.textContent =
        i18n?.passwordPromptWrong || "The password is incorrect.";
      body.appendChild(notice);
    }

    const input = document.createElement("input");
    input.type = "password";
    input.className = "password-input";
    input.placeholder = i18n?.passwordPlaceholder || "Enter password";
    input.value = prefill;
    body.appendChild(input);

    const foot = document.createElement("div");
    foot.className = "d-foot";
    const cancelBtn = document.createElement("button");
    cancelBtn.textContent = i18n?.cancel || "Cancel";
    const okBtn = document.createElement("button");
    okBtn.className = "primary";
    okBtn.textContent = i18n?.okBtn || "OK";
    foot.append(cancelBtn, okBtn);

    dialog.append(head, body, foot);
    overlay.appendChild(dialog);
    document.body.appendChild(overlay);

    const finish = (value) => {
      overlay.remove();
      resolve(value);
    };
    okBtn.onclick = (e) => {
      e.stopPropagation();
      finish(input.value);
    };
    cancelBtn.onclick = (e) => {
      e.stopPropagation();
      finish(null);
    };
    overlay.onclick = (e) => {
      if (e.target === overlay) finish(null);
    };
    overlay.addEventListener("keydown", (e) => {
      e.stopPropagation();
      if (e.key === "Enter") {
        e.preventDefault();
        finish(input.value);
      } else if (e.key === "Escape") {
        finish(null);
      }
    });

    input.focus();
    input.select();
  });
}
