use crate::core::utils;

#[tauri::command]
pub fn get_file_size(path: String) -> Result<i64, String> {
    utils::get_file_size(&path)
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    utils::read_text_file(&path)
}

#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    utils::write_text_file(&path, &content)
}

#[tauri::command]
pub fn dir_exists(path: String) -> Result<bool, String> {
    Ok(utils::dir_exists(&path))
}

#[tauri::command]
pub fn file_exists(path: String) -> Result<bool, String> {
    Ok(utils::file_exists(&path))
}
