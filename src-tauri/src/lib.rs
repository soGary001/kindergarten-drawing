mod permission;
mod settings;
mod gallery;
mod prompt;
mod image_gen;
mod asr;
mod mic;
mod commands;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_settings, set_settings, list_gallery, draw_random,
            generate_image, asr_start, asr_stop,
            save_snapshot, check_connectivity
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
