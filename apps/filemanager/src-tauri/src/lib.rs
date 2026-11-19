mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_dir_cmd,
            commands::list_roots_cmd,
            commands::search_cmd,
            commands::get_info_cmd,
            commands::create_dir_cmd,
            commands::rename_cmd,
            commands::delete_cmd,
            commands::transfer_cmd,
            commands::read_bytes_cmd,
            commands::read_text_cmd,
            commands::open_with_system_cmd,
            commands::storage_state_cmd,
            commands::list_places_cmd,
            commands::cancel_job_cmd,
            commands::dir_sizes_cmd,
            commands::thumbnail_cmd,
            commands::zip_cmd,
            commands::unzip_cmd,
            commands::trash_available_cmd,
            commands::bin_list_cmd,
            commands::bin_restore_cmd,
            commands::bin_purge_cmd,
            commands::bin_empty_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
