use tauri::Manager;

pub mod commands;
pub mod core;
pub mod db;
pub mod ipc;
pub mod services;
pub mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            let app_state = AppState::new(app.handle().clone())?;
            app.manage(app_state);
            app.manage(commands::discovery::SharedDaemon::new());
            app.manage(core::sensitivity::SensitivityManager::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth commands
            commands::auth::check_has_account,
            commands::auth::bootstrap,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::get_current_account,
            // Vault commands
            commands::vault::unlock,
            commands::discovery::mdns_advertise,
            // Vault-locked event: broadcast when vault is locked
            commands::vault::lock,

            // Object commands (UnifiedObject → Object, per 21_矛盾冲突)
            commands::object::object_list,
            commands::object::object_get,
            commands::object::object_create,
            commands::object::object_update,
            commands::object::object_delete,

            // Sensitivity commands
            commands::sensitivity::sensitivity_get_field,
            commands::sensitivity::sensitivity_get_map,
            commands::sensitivity::sensitivity_update_field,
            commands::sensitivity::sensitivity_get_log,

            // Search commands
            commands::search::search_unified,

            // Export/Import commands
            commands::export_import::export_get_scope_tree,
            commands::export_import::export_estimate_size,
            commands::export_import::export_execute,

            // Import commands
            commands::export_import::import_preview_package,
            commands::export_import::import_execute,
            commands::vault::get_state,
            commands::vault::change_password,
            commands::vault::delete_account,
            commands::vault::list_accounts,
            // Profile commands
            commands::profile::profile_save,
            commands::profile::profile_load,
            commands::profile::profile_list,
            commands::profile::profile_delete,
            // Crypto commands
            commands::crypto::encrypt_bytes,
            commands::crypto::decrypt_bytes,
            commands::crypto::encrypt_with_key,
            commands::crypto::decrypt_with_key,
            commands::crypto::derive_key,
            commands::crypto::generate_salt,
            commands::crypto::constant_time_compare,
            commands::crypto::get_vault_stats,
            // File System commands
            commands::fs::encrypt_file,
            commands::fs::decrypt_file,
            commands::fs::create_zip_package,
            commands::fs::extract_zip_package,
            commands::fs::inspect_backup,
            // Discovery commands
            commands::discovery::mdns_discover,
            commands::discovery::mdns_advertise,
            // System commands
            commands::system::get_app_info,
            commands::system::check_version,
            // Log commands
            commands::log::log_get_recent,
            commands::log::log_export,
            // Backup commands
            commands::backup::backup_list,
            commands::backup::backup_create,
            commands::backup::backup_restore,
            commands::backup::backup_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
