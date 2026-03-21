// settings-schema.js
export const settingsSchema = [
  {
    key: "startupCacheLimit",
    type: "number",
    min: 0,
    max: null,
    step: 1,
    category: "cache",
    label: "startupCacheLimit",
    hint: "startupCacheLimitHint",
  },
  {
    key: "autoBaseGenerationThreshold",
    type: "number",
    min: 0.1,
    max: 1.0,
    step: 0.1,
    category: "backup",
    label: "thresholdLabel",
    hint: "thresholdHint",
  },
  {
    key: "showMemoAfterBackup",
    type: "boolean",
    category: "memo",
    label: "showMemoAfterBackup",
    hint: null,
  },
  {
    key: "strictFileNameMatch",
    type: "boolean",
    category: "history",
    label: "filterHistoryByFilename",
    hint: "filterHistoryByFilenameHint",
  },
];

export const categoryLabels = {
  backup: { en: "Backup", ja: "バックアップ" },
  cache: { en: "Cache", ja: "キャッシュ" },
  memo: { en: "Memo", ja: "メモ" },
  history: { en: "History", ja: "履歴" },
};
