use crate::core::types::AppConfig;
use serde_json::Value;

const BACKUP_MODES: [&str; 3] = ["copy", "archive", "diff"];
const NOTIFY_STYLES: [&str; 3] = ["popup", "sound", "both"];
const REMINDER_METHODS: [&str; 2] = ["popup", "sound"];
const REMINDER_INTERVALS: [u64; 3] = [15, 30, 60];

fn as_bool(key: &str, value: &Value) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| format!("`{}` must be a boolean", key))
}

fn as_u64(key: &str, value: &Value) -> Result<u64, String> {
    value
        .as_u64()
        .ok_or_else(|| format!("`{}` must be a non-negative integer", key))
}

fn as_choice(key: &str, value: &Value, choices: &[&str]) -> Result<String, String> {
    let s = value
        .as_str()
        .ok_or_else(|| format!("`{}` must be a string", key))?;
    if choices.contains(&s) {
        Ok(s.to_string())
    } else {
        Err(format!(
            "`{}` must be one of {:?} (got `{}`)",
            key, choices, s
        ))
    }
}

/// ショートカット文字列("Ctrl+Alt+B" など)。空文字は「未割り当て」として許可する。
/// 実際に登録できるかどうか(他アプリとの重複など)は、登録時に判定する。
fn as_shortcut(key: &str, value: &Value) -> Result<String, String> {
    let s = value
        .as_str()
        .ok_or_else(|| format!("`{}` must be a string", key))?
        .trim()
        .to_string();
    if s.len() > 64 || s.chars().any(|c| c.is_control()) {
        return Err(format!("`{}` is not a valid shortcut", key));
    }
    Ok(s)
}

/// 設定項目を1つ更新する(値の検証つき)。保存は呼び出し側で行う。
pub fn apply_config_value(cfg: &mut AppConfig, key: &str, value: &Value) -> Result<(), String> {
    match key {
        // --- 真偽値 ---
        "hdiffStrictHashCheck" => cfg.hdiff_strict_hash_check = as_bool(key, value)?,
        "strictFileNameMatch" => cfg.strict_file_name_match = as_bool(key, value)?,
        "restorePreviousState" => cfg.restore_previous_state = as_bool(key, value)?,
        "rebuildCacheOnStartup" => cfg.rebuild_cache_on_startup = as_bool(key, value)?,
        "useSameDirForTemp" => cfg.use_same_dir_for_temp = as_bool(key, value)?,
        "showMemoAfterBackup" => cfg.show_memo_after_backup = as_bool(key, value)?,
        "globalShortcutEnabled" => cfg.global_shortcut_enabled = as_bool(key, value)?,
        "notifyEnabled" => cfg.notify_enabled = as_bool(key, value)?,
        "notifyOnlyOnFailure" => cfg.notify_only_on_failure = as_bool(key, value)?,
        "reminderEnabled" => cfg.reminder_enabled = as_bool(key, value)?,
        "reminderSkipUnchanged" => cfg.reminder_skip_unchanged = as_bool(key, value)?,
        "openAfterRestore" => cfg.open_after_restore = as_bool(key, value)?,

        // --- 数値 ---
        "startupCacheLimit" => cfg.startup_cache_limit = as_u64(key, value)? as usize,
        "autoBaseGenerationThreshold" => {
            let v = value
                .as_f64()
                .ok_or_else(|| format!("`{}` must be a number", key))?;
            if !(0.1..=1.0).contains(&v) {
                return Err(format!("`{}` must be between 0.1 and 1.0", key));
            }
            cfg.auto_base_generation_threshold = v;
        }
        "notifyDurationSec" => {
            let v = as_u64(key, value)?;
            if !(1..=5).contains(&v) {
                return Err(format!("`{}` must be between 1 and 5", key));
            }
            cfg.notify_duration_sec = v as u32;
        }
        "reminderIntervalMin" => {
            let v = as_u64(key, value)?;
            if !REMINDER_INTERVALS.contains(&v) {
                return Err(format!("`{}` must be one of {:?}", key, REMINDER_INTERVALS));
            }
            cfg.reminder_interval_min = v as u32;
        }

        // --- 選択肢 ---
        "notifyStyle" => cfg.notify_style = as_choice(key, value, &NOTIFY_STYLES)?,
        "reminderMethod" => cfg.reminder_method = as_choice(key, value, &REMINDER_METHODS)?,
        "shortcutBackupMode" => cfg.shortcut_backup_mode = as_choice(key, value, &BACKUP_MODES)?,

        // --- ショートカット ---
        "shortcutRun" => cfg.shortcut_run = as_shortcut(key, value)?,
        "shortcutRunWithMemo" => cfg.shortcut_run_with_memo = as_shortcut(key, value)?,

        _ => return Err(format!("Unknown config key: {}", key)),
    }
    Ok(())
}
