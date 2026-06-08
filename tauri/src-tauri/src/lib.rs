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
            commands::auth::verify_password,
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
            commands::object::object_trash_list,
            commands::object::object_restore,
            commands::object::object_purge,
            commands::object::trash_restore,
            commands::object::trash_permanent_delete,
            commands::object::page_delete,
            commands::object::page_restore,
            commands::object::trash_get_retention,
            commands::object::trash_set_retention,
            commands::object::trash_get_detail,
            commands::object::snapshot_list,
            commands::object::snapshot_get,
            commands::object::snapshot_count_batch,
            commands::object::snapshot_get_data,
            commands::object::snapshot_rollback,
            commands::template::template_save_from_object,
            commands::template::template_list,
            // Sensitivity commands
            commands::sensitivity::sensitivity_get_field,
            commands::sensitivity::sensitivity_get_map,
            commands::sensitivity::sensitivity_update_field,
            commands::sensitivity::sensitivity_get_log,
            // Search commands
            commands::search::search_unified,
            commands::search::search_advanced,
            // Export/Import commands (P0+P1)
            commands::export_import::export_get_scope_tree,
            commands::export_import::export_estimate_size,
            commands::export_import::export_get_attachments,
            commands::export_import::export_execute,
            commands::export_import::import_parse_package,
            commands::export_import::import_get_password_hint,
            commands::export_import::import_decrypt_preview,
            commands::export_import::import_execute,
            commands::export_import::import_execute_advanced,
            commands::vault::get_state,
            commands::vault::change_password,
            commands::vault::delete_account,
            commands::vault::list_accounts,
            commands::vault::vault_update_hint,
            // Profile commands
            commands::profile::profile_save,
            commands::profile::profile_load,
            commands::profile::profile_list,
            commands::profile::profile_delete,
            commands::profile::profile_get_section,
            commands::profile::profile_update_field,
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
            commands::fs::fs_scan_directory,
            commands::fs::fs_get_file_size,
            commands::fs::fs_read_file_as_data_url,
            // Discovery commands
            commands::discovery::mdns_discover,
            // System commands
            commands::system::get_app_info,
            commands::system::check_version,
            // Log commands
            commands::log::log_write,
            commands::log::log_get_recent,
            commands::log::log_export,
            // Backup commands
            commands::backup::backup_list,
            commands::backup::backup_create,
            commands::backup::backup_restore,
            commands::backup::backup_delete,
            // Settings / user_data commands
            commands::settings::user_data_get_preferences,
            commands::settings::user_data_update_preference,
            commands::settings::ui_get_preferences,
            commands::settings::ui_update_preference,
            // LLM commands
            commands::llm::llm_get_config,
            commands::llm::llm_get_providers,
            commands::llm::llm_save_provider,
            commands::llm::llm_set_active_provider,
            commands::llm::llm_set_ai_features,
            commands::llm::llm_accept_risk,
            commands::llm::llm_delete_provider,
            commands::llm::llm_get_api_key,
            commands::llm::llm_test_provider,
            commands::llm::llm_send_message,
            commands::llm::llm_list_conversations,
            commands::llm::llm_get_conversation,
            commands::llm::llm_save_conversation,
            commands::llm::llm_delete_conversation,
            commands::llm::llm_rename_conversation,
            commands::llm::llm_soft_delete_conversation,
            commands::llm::llm_restore_conversation,
            commands::llm::llm_permanent_delete,
            commands::llm::llm_list_trash,
            commands::llm::llm_check_connection,
            // Biometric commands
            commands::biometric::biometric_check_availability,
            commands::biometric::biometric_save_credential,
            commands::biometric::biometric_unlock,
            commands::biometric::biometric_delete_credential,
            commands::biometric::biometric_test,
            // OCR commands
            commands::ocr::ocr_scan_image,
            commands::ocr::ocr_get_supported_languages,
            // Attachment commands
            commands::attachment::attachment_list,
            commands::attachment::attachment_save,
            commands::attachment::attachment_soft_delete,
            commands::attachment::attachment_restore,
            commands::attachment::attachment_rename,
            commands::attachment::attachment_delete,
            commands::attachment::attachment_count_batch,
            commands::attachment::attachment_copy_to_vault,
            commands::attachment::attachment_cleanup_orphans,
            // Sync commands
            commands::sync::sync_discover,
            commands::sync::sync_get_status,
            commands::sync::sync_enable,
            commands::sync::sync_with_device,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
