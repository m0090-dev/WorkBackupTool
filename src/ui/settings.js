import {
  GetConfigDir,
  UpdateConfigValue,
  UpdateSessionTabValue,
  GetConfig,
} from "../tauri_exports";
import { i18n, getActiveTab } from "../state";
import { settingsSchema, categoryLabels } from "../settings-schema.js";
import { shortcutFromEvent } from "../shortcut.js";
import { escapeHtml } from "./html.js";
import { showFloatingMessage, showFloatingError } from "./messages.js";

export async function showSettingsModal() {
  const modal = document.getElementById("settings-modal");
  if (!modal || !i18n) return;
  try {
    const config = await GetConfig();
    const lang = config.language || "ja";

    const categories = [
      ...new Set(settingsSchema.map((item) => item.category)),
    ];

    const modalContent = modal.querySelector(".dialog");
    if (!modalContent) return;

    const tabsHtml = categories
      .map((cat, idx) => {
        const label = categoryLabels[cat]?.[lang] || cat;
        return `<button class="settings-tab-btn ${idx === 0 ? "active" : ""}" data-category="${cat}">${label}</button>`;
      })
      .join("");

    const pagesHtml = categories
      .map((cat, idx) => {
        const items = settingsSchema.filter((item) => item.category === cat);
        const itemsHtml = items
          .map((item) => {
            const labelText = i18n[item.label] || item.label;
            const hintText = item.hint ? i18n[item.hint] || "" : "";

            // tab スコープはアクティブタブから現在値を取得
            const currentValue =
              item.scope === "tab"
                ? (getActiveTab()?.[item.key] ?? [])
                : config[item.key];

            if (item.type === "boolean") {
              return `
            <div class="settings-item" data-key="${item.key}">
              <label class="check-row">
                <input type="checkbox" class="settings-input" data-key="${item.key}" ${currentValue ? "checked" : ""}>
                <span>${labelText}</span>
              </label>
              ${hintText ? `<div class="settings-hint">${hintText}</div>` : ""}
            </div>`;
            } else if (item.type === "number") {
              return `
            <div class="settings-item" data-key="${item.key}">
              <span class="settings-name">${labelText}</span>
              <input type="number" class="settings-input" data-key="${item.key}"
                value="${currentValue}"
                ${item.min !== null ? `min="${item.min}"` : ""}
                ${item.max !== null ? `max="${item.max}"` : ""}
                step="${item.step || 1}">
              ${hintText ? `<div class="settings-hint">${hintText}</div>` : ""}
            </div>`;
            } else if (item.type === "select") {
              const optionsHtml = (item.options || [])
                .map(
                  (o) =>
                    `<option value="${escapeHtml(o.value)}" ${String(o.value) === String(currentValue) ? "selected" : ""}>${escapeHtml(i18n[o.label] || o.label)}</option>`,
                )
                .join("");
              return `
            <div class="settings-item" data-key="${item.key}">
              <span class="settings-name">${labelText}</span>
              <select class="settings-input" data-key="${item.key}">${optionsHtml}</select>
              ${hintText ? `<div class="settings-hint">${hintText}</div>` : ""}
            </div>`;
            } else if (item.type === "shortcut") {
              return `
            <div class="settings-item" data-key="${item.key}">
              <span class="settings-name">${labelText}</span>
              <input type="text" class="settings-shortcut" data-key="${item.key}" readonly
                value="${escapeHtml(currentValue || "")}" placeholder="${escapeHtml(i18n.shortcutPlaceholder || "Press keys")}">
              ${hintText ? `<div class="settings-hint">${hintText}</div>` : ""}
            </div>`;
            } else if (item.type === "taglist") {
              // タグ（除外パターン）リストの表示。追加・削除をインタラクティブに行う。
              const tagsHtml = (Array.isArray(currentValue) ? currentValue : [])
                .map(
                  (tag) =>
                    `<span class="settings-tag" data-tag="${escapeHtml(tag)}">${escapeHtml(tag)}<button class="settings-tag-remove" data-key="${item.key}" data-tag="${escapeHtml(tag)}" title="Remove">×</button></span>`,
                )
                .join("");
              return `
            <div class="settings-item settings-taglist-item" data-key="${item.key}" data-scope="${item.scope || "config"}">
              <span class="settings-name">${labelText}</span>
              <div class="settings-tags" id="tags-${item.key}">${tagsHtml}</div>
              <div class="taglist-add">
                <input type="text" class="settings-tag-input" data-key="${item.key}"
                  placeholder="${i18n.hdiffIgnoreListPlaceholder || "e.g. *.tmp"}">
                <button class="settings-tag-add" data-key="${item.key}">${i18n.addBtn || "Add"}</button>
              </div>
              ${hintText ? `<div class="settings-hint">${hintText}</div>` : ""}
            </div>`;
            }
          })
          .join("");

        return `<div class="settings-page ${idx === 0 ? "" : "hidden"}" data-category="${cat}">${itemsHtml}</div>`;
      })
      .join("");

    modalContent.innerHTML = `
      <div class="d-head">${i18n.advancedSettingsTitle || "Advanced Settings"}</div>
      <div class="d-body">
        <div class="settings-tabs">${tabsHtml}</div>
        <div class="settings-pages">${pagesHtml}</div>
      </div>
      <div class="d-foot">
        <button id="settings-close-btn">${i18n.closeBtn || "Close"}</button>
      </div>
    `;

    modalContent.querySelectorAll(".settings-tab-btn").forEach((btn) => {
      btn.onclick = (e) => {
        e.stopPropagation();
        modalContent
          .querySelectorAll(".settings-tab-btn")
          .forEach((b) => b.classList.remove("active"));
        modalContent
          .querySelectorAll(".settings-page")
          .forEach((p) => p.classList.add("hidden"));
        btn.classList.add("active");
        modalContent
          .querySelector(
            `.settings-page[data-category="${btn.dataset.category}"]`,
          )
          ?.classList.remove("hidden");
      };
    });

    // ---- [FIX #28] 設定変更ハンドラ ----
    // boolean / number を型に応じて正しく処理する
    modalContent.querySelectorAll(".settings-input").forEach((input) => {
      const handler = async () => {
        const key = input.dataset.key;
        const schema = settingsSchema.find((s) => s.key === key);
        if (!schema) return;

        let value;
        if (schema.type === "boolean") {
          // boolean はチェック状態をそのまま使う（数値バリデーションは不要）
          value = input.checked;
        } else if (schema.type === "select") {
          value =
            schema.valueType === "number" ? Number(input.value) : input.value;
        } else {
          value = parseFloat(input.value);
          if (isNaN(value)) return;
          if (schema.min !== null && value < schema.min) return;
          if (schema.max !== null && value > schema.max) return;
        }

        await handleSettingChange(key, value);
      };
      input.addEventListener("change", handler);
    });

    // ---- キー割り当て欄(shortcut) ----
    // 欄を選んでキーを押すと割り当てる。Backspace/Delete で未割り当て、Escape で中止。
    modalContent.querySelectorAll(".settings-shortcut").forEach((input) => {
      input.addEventListener("keydown", async (e) => {
        if (e.key === "Tab") return;
        e.preventDefault();
        e.stopPropagation();
        if (e.key === "Escape") {
          input.blur();
          return;
        }
        let value;
        if (e.key === "Backspace" || e.key === "Delete") {
          value = "";
        } else {
          value = shortcutFromEvent(e);
          if (value === null) return; // 修飾キーだけ等は、次のキーを待つ
        }
        const key = input.dataset.key;
        const previous = input.value;
        input.value = value;
        try {
          await UpdateConfigValue(key, value);
          showFloatingMessage(i18n.settingsSaved || "Settings saved");
        } catch (err) {
          input.value = previous;
          console.error(`Failed to update ${key}:`, err);
          showFloatingError(i18n.settingsError || "Save failed");
        }
      });
    });

    // ---- 依存関係(dependsOn): 親のチェックがオフの間は無効にする ----
    const applyDependencies = () => {
      settingsSchema
        .filter((item) => item.dependsOn)
        .forEach((item) => {
          const parent = modalContent.querySelector(
            `.settings-input[data-key="${item.dependsOn}"]`,
          );
          const enabled = parent ? parent.checked : true;
          const row = modalContent.querySelector(
            `.settings-item[data-key="${item.key}"]`,
          );
          if (!row) return;
          row.classList.toggle("disabled", !enabled);
          row
            .querySelectorAll("input, select, button")
            .forEach((el) => (el.disabled = !enabled));
        });
    };
    modalContent
      .querySelectorAll('.settings-input[type="checkbox"]')
      .forEach((cb) => cb.addEventListener("change", applyDependencies));
    applyDependencies();

    // ---- taglist (tab スコープ) のイベントハンドラ ----
    // タグ追加: Add ボタン または Enter キー
    const setupTaglistHandlers = (container) => {
      container.querySelectorAll(".settings-tag-add").forEach((btn) => {
        btn.onclick = async () => {
          const key = btn.dataset.key;
          const textInput = container.querySelector(
            `.settings-tag-input[data-key="${key}"]`,
          );
          const pattern = textInput?.value.trim();
          if (!pattern) return;

          const tab = getActiveTab();
          if (!tab) return;
          const list = Array.isArray(tab[key]) ? [...tab[key]] : [];
          if (list.includes(pattern)) {
            textInput.value = "";
            return;
          }
          list.push(pattern);
          await handleTabSettingChange(tab, key, list);
          // タグDOM更新（再描画せずにインプレースで追加）
          const tagsContainer = container.querySelector(`#tags-${key}`);
          if (tagsContainer) {
            const span = document.createElement("span");
            span.className = "settings-tag";
            span.dataset.tag = pattern;
            span.innerHTML = `${pattern}<button class="settings-tag-remove" data-key="${key}" data-tag="${pattern}" title="Remove">×</button>`;
            tagsContainer.appendChild(span);
            span.querySelector(".settings-tag-remove").onclick = (e) =>
              removeTagHandler(e, container);
          }
          textInput.value = "";
        };
      });
      container.querySelectorAll(".settings-tag-input").forEach((input) => {
        input.addEventListener("keydown", (e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            container
              .querySelector(
                `.settings-tag-add[data-key="${input.dataset.key}"]`,
              )
              ?.click();
          }
        });
      });
      // 既存タグの削除ボタン
      container.querySelectorAll(".settings-tag-remove").forEach((btn) => {
        btn.onclick = (e) => removeTagHandler(e, container);
      });
    };

    const removeTagHandler = async (e, container) => {
      const btn = e.currentTarget;
      const key = btn.dataset.key;
      const tag = btn.dataset.tag;
      const tab = getActiveTab();
      if (!tab) return;
      const list = (Array.isArray(tab[key]) ? tab[key] : []).filter(
        (t) => t !== tag,
      );
      await handleTabSettingChange(tab, key, list);
      btn.closest(".settings-tag")?.remove();
    };

    setupTaglistHandlers(modalContent);

    modal.classList.remove("hidden");
  } catch (err) {
    console.error("Failed to load settings:", err);
    showFloatingError(i18n.errorLoadingHistory || "Failed to load settings");
  }
}

export async function handleSettingChange(key, value) {
  try {
    await UpdateConfigValue(key, value);
    showFloatingMessage(i18n.settingsSaved || "Settings saved");
  } catch (err) {
    console.error(`Failed to update ${key}:`, err);
    showFloatingError(i18n.settingsError || "Save failed");
  }
}

/// タブスコープの設定を session.json に保存し、インメモリのタブにも反映する
export async function handleTabSettingChange(tab, key, value) {
  try {
    tab[key] = value; // インメモリ更新
    const configDir = await GetConfigDir();
    const sessionPath = configDir + "/session.json";
    await UpdateSessionTabValue(sessionPath, tab.id, key, value);
    showFloatingMessage(i18n.settingsSaved || "Settings saved");
  } catch (err) {
    console.error(`Failed to update tab.${key}:`, err);
    showFloatingError(i18n.settingsError || "Save failed");
  }
}
