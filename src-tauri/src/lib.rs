//! EmoteForge デスクトップアプリ（Tauri v2）。

mod commands;
mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let st = AppState::load(app.handle()).map_err(|e| {
                eprintln!("[EmoteForge] state load error: {e}");
                std::io::Error::new(std::io::ErrorKind::Other, e)
            })?;
            app.manage(st);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::generate_emote,
            commands::validate_emote,
            commands::search_catalog,
            commands::preview_emote,
            commands::stop_preview,
            commands::set_bridge_url,
            commands::set_codex_model,
            commands::export_emotes,
            commands::install_bridge_resource,
            commands::import_bvh_ycd_xml,
            commands::generate_ai_motion_ycd_xml,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EmoteForge");
}
