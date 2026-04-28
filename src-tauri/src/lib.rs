pub mod app;
pub mod core;
use crate::app::commands::*;

use crate::app::setup;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            setup::init(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_always_on_top,
            get_restore_previous_state,
            get_auto_base_generation_threshold,
            get_language_text,
            get_i18n,
            set_language,
            get_config_dir,
            backup_or_diff,
            apply_multi_diff,
            copy_backup_file,
            archive_backup_file,
            dir_exists,
            file_exists,
            restore_backup,
            get_file_size,
            select_any_file,
            select_any_folder,
            open_directory,
            toggle_compact_mode,
            write_text_file,
            read_text_file,
            get_backup_list,
            archive_generation,
            get_generation_folders,
            clear_all_caches,
            rebuild_archive_caches,
            prepare_archive_cache,
            get_rebuild_cache_on_startup,
            get_show_memo_after_backup,
            get_startup_cache_limit,
            get_config,
            update_config_value,
            update_session_tab_value
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
