use serde::{Deserialize, Serialize};
use std::path::PathBuf;


// 設定ファイル情報
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub language: String,
    pub always_on_top: bool,
    pub restore_previous_state: bool,
    pub tray_mode: bool,
    pub auto_base_generation_threshold: f64,
    #[serde(skip_serializing, default)]
    pub compact_mode: bool,
    pub tray_backup_mode: String,
    pub use_same_dir_for_temp: bool,
    pub rebuild_cache_on_startup: bool,
    pub startup_cache_limit: usize,
    pub show_memo_after_backup: bool,
    pub strict_file_name_match: bool,
    pub hdiff_strict_hash_check: bool
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
