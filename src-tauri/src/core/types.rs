use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---- AppConfig の既定値 ----
// 設定ファイル(AppConfig.json)に項目が無い場合(旧バージョンの設定ファイルなど)でも、
// 設定全体が初期化されないように、すべての項目に既定値を用意している。
// ※ ここの値は src/assets/AppConfig.json(組み込みの既定値)と一致させること。
fn default_language() -> String {
    "ja".to_string()
}
fn default_true() -> bool {
    true
}
fn default_threshold() -> f64 {
    1.0
}
fn default_tray_backup_mode() -> String {
    "diff".to_string()
}
fn default_startup_cache_limit() -> usize {
    5
}
fn default_shortcut_run() -> String {
    "Ctrl+Alt+B".to_string()
}
fn default_shortcut_run_with_memo() -> String {
    "Ctrl+Alt+Shift+B".to_string()
}
fn default_notify_style() -> String {
    "popup".to_string()
}
fn default_notify_duration_sec() -> u32 {
    3
}
fn default_reminder_interval_min() -> u32 {
    30
}
fn default_reminder_method() -> String {
    "popup".to_string()
}

// 設定ファイル情報
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub always_on_top: bool,
    #[serde(default = "default_true")]
    pub restore_previous_state: bool,
    #[serde(default)]
    pub tray_mode: bool,
    #[serde(default = "default_threshold")]
    pub auto_base_generation_threshold: f64,
    #[serde(skip_serializing, default)]
    pub compact_mode: bool,
    #[serde(default = "default_tray_backup_mode")]
    pub tray_backup_mode: String,
    #[serde(default)]
    pub use_same_dir_for_temp: bool,
    #[serde(default = "default_true")]
    pub rebuild_cache_on_startup: bool,
    #[serde(default = "default_startup_cache_limit")]
    pub startup_cache_limit: usize,
    #[serde(default = "default_true")]
    pub show_memo_after_backup: bool,
    #[serde(default = "default_true")]
    pub strict_file_name_match: bool,
    #[serde(default = "default_true")]
    pub hdiff_strict_hash_check: bool,

    // --- グローバルショートカット ---
    #[serde(default)]
    pub global_shortcut_enabled: bool,
    /// 「実行」用のキー割り当て(例 "Ctrl+Alt+B")。空文字なら未割り当て
    #[serde(default = "default_shortcut_run")]
    pub shortcut_run: String,
    /// 「メモ付きで実行」用のキー割り当て。空文字なら未割り当て
    #[serde(default = "default_shortcut_run_with_memo")]
    pub shortcut_run_with_memo: String,
    /// ショートカットで実行するバックアップモード("copy" / "archive" / "diff")
    #[serde(default = "default_tray_backup_mode")]
    pub shortcut_backup_mode: String,

    // --- 完了の通知 ---
    #[serde(default)]
    pub notify_enabled: bool,
    /// "popup"(小さなポップアップ) / "sound"(音だけ) / "both"(両方)
    #[serde(default = "default_notify_style")]
    pub notify_style: String,
    /// ポップアップの表示時間(秒、1〜5)
    #[serde(default = "default_notify_duration_sec")]
    pub notify_duration_sec: u32,
    #[serde(default)]
    pub notify_only_on_failure: bool,

    // --- 取り忘れの知らせ ---
    #[serde(default)]
    pub reminder_enabled: bool,
    /// 知らせる間隔(分: 15 / 30 / 60)
    #[serde(default = "default_reminder_interval_min")]
    pub reminder_interval_min: u32,
    /// "popup" / "sound"
    #[serde(default = "default_reminder_method")]
    pub reminder_method: String,
    /// 作業ファイルが前回のバックアップから変わっていないときは知らせない
    #[serde(default = "default_true")]
    pub reminder_skip_unchanged: bool,

    // --- 復元 ---
    #[serde(default)]
    pub open_after_restore: bool,

    /// 復元したものを、新しい作業対象に設定し直すかどうかの確認(この機能全体のオン/オフ)
    #[serde(default = "default_true")]
    pub restore_relink_enabled: bool,
    /// 確認せずに、自動で設定し直す
    #[serde(default)]
    pub restore_relink_auto: bool,
    /// 作業対象が存在しない(または空の)ときだけ行う。オフなら、存在していても行う
    #[serde(default = "default_true")]
    pub restore_relink_only_when_missing: bool,
    /// 設定し直すときに、保存先も初期状態に戻す
    #[serde(default = "default_true")]
    pub restore_relink_reset_backup_dir: bool,
    /// 作業対象が空のとき、復元先のフォルダを毎回尋ねる(オフなら、標準の配置は自動で決める)
    #[serde(default)]
    pub restore_always_ask_output_dir: bool,

    // --- カスタム保存先の使い方 ---
    /// カスタム保存先でも、中に `wbt_backup_<対象名>` フォルダを作って保存する
    /// (保存先指定時の既定の動作で、確認ポップアップの初期選択にも使う)
    #[serde(default = "default_true")]
    pub custom_backup_dir_use_subfolder: bool,
    /// 保存先を指定したときに、使い方(親フォルダ / 直接)の確認ポップアップを出さない
    #[serde(default)]
    pub custom_backup_dir_hide_mode_prompt: bool,

    // --- 履歴プレビュー ---
    /// 履歴のホバーポップアップに、その時点の作業ファイルのプレビュー画像を出す。
    /// オフのときは、プレビュー用の一時ファイル・キャッシュも一切作らない。
    #[serde(default = "default_true")]
    pub preview_enabled: bool,
}

// 差分ファイル情報
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffFileInfo {
    pub file_name: String, // test-project.clip.2025...diff
    pub file_path: String, // フルパス
    pub timestamp: String, // 2025... 部分
    pub file_size: i64,
}

// 履歴リストに表示する各ファイルの情報を保持
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupItem {
    pub file_name: String,
    pub file_path: String,
    pub timestamp: String,
    pub file_size: i64,
    pub generation: i32,
    pub is_archived: bool,
    pub is_folder: bool,
}

// session.json のタブ1件を表す構造体（セッション更新コマンド用）
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TabSession {
    pub id: u64,
    pub work_file: String,
    #[serde(default)]
    pub work_file_size: i64,
    pub backup_dir: String,
    /// 保存先フォルダの使い方。"parent" は backup_dir の中に `wbt_backup_<対象名>` を作る、
    /// "direct" は backup_dir 自体に保存する。欠落時(None)は従来通り "direct" 扱い。
    /// (JS 側の session.json を Rust が読み書きしても、この項目が消えないようにするための定義)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_dir_mode: Option<String>,
    pub active: bool,
    pub backup_mode: String,
    pub compress_mode: String,
    #[serde(default)]
    pub selected_target_dir: String,
    #[serde(default)]
    pub is_locked: bool,
    /// hdiffz -g オプションに渡す除外パターンリスト（タブごと）
    #[serde(default)]
    pub hdiff_ignore_list: Vec<String>,
}

// session.json のルート構造体
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionData {
    pub tabs: Vec<TabSession>,
    #[serde(default)]
    pub recent_files: Vec<String>,
}

// 世代管理を司る構造体 (JSに送らない場合は Serialize 不要ですが、一応付与)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenerationManager {
    pub backup_root: String, // cg_backup_元ファイル名/ のパス
    pub threshold: f64,      // ベース更新の閾値 (例: 0.8 = 80%)
}

// 現在の世代情報
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupGenInfo {
    pub dir_path: PathBuf,
    pub base_idx: i32,
}
