use std::fs;
use tempfile::tempdir;
use work_backup_tool::app::commands::list_dir_entries;
use work_backup_tool::core::types::{SessionData, TabSession};

// =====================================================================
// list_dir_entries (保存先フォルダの中身検査用コマンド)
// =====================================================================

#[test]
fn test_list_dir_entries_returns_sorted_names() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("work.clip.20260101_120000.diff"), b"x").unwrap();
    fs::write(root.join("work.clip.base"), b"x").unwrap();
    fs::create_dir(root.join("base1_20260101_120000")).unwrap();
    fs::create_dir(root.join("wbt_backup_work")).unwrap();

    let names = list_dir_entries(root.to_string_lossy().into_owned()).unwrap();
    assert_eq!(
        names,
        vec![
            "base1_20260101_120000",
            "wbt_backup_work",
            "work.clip.20260101_120000.diff",
            "work.clip.base",
        ]
    );
}

#[test]
fn test_list_dir_entries_missing_dir_is_empty() {
    let names = list_dir_entries("/definitely/not/a/real/dir/wbt".to_string()).unwrap();
    assert!(names.is_empty());
}

// =====================================================================
// session.json のタブ: backupDirMode の後方互換
// =====================================================================

fn tab_json(extra: &str) -> String {
    format!(
        r#"{{"id":1,"workFile":"a.clip","backupDir":"/backups","active":true,"backupMode":"diff","compressMode":"zstd"{}}}"#,
        extra
    )
}

#[test]
fn test_tab_session_without_backup_dir_mode_is_none() {
    // 旧 session.json(フィールド無し)は None = 従来通り「直接」扱い
    let tab: TabSession = serde_json::from_str(&tab_json("")).unwrap();
    assert_eq!(tab.backup_dir_mode, None);
    // None は書き出しでも省略され、旧形式のままになる
    let out = serde_json::to_value(&tab).unwrap();
    assert!(out.get("backupDirMode").is_none());
}

#[test]
fn test_tab_session_with_backup_dir_mode_roundtrips() {
    let tab: TabSession =
        serde_json::from_str(&tab_json(r#","backupDirMode":"parent""#)).unwrap();
    assert_eq!(tab.backup_dir_mode.as_deref(), Some("parent"));

    let session: SessionData = serde_json::from_str(
        &format!(r#"{{"tabs":[{}],"recentFiles":[]}}"#, tab_json(r#","backupDirMode":"direct""#)),
    )
    .unwrap();
    assert_eq!(session.tabs[0].backup_dir_mode.as_deref(), Some("direct"));
    let out = serde_json::to_string(&session).unwrap();
    assert!(out.contains(r#""backupDirMode":"direct""#));
}
