// 標準ライブラリ
use std::fs;
use std::path::Path;

// 外部クレート

// Tauriプラグイン

// 内部モジュール (自作)

#[tauri::command]
pub fn get_file_size(path: String) -> Result<i64, String> {
    if path.is_empty() {
        return Err("path is empty".to_string());
    }

    let p = Path::new(&path);

    // ファイルのメタデータを取得 (os.Stat 相当)
    let metadata = fs::metadata(p).map_err(|e| e.to_string())?;

    // ディレクトリの場合はエラーを返す
    if metadata.is_dir() {
        return Err("path is a directory".to_string());
    }

    // サイズを返す (i64にキャスト)
    Ok(metadata.len() as i64)
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    let p = std::path::Path::new(&path);

    // 1. ファイルが存在するかチェック
    if !p.exists() {
        // Go版と同様、存在しない場合はエラーにせず空文字を返す
        return Ok("".to_string());
    }

    // 2. ファイルを読み込む
    // fs::read_to_string は UTF-8 を想定しています
    match fs::read_to_string(p) {
        Ok(content) => Ok(content),
        Err(e) => {
            // 読み込みに失敗した場合（権限不足など）はエラーを返す
            Err(format!("Failed to read file: {}", e))
        }
    }
}

/// 指定されたパスに文字列を書き込む (Goの WriteTextFile 相当)
#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    let path_obj = Path::new(&path);

    // 親ディレクトリが存在しない場合は作成する (Go版より少し親切な設計)
    if let Some(parent) = path_obj.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    // ファイル書き込み (0644相当はRustの標準的な挙動)
    fs::write(path_obj, content).map_err(|e| format!("Failed to write text file: {}", e))?;

    Ok(())
}

/// 指定されたパスがディレクトリとして存在するか確認します (Go版の DirExists 相当)
#[tauri::command]
pub fn dir_exists(path: String) -> Result<bool, String> {
    let p = Path::new(&path);
    // exists() かつ is_dir() であることを1行で判定できます
    Ok(p.is_dir())
}

/// 指定されたパスがファイルとして存在するか確認します
#[tauri::command]
pub fn file_exists(path: String) -> Result<bool, String> {
    let p = Path::new(&path);
    // exists() かつ is_file() であることを判定します
    // (ディレクトリの場合は false になります)
    Ok(p.is_file())
}
