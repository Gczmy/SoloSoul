pub mod android_glass_plugin;
pub mod attachment_import_plugin;
pub mod commands;
pub mod fs;
pub mod keystore_plugin;
pub mod local_embed;
pub mod lock_state_plugin;
pub mod mobile_ocr_plugin;
pub mod network_status_plugin;
pub mod nsd_plugin;
pub mod plugin;
pub mod preview_pdf_protocol;
pub mod services;
pub mod setup;
pub mod state;

pub mod status_bar_plugin;
pub mod sync;
pub mod update_plugin;

/// 涉及 VaultService 的测试共用此锁，避免账户生命周期测试并发干扰。
#[cfg(test)]
pub(crate) static VAULT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
//   plugin_* / create_plugin_install → register_plugin_commands（插件市场）
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
        commands::object::object_field_suggestions,
        commands::object::object_create,
        commands::object::object_update,
        commands::object::object_delete,
        commands::object::object_sync_with_template,
        commands::object::object_ignore_template_sync,
        commands::object::object_list_deprecated_fields,
        commands::object::object_trash_list,
        commands::object::trash_restore,
        commands::object::trash_restore_batch,
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
        commands::cloud_targets::cloud_targets_detect,
        commands::settings::cloud_sync_get_config,
        commands::settings::cloud_sync_save_config,
        commands::settings::cloud_sync_delete_config,
        commands::settings::cloud_sync_test_connection,
        commands::settings::cloud_sync_now,
        commands::settings::cloud_sync_mark_applied,
        commands::settings::cloud_sync_list_incoming,
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
        commands::attachment::tree::attachment_count_stats,
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
        commands::window::set_titlebar_controls,
        commands::window::get_window_layout,
        commands::window::show_main_window,
        status_bar_plugin::set_status_bar_style,
        android_glass_plugin::android_glass_capabilities,
        android_glass_plugin::android_show_glass_menu,
        android_glass_plugin::android_close_glass_menu,
        lock_state_plugin::dismiss_lock_mask,
        lock_state_plugin::get_lock_pending,
        commands::update::create_update_download,
        commands::update::cancel_update_download,
        // Android 更新命令
        commands::update::android_cached_update,
        commands::update::android_check_update,
        commands::update::android_download_apk,
        commands::update::android_get_apk_path,
        commands::update::android_is_apk_downloaded,
        update_plugin::android_install_apk,
        // 桌面端更新检查命令（仅桌面端编译）
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        commands::update::desktop_check_update,
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        commands::update::desktop_prepare_update,
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        commands::update::desktop_download_update,
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        commands::update::desktop_install_update,
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
        commands::sync::sync_rename_peer,
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
        commands::plugin::create_plugin_install,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IpcCommandCluster {
    Core,
    Sync,
    Ocr,
    Llm,
    Plugin,
}

/// 分发器与回归测试共用此路由，避免测试另写一套前缀判断而漏掉实际失配。
fn ipc_command_cluster(cmd: &str) -> IpcCommandCluster {
    if cmd.starts_with("sync_") || cmd.starts_with("recovery_") || cmd.starts_with("mdns_") {
        IpcCommandCluster::Sync
    } else if cmd.starts_with("ocr_") || cmd.starts_with("mobile_ocr_") {
        IpcCommandCluster::Ocr
    } else if cmd.starts_with("llm_") || cmd.starts_with("guide_") {
        IpcCommandCluster::Llm
    } else if cmd.starts_with("plugin_") || cmd == "create_plugin_install" {
        // 安装取消资源的创建命令沿用 create_* 命名，必须显式进入插件簇。
        IpcCommandCluster::Plugin
    } else {
        IpcCommandCluster::Core
    }
}

/// IPC 命令分发器：按前缀及明确的例外路由，簇内仍按完整命令名精确匹配。
/// 新增命令需加入对应 generate_handler!、同步 ACL；不符合现有前缀时同时更新
/// ipc_command_cluster。测试从注册列表提取命令并验证实际路由，无需另维护一份列表。
fn dispatch_ipc(invoke: tauri::ipc::Invoke<tauri::Wry>) -> bool {
    match ipc_command_cluster(invoke.message.command()) {
        IpcCommandCluster::Core => register_core_commands()(invoke),
        IpcCommandCluster::Sync => register_sync_commands()(invoke),
        IpcCommandCluster::Ocr => register_ocr_commands()(invoke),
        IpcCommandCluster::Llm => register_llm_commands()(invoke),
        IpcCommandCluster::Plugin => register_plugin_commands()(invoke),
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
        .plugin(android_glass_plugin::init())
        .plugin(lock_state_plugin::init())
        .plugin(network_status_plugin::init())
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
        let mut window_state = tauri_plugin_window_state::Builder::new();
        if cfg!(any(target_os = "macos", target_os = "windows")) {
            // 最大化/全屏恢复也可能显示窗口，统一推迟到品牌首帧就绪。
            window_state = window_state.skip_initial_state("main").with_state_flags(
                tauri_plugin_window_state::StateFlags::all()
                    - tauri_plugin_window_state::StateFlags::VISIBLE,
            );
        }
        builder = builder
            .plugin(window_state.build())
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
    use super::{ipc_command_cluster, IpcCommandCluster};
    use std::collections::HashSet;

    /// 直接检查注册源码与生产路由，覆盖所有平台的条件命令；不复制命令名或前缀表。
    #[test]
    fn test_dispatch_cluster_prefixes_consistent() {
        let source = include_str!("lib.rs");
        let mut registered = HashSet::new();
        for (name, expected) in [
            ("core", IpcCommandCluster::Core),
            ("sync", IpcCommandCluster::Sync),
            ("ocr", IpcCommandCluster::Ocr),
            ("llm", IpcCommandCluster::Llm),
            ("plugin", IpcCommandCluster::Plugin),
        ] {
            let function = format!("fn register_{name}_commands(");
            let body = source.split_once(&function).expect("注册函数存在").1;
            let block = body
                .split_once("tauri::generate_handler![")
                .expect("注册宏存在")
                .1
                .split_once("\n    ]")
                .expect("注册宏结束")
                .0;
            let mut count = 0;
            for line in block.lines().map(str::trim) {
                if line.starts_with("//") || line.starts_with("#[") || line.is_empty() {
                    continue;
                }
                let (_, command) = line
                    .trim_end_matches(',')
                    .rsplit_once("::")
                    .expect("每行注册一个完整命令路径");
                assert!(registered.insert(command), "命令 {command} 重复注册");
                assert_eq!(
                    ipc_command_cluster(command),
                    expected,
                    "命令 {command} 无法路由到已注册的 {name} 簇"
                );
                count += 1;
            }
            assert!(count > 0, "{name} 簇应包含命令");
        }
        assert!(registered.contains("create_plugin_install"));
        assert!(registered.contains("create_update_download"));
        assert_eq!(
            ipc_command_cluster("unknown_command"),
            IpcCommandCluster::Core
        );
    }
}
