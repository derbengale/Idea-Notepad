mod commands;
mod db;
mod drive;
mod dto;
mod error;
mod export;
mod logging;
mod markdown;
mod schema;
mod state;
mod util;

use state::AppState;
use std::time::Duration;
use tauri::Manager;

pub fn run() {
    let paths = state::resolve_paths();
    let _log_guard = match logging::init(&paths.log_dir) {
        Ok(guard) => Some(guard),
        Err(error) => {
            eprintln!("Cannot initialise logging: {error}");
            None
        }
    };

    let state = match AppState::new(paths) {
        Ok(state) => state,
        Err(error) => {
            tracing::error!(error = %error, "cannot start Better Notepad");
            return;
        }
    };
    let setup_state = state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(state)
        .setup(move |app| {
            // The mirror directory is resolved at runtime (it can be relocated with
            // BETTER_NOTEPAD_DATA_DIR), so the asset protocol scope is extended here
            // instead of relying on the static pattern in tauri.conf.json.
            if let Err(error) = app
                .asset_protocol_scope()
                .allow_directory(setup_state.mirror_dir(), true)
            {
                tracing::error!(error = %error, "cannot expose the mirror directory to the webview");
            }
            setup_state.attach_app(app.handle().clone());
            let worker = setup_state.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(5));
                loop {
                    let signed_in = crate::drive::sync::auth_status(&worker)
                        .map(|status| status.signed_in)
                        .unwrap_or(false);
                    if signed_in
                        && !worker
                            .inner
                            .sync
                            .paused
                            .load(std::sync::atomic::Ordering::SeqCst)
                    {
                        if let Err(error) = crate::drive::sync::run_cycle(&worker) {
                            tracing::warn!(error = %error, "background sync failed");
                        }
                    }
                    std::thread::sleep(Duration::from_secs(300));
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::app::app_open_data_dir,
            commands::app::settings_get,
            commands::app::settings_set,
            commands::app::diagnostics,
            commands::workspace::workspace_stats,
            commands::workspace::workspace_rebuild_index,
            commands::entries::entry_list,
            commands::entries::entry_get,
            commands::entries::entry_create,
            commands::entries::entry_update,
            commands::entries::entry_delete,
            commands::entries::entry_restore,
            commands::entries::entry_duplicate,
            commands::entries::entry_move_project,
            commands::entries::entry_set_tags,
            commands::entries::link_create,
            commands::entries::link_delete,
            commands::entries::link_list,
            commands::entries::tag_list,
            commands::entries::tag_create,
            commands::entries::tag_delete,
            commands::entries::search_query,
            commands::schemas::schema_list,
            commands::schemas::schema_create,
            commands::schemas::schema_update,
            commands::schemas::schema_delete,
            commands::schemas::schema_reorder,
            commands::schemas::schema_duplicate,
            commands::schemas::formula_validate,
            commands::schemas::formula_eval,
            commands::projects::project_list,
            commands::projects::project_create,
            commands::projects::project_update,
            commands::projects::project_delete,
            commands::attachments::attachment_list,
            commands::attachments::attachment_add,
            commands::attachments::attachment_delete,
            commands::attachments::attachment_open,
            commands::attachments::sketch_create,
            commands::attachments::sketch_load,
            commands::attachments::sketch_save,
            commands::attachments::sketch_delete,
            commands::export::export_entry_markdown,
            commands::export::bundle_export,
            commands::export::bundle_inspect,
            commands::export::bundle_import,
            commands::auth::auth_status,
            commands::auth::auth_sign_in,
            commands::auth::auth_sign_out,
            commands::sync::sync_status,
            commands::sync::sync_now,
            commands::sync::sync_pause,
            commands::sync::sync_resume,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Better Notepad");
}
