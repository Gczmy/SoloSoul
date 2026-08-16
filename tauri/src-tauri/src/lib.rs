pub mod attachment_import_plugin;
pub mod commands;
pub mod fs;
pub mod keystore_plugin;
pub mod local_embed;
pub mod lock_state_plugin;
pub mod mobile_ocr_plugin;
pub mod nsd_plugin;
pub mod plugin;
pub mod preview_pdf_protocol;
pub mod services;
pub mod setup;
pub mod state;

pub mod status_bar_plugin;
pub mod sync;
pub mod update_plugin;

// ─────────────────────────────────────────────────────────────────────────
// IPC 命令分簇注册
// ─────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────
// IPC 命令分簇注册（P223-③）
//
// tauri 2.11 的 `Builder::invoke_handler` 为**覆盖式**语义（`self.invoke_handler =
// Box::new(...)`，多次调用互相覆盖），因此无法像插件那样链式累加。这里采用
// 「单分发器 + 5 簇」模式：分发器 `dispatch_ipc` 读取命令名（`generate_handler!`
// 展开闭包的匹配键），按**前缀**路由到对应簇的 `generate_handler!` 闭包；未命中
// 任何前缀的其余命令全部落入核心簇（兜底）。各簇闭包内部仍按完整命令名精确匹配
// （与原先单个大列表逐字等价——分发只是把同一批路径拆到 5 个宏调用中）。
//
// 前缀路由约定（新增命令必须放入对应簇，否则会被路由到错误簇而失配返回 false）：
//   sync_* / recovery_* / mdns_*      → register_sync_commands（同步）
//   ocr_* / mobile_ocr_*             → register_ocr_commands（OCR）
//   llm_* / guide_*                  → register_llm_commands（LLM + Embedding）
//   plugin_*                         → register_plugin_commands（插件市场）
//   其余（auth/vault/object/template/…）→ register_core_commands（核心，兜底）
// ─────────────────────────────────────────────────────────────────────────

/// 核心簇：Auth / Vault / Object / Template / Search / Export-Import / FS / System /
/// Log / Backup / Settings / Biometric / PIN / Attachment / Window / Update。
fn register_core_commands(
) -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        // Auth commands
        commands::auth::check_has_account,
        commands::auth::bootstrap,
        commands::auth::login,
        commands::auth::logout,
        commands::auth::verify_password,
        commands::auth::unlock_with_password,
        commands::auth::reset_security_flags,
        // Vault commands
        commands::vault::unlock,
        commands::vault::lock,
        commands::vault_directory::vault_get_directory,
        commands::vault_directory::vault_set_directory,
        commands::vault_directory::vault_sync_to_remote,
        commands::vault_directory::vault_sync_from_remote,
        commands::vault_directory::vault_sync_background,
        commands::vault_directory::vault_check_directory,
        commands::vault_directory::init_vault_directory,
        // Object commands
        commands::object::object_list,
        commands::object::object_get,
        commands::object::object_create,
        commands::object::object_update,
        commands::object::object_delete,
        commands::object::object_sync_with_template,
        commands::object::object_ignore_template_sync,
        commands::object::object_list_deprecated_fields,
        commands::object::object_trash_list,
        commands::object::trash_restore,
        commands::object::trash_permanent_delete_batch,
        commands::object::page_delete,
        commands::object::trash_get_detail,
        commands::object::snapshot_list,
        commands::object::snapshot_count_batch,
        commands::object::snapshot_get_data,
        commands::object::snapshot_rollback,
        // Template commands
        commands::template::template_create,
        commands::template::template_update,
        commands::template::template_delete,
        commands::template::template_restore,
        commands::template::template_get,
        commands::template::template_list,
        commands::template::template_hash_map,
        commands::template::template_check_field_usage,
        // Search commands
        commands::search::search_unified,
        // Export/Import commands
        commands::export_import::export_get_scope_tree,
        commands::export_import::export_estimate_size,
        commands::export_import::export_get_attachments_batch,
        commands::export_import::export_execute,
        commands::export_import::export_document_preflight,
        commands::export_import::export_objects_document,
        commands::export_import::import_parse_package,
        commands::export_import::import_decrypt_preview,
        commands::export_import::import_execute_advanced,
        commands::vault::change_password,
        commands::vault::vault_list_accounts,
        commands::vault::vault_update_hint,
        commands::vault::vault_rename_account,
        // Profile commands
        commands::profile::profile_load,
        commands::vault::get_vault_stats,
        // File System commands
        commands::fs::fs_scan_directory,
        commands::fs::fs_get_file_size,
        commands::fs::fs_is_dir,
        commands::fs::fs_read_file_as_data_url,
        commands::fs::fs_read_image_preview,
        commands::fs::fs_read_file_as_text,
        // System commands
        commands::system::get_app_info,
        commands::system::get_system_theme,
        commands::system::get_system_locale,
        // Log commands
        commands::log::log_write,
        commands::log::log_get_recent,
        commands::log::log_export,
        // Backup commands
        commands::backup::backup_list,
        commands::backup::backup_create,
        commands::backup::backup_restore,
        commands::backup::backup_delete,
        // Settings commands
        commands::settings::user_data_get_preferences,
        commands::settings::user_data_update_preference,
        commands::settings::ui_get_preferences,
        commands::settings::ui_update_preference,
        // Biometric commands
        commands::biometric::biometric_check_availability,
        commands::biometric::biometric_save_credential,
        commands::biometric::biometric_unlock,
        commands::biometric::biometric_delete_credential,
        commands::biometric::biometric_test,
        // PIN commands
        commands::pin::pin_check_availability,
        commands::pin::pin_setup,
        commands::pin::pin_unlock,
        commands::pin::pin_disable,
        // Attachment commands
        commands::attachment::crud::attachment_list,
        commands::attachment::crud::attachment_save,
        commands::attachment::crud::attachment_soft_delete,
        commands::attachment::crud::attachment_batch_soft_delete,
        commands::attachment::crud::attachment_batch_restore,
        commands::attachment::crud::attachment_batch_delete,
        commands::attachment::crud::attachment_restore,
        commands::attachment::crud::attachment_rename,
        commands::attachment::crud::attachment_update_meta,
        commands::attachment::crud::attachment_delete,
        commands::attachment::crud::attachment_count_batch,
        commands::attachment::crud::attachment_copy_to_vault,
        commands::attachment::tree::attachment_list_all,
        commands::attachment::attachment_download,
        commands::attachment::attachment_open,
        commands::attachment::share::attachment_share,
        attachment_import_plugin::attachment_import_content_uri,
        attachment_import_plugin::attachment_export_content_uri,
        attachment_import_plugin::attachment_export_tree_uri,
        attachment_import_plugin::attachment_pick_tree_uri,
        attachment_import_plugin::copy_content_uri_to_path,
        attachment_import_plugin::vault_pick_directory,
        // Window chrome commands
        commands::window::set_titlebar_color,
        status_bar_plugin::set_status_bar_style,
        lock_state_plugin::dismiss_lock_mask,
        lock_state_plugin::get_lock_pending,
        // Android 更新命令
        commands::update::android_check_update,
        commands::update::android_download_apk,
        commands::update::android_get_apk_path,
        commands::update::android_is_apk_downloaded,
        update_plugin::android_install_apk,
        // 桌面端更新检查命令（仅桌面端编译）
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        commands::update::desktop_check_update,
    ]
}

/// 同步簇：Sync / Recovery / Discovery。
fn register_sync_commands(
) -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        // Sync commands
        commands::sync::sync_get_status,
        commands::sync::sync_enable,
        commands::sync::sync_listen_addr,
        commands::sync::sync_generate_qr_payload,
        commands::sync::sync_with_device,
        commands::sync::sync_trust_peer,
        commands::sync::sync_forget_peer,
        commands::sync::sync_trigger_foreground,
        commands::sync::sync_set_auto_enabled,
        commands::sync::sync_get_auto_status,
        commands::sync::sync_set_ui_prefs_sync,
        commands::sync::sync_get_ui_prefs_sync,
        commands::sync::sync_list_conflicts,
        commands::sync::sync_get_conflict_detail,
        commands::sync::sync_resolve_conflict,
        // Recovery commands
        commands::recovery::recovery_host_start,
        commands::recovery::recovery_host_cancel,
        commands::recovery::recovery_restore_from_host,
        // Discovery commands
        commands::discovery::mdns_discover,
        commands::discovery::recovery_discover_hosts,
    ]
}

/// OCR 簇：OCR（PP-OCRv6）+ 移动端拍照。
fn register_ocr_commands() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static
{
    tauri::generate_handler![
        // OCR commands
        mobile_ocr_plugin::mobile_ocr_take_photo,
        commands::ocr::ocr_scan_image,
        commands::ocr::ocr_scan_mrz,
        commands::ocr::ocr_list_available_tiers,
        commands::ocr::ocr_get_active_tier,
        commands::ocr::ocr_set_active_tier,
        commands::ocr::ocr_get_model_status,
        commands::ocr::ocr_install_bundled_model,
        commands::ocr::ocr_install_bundled_model_with_progress,
        commands::ocr::ocr_download_model,
        commands::ocr::ocr_delete_model,
    ]
}

/// LLM 簇：LLM 会话 / 指南 / Embedding（前缀 llm_/guide_）。
fn register_llm_commands() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static
{
    tauri::generate_handler![
        // LLM commands
        commands::llm::llm_get_config,
        commands::llm::llm_get_providers,
        commands::llm::llm_save_provider,
        commands::llm::llm_set_active_provider,
        commands::llm::llm_set_ai_features,
        commands::llm::llm_set_system_prompt_switch,
        commands::llm::llm_accept_risk,
        commands::llm::llm_delete_provider,
        commands::llm::llm_get_api_key,
        commands::llm::llm_test_provider,
        commands::llm::llm_list_conversations,
        commands::llm::llm_get_conversation,
        commands::llm::llm_save_conversation,
        commands::llm::llm_rename_conversation,
        commands::llm::llm_soft_delete_conversation,
        commands::llm::llm_restore_conversation,
        commands::llm::llm_permanent_delete,
        commands::llm::llm_list_trash,
        commands::llm::llm_check_connection,
        commands::llm::llm_get_stats,
        commands::llm::llm_reset_stats,
        commands::llm::llm_send_message_stream,
        commands::llm::guide_load_index,
        commands::llm::guide_load_content,
        commands::llm::guide_search,
        commands::llm::llm_search_guide_chunks,
        commands::llm::llm_rebuild_guide_embeddings,
        commands::llm::llm_check_embedding_available,
        commands::llm::llm_set_local_embedding,
        // Embedding model commands
        commands::embed_model::llm_get_embed_models,
        commands::embed_model::llm_download_embed_model,
        commands::embed_model::llm_delete_embed_model,
    ]
}

/// 插件市场簇：Plugin。
fn register_plugin_commands(
) -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        // Plugin commands
        commands::plugin::plugin_list_all,
        commands::plugin::plugin_list_installed,
        commands::plugin::plugin_list_attachments,
        commands::plugin::plugin_install,
        commands::plugin::plugin_update,
        commands::plugin::plugin_uninstall,
        commands::plugin::plugin_run,
        commands::plugin::plugin_consent_response,
        commands::plugin::plugin_dialog_response,
        commands::plugin::plugin_list_sessions,
        commands::plugin::plugin_audit_log,
        commands::plugin::plugin_update_registry,
        commands::plugin::plugin_open_output_file,
        commands::plugin::plugin_copy_output_file,
    ]
}

/// IPC 命令分发器：按命令名前缀路由到对应簇，未命中前缀的落入核心簇（兜底）。
/// 语义与原先单个 `generate_handler!` 大列表逐字等价（分发 + 簇内精确匹配双层兜底）。
///
/// ⚠️ P042 维护提醒：新增命令时必须同步三处——①加入对应 `register_*_commands` 的
/// `generate_handler!` 列表；②若新命令前缀不在现有路由条件下，更新此处路由；
/// ③同步 `tests::test_dispatch_cluster_prefixes_consistent` 的簇命令名/前缀映射
/// 列表（该测试断言两处一致性，防止静默失配）。
fn dispatch_ipc(invoke: tauri::ipc::Invoke<tauri::Wry>) -> bool {
    // 借用命令名（NLL：借用在其最后一次使用——路由 if 条件——后即结束，分支内 move invoke 不冲突，避免每 invoke 一次 String 分配）
    let cmd = invoke.message.command();
    if cmd.starts_with("sync_") || cmd.starts_with("recovery_") || cmd.starts_with("mdns_") {
        register_sync_commands()(invoke)
    } else if cmd.starts_with("ocr_") || cmd.starts_with("mobile_ocr_") {
        register_ocr_commands()(invoke)
    } else if cmd.starts_with("llm_") || cmd.starts_with("guide_") {
        register_llm_commands()(invoke)
    } else if cmd.starts_with("plugin_") {
        register_plugin_commands()(invoke)
    } else {
        register_core_commands()(invoke)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── 第 0 步：注册 panic hook（在一切初始化之前）──
    // 注意：此时可能还没有正确的日志目录（移动端需进入 setup 后才能解析），
    // panic 信息会先写入 stderr；setup 中设置 LOG_DIR 后则可写入文件。
    setup::setup_panic_hook();

    // ── 第 1 步：构建 Tauri 应用 ──
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(status_bar_plugin::init())
        .plugin(lock_state_plugin::init())
        .plugin(attachment_import_plugin::init())
        .plugin(nsd_plugin::init())
        .plugin(mobile_ocr_plugin::init())
        .plugin(keystore_plugin::init())
        .plugin(update_plugin::init());

    // 移动端专属插件
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        builder = builder.plugin(tauri_plugin_biometric::init());
    }

    // 桌面端专属插件
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder
            .plugin(tauri_plugin_window_state::Builder::new().build())
            .plugin(tauri_plugin_updater::Builder::new().build());
    }

    // 桌面端：solosoul-pdf:// 自定义协议——PDF 附件内嵌预览（WebView2 无法渲染
    // data:/blob: URL 的 embed，且 fs_read_file_as_data_url 有 10 MiB 上限）。
    // 移动端 PDF 预览走系统应用，无需注册该协议。
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = preview_pdf_protocol::register(builder);
    }

    let result = builder
        .setup(setup::setup_app)
        .invoke_handler(dispatch_ipc)
        .run(tauri::generate_context!());

    if let Err(e) = result {
        let err_msg = format!("{:#}", e);
        tracing::error!("[fatal] Tauri 应用启动失败: {}", e);
        eprintln!(
            "SoloSoul 启动失败: {}

请将以下信息发送给开发团队：
{}",
            e, err_msg
        );
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    /// P223-③ 前缀路由守卫：每簇命令名必须匹配 `dispatch_ipc` 的路由前缀，
    /// 防止未来新增命令被放进错误簇导致静默失配（返回 false 等同未知命令）。
    ///
    /// ⚠️ P042 维护提醒：本列表为命令名的第二份真相（第一份是各 `register_*_commands`
    /// 的 `generate_handler!` 列表）。Rust 宏输出不可内省，无法自动提取命令名，故以
    /// 本守卫测试保证两份真相同步——新增命令时**必须**同时更新对应簇的列表与本映射。
    #[test]
    fn test_dispatch_cluster_prefixes_consistent() {
        // 各簇命令名与其路由前缀的映射（与 dispatch_ipc / register_*_commands 一一对应）
        // ⚠️ 新增命令时同步更新下方命令名与总数断言（当前 total == 194）。
        let clusters: [(&str, &[&str], &[&str]); 5] = [
            (
                "sync",
                &[
                    "sync_get_status",
                    "sync_enable",
                    "sync_listen_addr",
                    "sync_generate_qr_payload",
                    "sync_with_device",
                    "sync_trust_peer",
                    "sync_forget_peer",
                    "sync_trigger_foreground",
                    "sync_set_auto_enabled",
                    "sync_get_auto_status",
                    "sync_set_ui_prefs_sync",
                    "sync_get_ui_prefs_sync",
                    "sync_list_conflicts",
                    "sync_get_conflict_detail",
                    "sync_resolve_conflict",
                    "recovery_host_start",
                    "recovery_host_cancel",
                    "recovery_restore_from_host",
                    "mdns_discover",
                    "recovery_discover_hosts",
                ],
                &["sync_", "recovery_", "mdns_"],
            ),
            (
                "ocr",
                &[
                    "mobile_ocr_take_photo",
                    "ocr_scan_image",
                    "ocr_scan_mrz",
                    "ocr_list_available_tiers",
                    "ocr_get_active_tier",
                    "ocr_set_active_tier",
                    "ocr_get_model_status",
                    "ocr_install_bundled_model",
                    "ocr_install_bundled_model_with_progress",
                    "ocr_download_model",
                    "ocr_delete_model",
                ],
                &["ocr_", "mobile_ocr_"],
            ),
            (
                "llm",
                &[
                    "llm_get_config",
                    "llm_get_providers",
                    "llm_save_provider",
                    "llm_set_active_provider",
                    "llm_set_ai_features",
                    "llm_set_system_prompt_switch",
                    "llm_accept_risk",
                    "llm_delete_provider",
                    "llm_get_api_key",
                    "llm_test_provider",
                    "llm_list_conversations",
                    "llm_get_conversation",
                    "llm_save_conversation",
                    "llm_rename_conversation",
                    "llm_soft_delete_conversation",
                    "llm_restore_conversation",
                    "llm_permanent_delete",
                    "llm_list_trash",
                    "llm_check_connection",
                    "llm_get_stats",
                    "llm_reset_stats",
                    "llm_send_message_stream",
                    "guide_load_index",
                    "guide_load_content",
                    "guide_search",
                    "llm_search_guide_chunks",
                    "llm_rebuild_guide_embeddings",
                    "llm_check_embedding_available",
                    "llm_set_local_embedding",
                    "llm_get_embed_models",
                    "llm_download_embed_model",
                    "llm_delete_embed_model",
                ],
                &["llm_", "guide_"],
            ),
            (
                "plugin",
                &[
                    "plugin_list_all",
                    "plugin_list_installed",
                    "plugin_list_attachments",
                    "plugin_install",
                    "plugin_update",
                    "plugin_uninstall",
                    "plugin_run",
                    "plugin_consent_response",
                    "plugin_dialog_response",
                    "plugin_list_sessions",
                    "plugin_audit_log",
                    "plugin_update_registry",
                    "plugin_open_output_file",
                    "plugin_copy_output_file",
                ],
                &["plugin_"],
            ),
            (
                "core",
                &[
                    "check_has_account",
                    "bootstrap",
                    "login",
                    "logout",
                    "verify_password",
                    "unlock_with_password",
                    "reset_security_flags",
                    "unlock",
                    "lock",
                    "vault_get_directory",
                    "vault_set_directory",
                    "vault_sync_to_remote",
                    "vault_sync_from_remote",
                    "vault_sync_background",
                    "vault_check_directory",
                    "init_vault_directory",
                    "object_list",
                    "object_get",
                    "object_create",
                    "object_update",
                    "object_delete",
                    "object_sync_with_template",
                    "object_ignore_template_sync",
                    "object_list_deprecated_fields",
                    "object_trash_list",
                    "trash_restore",
                    "trash_permanent_delete_batch",
                    "page_delete",
                    "trash_get_detail",
                    "snapshot_list",
                    "snapshot_count_batch",
                    "snapshot_get_data",
                    "snapshot_rollback",
                    "template_create",
                    "template_update",
                    "template_delete",
                    "template_restore",
                    "template_get",
                    "template_list",
                    "template_hash_map",
                    "template_check_field_usage",
                    "search_unified",
                    "export_get_scope_tree",
                    "export_estimate_size",
                    "export_get_attachments_batch",
                    "export_execute",
                    "export_document_preflight",
                    "export_objects_document",
                    "import_parse_package",
                    "import_decrypt_preview",
                    "import_execute_advanced",
                    "change_password",
                    "vault_list_accounts",
                    "vault_update_hint",
                    "profile_load",
                    "get_vault_stats",
                    "fs_scan_directory",
                    "fs_get_file_size",
                    "fs_is_dir",
                    "fs_read_file_as_data_url",
                    "fs_read_image_preview",
                    "fs_read_file_as_text",
                    "get_app_info",
                    "get_system_theme",
                    "get_system_locale",
                    "log_write",
                    "log_get_recent",
                    "log_export",
                    "backup_list",
                    "backup_create",
                    "backup_restore",
                    "backup_delete",
                    "user_data_get_preferences",
                    "user_data_update_preference",
                    "ui_get_preferences",
                    "ui_update_preference",
                    "biometric_check_availability",
                    "biometric_save_credential",
                    "biometric_unlock",
                    "biometric_delete_credential",
                    "biometric_test",
                    "pin_check_availability",
                    "pin_setup",
                    "pin_unlock",
                    "pin_disable",
                    "attachment_list",
                    "attachment_save",
                    "attachment_soft_delete",
                    "attachment_batch_soft_delete",
                    "attachment_batch_restore",
                    "attachment_batch_delete",
                    "attachment_restore",
                    "attachment_rename",
                    "attachment_update_meta",
                    "attachment_delete",
                    "attachment_count_batch",
                    "attachment_copy_to_vault",
                    "attachment_list_all",
                    "attachment_download",
                    "attachment_open",
                    "attachment_share",
                    "attachment_import_content_uri",
                    "attachment_export_content_uri",
                    "attachment_export_tree_uri",
                    "attachment_pick_tree_uri",
                    "copy_content_uri_to_path",
                    "vault_pick_directory",
                    "set_titlebar_color",
                    "set_status_bar_style",
                    "dismiss_lock_mask",
                    "get_lock_pending",
                    "android_check_update",
                    "android_download_apk",
                    "android_get_apk_path",
                    "android_is_apk_downloaded",
                    "android_install_apk",
                    "desktop_check_update",
                ],
                &[],
            ),
        ];

        // 前缀路由必须不重叠：每个命令名应恰好被一个簇的前缀覆盖（core 兜底）
        let mut total = 0usize;
        for (name, cmds, prefixes) in &clusters {
            let mut routed = 0usize;
            for cmd in *cmds {
                if name == &"core" {
                    // core 是兜底：不应命中任何其他簇的前缀
                    assert!(
                        !clusters
                            .iter()
                            .filter(|(n, _, _)| *n != "core")
                            .any(|(_, _, ps)| ps.iter().any(|p| cmd.starts_with(p))),
                        "core 命令 {cmd} 命中其他簇前缀"
                    );
                } else {
                    assert!(
                        prefixes.iter().any(|p| cmd.starts_with(p)),
                        "簇 {name} 命令 {cmd} 未命中自身前缀 {:?}",
                        prefixes
                    );
                    routed += 1;
                }
                total += 1;
            }
            // core 为兜底簇，不参与 routed 断言
            if name != &"core" {
                assert_eq!(routed, cmds.len());
            }
        }
        // 共 194 条命令全覆盖（核心 117 + 同步 20 + OCR 11 + LLM 32 + 插件 14）
        assert_eq!(total, 194);
    }
}
