//! Crate do app: só a ponte entre o Tauri e o `anansi-core` (Seção 3 da spec).

mod commands;
mod events;
mod prefs;
mod state;

use state::AppState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::workspace::workspace_open,
            commands::workspace::workspace_init,
            commands::workspace::workspace_current,
            commands::workspace::workspace_tree,
            commands::workspace::item_request_count,
            commands::collections::collection_create,
            commands::collections::collection_update,
            commands::collections::collection_delete,
            commands::folders::folder_create,
            commands::folders::folder_rename,
            commands::folders::folder_delete,
            commands::requests::request_get,
            commands::requests::request_create,
            commands::requests::request_save,
            commands::requests::request_delete,
            commands::requests::request_duplicate,
            commands::environments::env_list,
            commands::environments::env_get,
            commands::environments::env_create,
            commands::environments::env_delete,
            commands::environments::env_save_variables,
            commands::environments::env_save_secrets,
            commands::environments::env_reveal_secret,
            commands::environments::env_get_active,
            commands::environments::env_set_active,
            commands::environments::env_variable_names,
            commands::environments::gitignore_status,
            commands::environments::gitignore_fix,
            commands::git::git_status,
            commands::git::git_pull,
            commands::git::git_push,
            commands::http::http_send,
            commands::http::http_cancel,
            commands::http::http_save_body,
            commands::http::http_release,
            commands::curl::curl_export,
            commands::curl::curl_import,
            commands::settings::prefs_get,
            commands::settings::prefs_set,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o anansi");
}
