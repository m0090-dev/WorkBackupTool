// settings-schema.js
// 詳細設定画面の項目定義。ui/settings.js がこの定義から画面を組み立てる。
//
// 共通の項目:
//   key       設定のキー(AppConfig の camelCase 名。tab スコープは session.json のタブのキー)
//   type      "boolean" | "number" | "select" | "shortcut" | "taglist"
//   category  タブ分けのカテゴリ(categoryLabels に表示名を用意する)
//   label     表示名の i18n キー / hint  補足文の i18n キー(省略可)
//   scope     "tab" を指定するとアクティブなタブの設定(session.json に保存)
//   dependsOn 別の boolean 項目のキー。その項目がオフの間はこの項目を無効にする
//
// 種類ごとの項目:
//   number    min / max / step
//   select    options: [{ value, label(i18nキー) }]、valueType: "number" で数値として保存
//   shortcut  キーを押して割り当てるテキスト欄(例 "Ctrl+Alt+B")。Backspace で未割り当て
export const settingsSchema = [
  // --- 一般(メニューの「起動時の動作」から移動) ---
  {
    key: "restorePreviousState",
    type: "boolean",
    category: "general",
    label: "restoreState",
  },

  // --- バックアップ ---
  {
    key: "autoBaseGenerationThreshold",
    type: "percent",
    min: 10,
    max: 300,
    step: 5,
    category: "backup",
    label: "thresholdLabel",
    hint: "thresholdHint",
  },
  {
    key: "hdiffStrictHashCheck",
    type: "boolean",
    category: "backup",
    label: "hdiffStrictHashCheckLabel",
    hint: "hdiffStrictHashCheckHint",
  },
  {
    key: "showMemoAfterBackup",
    type: "boolean",
    category: "backup",
    label: "showMemoAfterBackup",
  },

  // --- キャッシュ ---
  {
    key: "rebuildCacheOnStartup",
    type: "boolean",
    category: "cache",
    label: "rebuildCacheOnStartup",
  },
  {
    key: "startupCacheLimit",
    type: "number",
    min: 0,
    max: null,
    step: 1,
    category: "cache",
    label: "startupCacheLimit",
    hint: "startupCacheLimitHint",
    dependsOn: "rebuildCacheOnStartup",
  },
  {
    key: "useSameDirForTemp",
    type: "boolean",
    category: "cache",
    label: "useSameDirForTemp",
  },

  // --- 履歴 ---
  {
    key: "strictFileNameMatch",
    type: "boolean",
    category: "history",
    label: "filterHistoryByFilename",
    hint: "filterHistoryByFilenameHint",
  },

  // --- 完了の通知 ---
  {
    key: "notifyEnabled",
    type: "boolean",
    category: "notify",
    label: "notifyEnabledLabel",
    hint: "notifyEnabledHint",
  },
  {
    key: "notifyStyle",
    type: "select",
    category: "notify",
    label: "notifyStyleLabel",
    options: [
      { value: "popup", label: "notifyStylePopup" },
      { value: "sound", label: "notifyStyleSound" },
      { value: "both", label: "notifyStyleBoth" },
    ],
    dependsOn: "notifyEnabled",
  },
  {
    key: "notifyDurationSec",
    type: "number",
    min: 1,
    max: 5,
    step: 1,
    category: "notify",
    label: "notifyDurationLabel",
    hint: "notifyDurationHint",
    dependsOn: "notifyEnabled",
  },
  {
    key: "notifyOnlyOnFailure",
    type: "boolean",
    category: "notify",
    label: "notifyOnlyOnFailureLabel",
    dependsOn: "notifyEnabled",
  },

  // --- タブごとの設定 (session.json に保存される) ---
  {
    key: "hdiffIgnoreList",
    type: "taglist",
    category: "tab",
    label: "hdiffIgnoreListLabel",
    hint: "hdiffIgnoreListHint",
    // scope: "tab" を明示しておくことで ui.js 側が分岐判断に使える
    scope: "tab",
  },
];

export const categoryLabels = {
  general: { en: "General", ja: "一般" },
  backup: { en: "Backup", ja: "バックアップ" },
  cache: { en: "Cache", ja: "キャッシュ" },
  history: { en: "History", ja: "履歴" },
  tab: { en: "Tab (Current)", ja: "タブ (現在)" },
  // 今後の機能追加用(項目を足すときに使う)
  shortcut: { en: "Shortcut", ja: "ショートカット" },
  notify: { en: "Notification", ja: "通知" },
  reminder: { en: "Reminder", ja: "取り忘れ" },
  restore: { en: "Restore", ja: "復元" },
};
