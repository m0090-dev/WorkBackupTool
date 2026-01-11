use tauri::AppHandle;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use chrono::{DateTime, Local}; // 要: chrono クレート
use crate::app::utils::*;
use std::io::Cursor;

/// 純粋なバイナリ差分作成 (修正版)
pub async fn create_bsdiff(
    _app: AppHandle,
    old_file: &str,
    new_file: &str,
    diff_file: &str,
) -> Result<(), String> {
    let old_data = if Path::new(old_file).exists() {
        fs::read(old_file).map_err(|e| format!("OldFile読み込み失敗: {}", e))?
    } else {
        Vec::new()
    };
    let new_data = fs::read(new_file).map_err(|e| format!("NewFile読み込み失敗: {}", e))?;

    let mut patch_data = Vec::new();
    // bsdiff-rs は非常にメモリとCPUを食うため、100MB以下でも慎重に
    bsdiff::diff(&old_data, &new_data, &mut patch_data)
        .map_err(|e| format!("bsdiff作成失敗: {}", e))?;

    fs::write(diff_file, patch_data).map_err(|e| format!("DiffFile保存失敗: {}", e))?;
    Ok(())
}


pub async fn apply_bsdiff(
    _app: AppHandle,
    work_file: &str,
    diff_file: &str,
) -> Result<(), String> {
    let diff_path = Path::new(diff_file);
    let backup_dir = diff_path.parent().unwrap_or_else(|| Path::new("."));
    let diff_filename = diff_path.file_name().unwrap().to_string_lossy();

    // --- 1. ベースファイル名の推測 (Go版ロジック) ---
    let mut guessed_base_name = String::new();
    let count_dots = diff_filename.matches('.').count();

    if count_dots >= 3 && (diff_filename.contains(".bsdiff.") || diff_filename.contains(".hdiff.")) {
        // 新仕様: filename.YYYYMMDD_HHMMSS.algo.diff
        let parts: Vec<&str> = diff_filename.split('.').collect();
        if parts.len() >= 3 {
            // 後ろから3つを除いたものが元のファイル名
            guessed_base_name = format!("{}.base", parts[..parts.len()-3].join("."));
        }
    } else {
        // 旧仕様: filename.2024... で分割
        if let Some(base_part) = diff_filename.split(".20").next() {
            guessed_base_name = format!("{}.base", base_part);
        }
    }

    let mut base_full = backup_dir.join(&guessed_base_name);

    // 推測したベースが見つからない場合、現在の作業ファイル名.base を最終確認
    if !base_full.exists() {
        let work_filename = Path::new(work_file).file_name().unwrap().to_string_lossy();
        base_full = backup_dir.join(format!("{}.base", work_filename));
    }

    // 存在チェック
    if !base_full.exists() {
        return Err(format!("ベースファイル (.base) が見つかりません: {}", guessed_base_name));
    }

    // --- 2. 実際のパッチ処理 (bsdiff-rs + Cursor) ---
    let old_data = fs::read(&base_full).map_err(|e| format!("Base読み込み失敗: {}", e))?;
    let patch_raw = fs::read(diff_file).map_err(|e| format!("Patch読み込み失敗: {}", e))?;

    // 出力バッファ。old_dataのサイズを参考に容量を確保して高速化
    let mut patched_data = Vec::with_capacity(old_data.len());
    let mut patch_cursor = Cursor::new(patch_raw);

    // パッチ適用実行
    bsdiff::patch(&old_data, &mut patch_cursor, &mut patched_data)
        .map_err(|e| format!("bsdiffパッチ適用失敗 (データ不整合または形式エラー): {}", e))?;

    // --- 3. 出力先の決定と書き出し ---
    // Go版の outPath := autoOutputPath(workFile) を再現
    let out_path = auto_output_path(work_file);
    
    fs::write(&out_path, patched_data).map_err(|e| format!("復元ファイルの書き出し失敗: {}", e))?;

    Ok(())
}

// リストを受け取って順次適用 (Go版の ApplyMultiBsdiff 相当)

pub async fn apply_multi_bsdiff(
    _app: AppHandle,
    work_file: &str,
    diff_paths: Vec<String>,
) -> Result<(), String> {
    for dp in diff_paths {
        apply_bsdiff(_app.clone(), work_file, &dp).await?;
    }
    Ok(())
}
