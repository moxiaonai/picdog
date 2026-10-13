mod commands;
mod compression;
mod credentials;
mod desktop_shell;
mod error;
mod models;
mod state;
mod storage;
mod uploaders;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("failed to locate PicDog data directory: {err}"),
                )
            })?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("picdog.sqlite");
            let state = state::AppState::open(&db_path)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.message))?;

            app.manage(state);
            desktop_shell::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::batch::scan_dropped_paths,
            commands::batch::select_files,
            commands::batch::create_batch,
            commands::batch::append_to_batch,
            commands::batch::get_batch,
            commands::batch::get_latest_batch,
            commands::batch::read_image_preview,
            commands::batch::read_compare_images,
            commands::batch::clear_batch,
            commands::batch::cancel_task,
            commands::batch::reveal_output,
            commands::compression::check_compression_tools,
            commands::compression::start_compression,
            commands::upload::upload_assets,
            commands::link::copy_link,
            commands::link::write_clipboard_text,
            commands::link::mark_link_copied,
            commands::link::mark_links_copied,
            commands::routes::list_routes,
            commands::routes::select_directory,
            commands::routes::save_route,
            commands::routes::delete_route,
            commands::routes::save_route_credential,
            commands::routes::get_route_credential_status,
            commands::routes::delete_route_credential,
            commands::routes::test_route,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::history::list_history,
            commands::history::clear_history,
            commands::history::export_history
        ])
        .run(tauri::generate_context!())
        .expect("failed to run PicDog");
}
