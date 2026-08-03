use std::path::PathBuf;
use std::sync::OnceLock;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri::Emitter;
use tauri::Manager;

pub mod attachment_import_plugin;
pub mod commands;
pub mod fs;
pub mod keystore_plugin;
pub mod local_embed;
pub mod lock_state_plugin;
pub mod mobile_ocr_plugin;
pub mod nsd_plugin;
pub mod plugin;
pub mod services;
pub mod state;
pub mod status_bar_plugin;
pub mod sync;
pub mod update_plugin;

use state::AppState;

/// 全局日志目录，在 panic hook 中可直接写文件（不依赖 tracing 基础设施）。
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 注册全局 panic hook，将 panic 信息写入文件日志。
/// 在 Windows Release 构建中还会弹出 MessageBox 提示用户。
fn setup_panic_hook() {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "无位置信息".to_string());
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "无法解析的 panic payload"
        };

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let msg = format!(
            "[FATAL PANIC] time={} location={} payload={}\n",
            timestamp, location, payload
        );

        // 直接写入文件日志（tracing 基础设施在 panic 时可能已不可用）
        if let Some(log_dir) = LOG_DIR.get() {
            let _ = std::fs::create_dir_all(log_dir);
            let log_path = log_dir.join("app.log");
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
            {
                use std::io::Write;
                let _ = writeln!(f, "{}", msg);
                let _ = f.flush();
            }
        }

        // 也写入 stderr（调试构建中可见）
        eprintln!("{}", msg);

        // 调用之前的 hook（默认行为，会打印 backtrace 到 stderr）
        previous_hook(panic_info);
    }));
}

/// 解析应用数据目录。
/// - 桌面端：使用 `dirs::data_dir()/com.solosoul.app`
/// - 移动端：使用 Tauri 的 `BaseDirectory::Data`，通过 `app.path()` 解析
fn resolve_app_data_dir(
    #[allow(unused_variables)] app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        app.path()
            .resolve(".", tauri::path::BaseDirectory::Data)
            .map_err(|e| format!("无法解析应用数据目录: {e}"))
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        Ok(dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("com.solosoul.app"))
    }
}

/// 解析日志目录。
fn resolve_log_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    resolve_app_data_dir(app).map(|d| d.join("logs"))
}

/// 初始化 tracing（文件 + stderr）。在移动端于 setup 中调用，以获取正确的应用私有目录。
fn init_tracing(log_dir: &PathBuf) {
    let file_appender = tracing_appender::rolling::never(log_dir, "app.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    // `ort` crate 2.x 会通过内置 tracing 输出 session 创建 / 算子分配日志，在开发模式下
    // 污染 stderr。仅在用户未提供 RUST_LOG 时应用默认收敛策略（INFO + ort=WARN）；
    // 一旦用户在环境变量里写 RUST_LOG=ort=debug 这样的表达式，则完全交由 RUST_LOG 主导，
    // 避免 add_directive / from_env_lossy 在不同 tracing-subscriber 版本下的优先级差异。
    let env_filter = std::env::var("RUST_LOG")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| tracing_subscriber::EnvFilter::new(&s))
        .unwrap_or_else(|| tracing_subscriber::EnvFilter::new("info,ort=warn"));

    // 将 guard 泄漏，确保 non-blocking writer 在进程生命周期内不会 drop
    Box::leak(Box::new(guard));

    // 使用 tracing-subscriber registry + layers，同时输出到文件和 stderr
    {
        use tracing_subscriber::prelude::*;

        let file_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(file_writer);

        let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);

        tracing_subscriber::registry()
            .with(file_layer)
            .with(stderr_layer)
            .with(env_filter)
            .init();
    }
}

// ─────────────────────────────────────────────────────────────────────────
// setup 初始化步骤（每步一个独立函数，便于阅读与单测）
// ─────────────────────────────────────────────────────────────────────────

/// 第 0 步：解析日志目录并初始化 tracing。失败为致命错误（中止启动）。
fn setup_logging(app: &tauri::AppHandle) -> Result<(), String> {
    let log_dir = match resolve_log_dir(app) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("[fatal] 无法解析日志目录: {e}");
            return Err(format!("无法解析日志目录: {e}"));
        }
    };
    let _ = std::fs::create_dir_all(&log_dir);
    LOG_DIR.set(log_dir.clone()).ok();
    init_tracing(&log_dir);

    tracing::info!("[init] SoloSoul v{} 启动", env!("CARGO_PKG_VERSION"));
    tracing::info!("[init] 日志目录: {}", log_dir.display());
    tracing::info!("[init] 目标平台: {}", std::env::consts::OS);
    Ok(())
}

/// 第 1 步：检查数据目录是否可写。
fn setup_check_data_dir(app: &tauri::AppHandle) -> Result<(), String> {
    let data_dir = match resolve_app_data_dir(app) {
        Ok(dir) => dir,
        Err(e) => {
            tracing::error!("[setup] ❌ 无法解析数据目录: {}", e);
            return Err(format!("无法解析数据目录: {e}"));
        }
    };
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        tracing::error!(
            "[setup] ❌ 数据目录不可写: {} 错误: {}",
            data_dir.display(),
            e
        );
    }
    Ok(())
}

/// 第 1.5 步：P209 迁移窗口诊断——统计仍为 legacy XOR 格式的生物识别凭证存量。
///
/// 仅输出 tel 级日志（`RUST_LOG=solo_soul=trace` 可见），供大版本发布后评估
/// 关闭 `legacy.rs` XOR 迁移路径（删除 LEGACY_XOR_KEY / legacy_xor_decrypt）
/// 的依据：当该计数持续为 0 时迁移窗口已关闭。不读取/解密任何密钥内容。
fn setup_scan_legacy_biometric(app: &tauri::AppHandle) {
    let data_dir = match resolve_app_data_dir(app) {
        Ok(dir) => dir,
        Err(e) => {
            tracing::warn!("[setup] 无法解析数据目录以扫描 legacy 生物识别凭证: {}", e);
            return;
        }
    };
    let manager = solosoul_core::biometric::BiometricManager::new(data_dir);
    let legacy_count = manager.count_legacy_key_files();
    tracing::trace!(
        "[setup] legacy biometric credential files remaining: {} (0 表示 <2.0 迁移窗口已关闭，可移除 legacy.rs XOR 路径)",
        legacy_count
    );
}

/// 第 2 步：检查资源目录与关键子目录（仅记录日志，不中止启动）。
fn setup_check_resource_dirs(app: &mut tauri::App) {
    match app.path().resource_dir() {
        Ok(resource_dir) => {
            if !resource_dir.join("SoloSoul_plugin_market").exists() {
                tracing::warn!(
                    "[setup] ⚠️  插件市场目录不存在: {} （插件功能可能不可用）",
                    resource_dir.join("SoloSoul_plugin_market").display()
                );
            }
            if !resource_dir.join("docs").exists() {
                tracing::warn!(
                    "[setup] ⚠️  文档目录不存在: {}",
                    resource_dir.join("docs").display()
                );
            }
        }
        Err(e) => {
            tracing::error!("[setup] ❌ 无法获取资源目录: {}", e);
        }
    }
}

/// 第 4 步：初始化 AppState 并管理到应用状态（关键步骤，失败时中止启动）。
/// 同时在 SAF 模式下触发一次冷启动同步。
fn setup_init_state(app: &mut tauri::App) -> Result<(), String> {
    tracing::debug!("[setup] 正在创建 AppState...");
    let app_state = match AppState::new(app.handle().clone()) {
        Ok(state) => state,
        Err(e) => {
            tracing::error!("[setup] ❌ AppState 创建失败: {:#}", e);
            return Err(format!("AppState 创建失败: {:#}", e));
        }
    };
    let has_saf_vault = app_state.has_saf_vault();
    app.manage(app_state);

    // 启动 AutoSyncManager，并在 SAF 模式下触发一次冷启动同步。
    // AutoSyncManager 内部已包含 30 秒周期兜底和 30 秒防抖逻辑，
    // 这里只需要在有 SAF 时触发一次即时同步，避免应用意外退出后数据丢失。
    if has_saf_vault {
        if let Some(state) = app.handle().try_state::<AppState>() {
            state.auto_sync.trigger_immediate();
            tracing::info!("[setup] SAF auto-sync manager started, cold-start sync triggered");
        }
    }
    Ok(())
}

/// 第 6 步：初始化 RESOURCE_DIR。
/// Android 上 Tauri 的 resource_dir 返回 asset:// URL，std::fs 无法直接读取。
/// MainActivity 已在 onCreate 中将所需资源复制到 files/app_resources/，这里优先使用它。
fn setup_init_resource_dir(app: &mut tauri::App) {
    #[cfg(target_os = "android")]
    let resource_dir: Result<PathBuf, String> = match resolve_app_data_dir(app.handle()) {
        Ok(data_dir) => Ok(data_dir.join("app_resources")),
        Err(e) => {
            tracing::error!("[setup] ❌ 无法解析数据目录以设置 RESOURCE_DIR: {}", e);
            Err(e)
        }
    };
    #[cfg(not(target_os = "android"))]
    let resource_dir = app.path().resource_dir();

    match resource_dir {
        Ok(dir) => {
            tracing::info!("[setup] RESOURCE_DIR set to: {}", dir.display());
            let _ = commands::llm::RESOURCE_DIR.set(dir.clone());
        }
        Err(e) => {
            tracing::error!(
                "[setup] ❌ 无法获取 resource_dir，RESOURCE_DIR 未设置: {}",
                e
            );
        }
    }
}

/// 第 7 步：应用启动时后台静默刷新插件注册表（不阻塞启动，失败仅记录日志）— 桌面端先行。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn setup_spawn_registry_refresh(app: &tauri::AppHandle) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        // 若未配置公钥则跳过，避免每次启动都报错
        if std::env::var("SOLOSOUL_REGISTRY_PUBKEY").is_err() {
            tracing::debug!("[plugin] SOLOSOUL_REGISTRY_PUBKEY 未配置，跳过启动时注册表刷新");
            return;
        }
        // 若 1 小时内已刷新过则跳过
        let data_dir = match resolve_app_data_dir(&app_handle) {
            Ok(dir) => dir,
            Err(e) => {
                tracing::warn!("[plugin] 无法解析数据目录，跳过注册表刷新: {}", e);
                return;
            }
        };
        let last_update_path = data_dir.join(".last_registry_update");
        let should_refresh = if let Ok(meta) = std::fs::metadata(&last_update_path) {
            meta.modified()
                .ok()
                .and_then(|t| t.elapsed().ok())
                .map(|d| d.as_secs() > 3600)
                .unwrap_or(true)
        } else {
            true
        };
        if !should_refresh {
            tracing::debug!("[plugin] 注册表 1 小时内已刷新过，跳过");
            return;
        }
        if let Some(state) = app_handle.try_state::<AppState>() {
            match state.plugin_manager.update_registry().await {
                Ok(()) => {
                    tracing::info!("[plugin] 注册表后台刷新成功");
                    let _ = std::fs::write(&last_update_path, b"");
                }
                Err(e) => {
                    tracing::warn!(
                        "[plugin] 注册表后台刷新失败（将在下次手动刷新时重试）: {}",
                        e
                    )
                }
            }
        }
    });
}

/// 第 8 步：检测系统 locale（前端通过 IPC get_system_locale + navigator.language 获取）。
fn setup_detect_locale() {
    let locale = commands::system::get_ui_language().unwrap_or_else(|| "en-US".to_string());
    let locale_flag = if locale.starts_with("zh") || locale.starts_with("cmn") {
        "zh-CN"
    } else {
        "en-US"
    };
    tracing::debug!(
        "[setup] locale: get_ui_language()={}, resolved={}",
        locale,
        locale_flag
    );
}

/// 第 9 步：启动系统主题轮询任务 — 桌面端先行，移动端使用前端 CSS media query。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn setup_spawn_theme_polling(app: &tauri::AppHandle) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        use std::time::Duration;
        let mut last_theme = String::new();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if let Ok(theme) = commands::system::get_system_theme() {
                if theme != last_theme {
                    last_theme = theme.clone();
                    let _ = app_handle.emit("system-theme-changed", theme);
                }
            }
        }
    });
}

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
        commands::object::object_restore,
        commands::object::object_purge,
        commands::object::trash_restore,
        commands::object::trash_permanent_delete,
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
        commands::template::template_save_from_object,
        commands::template::template_check_field_usage,
        // Search commands
        commands::search::search_unified,
        // Export/Import commands
        commands::export_import::export_get_scope_tree,
        commands::export_import::export_estimate_size,
        commands::export_import::export_get_attachments,
        commands::export_import::export_execute,
        commands::export_import::import_parse_package,
        commands::export_import::import_decrypt_preview,
        commands::export_import::import_execute_advanced,
        commands::vault::get_state,
        commands::vault::change_password,
        commands::vault::delete_account,
        commands::vault::vault_list_accounts,
        commands::vault::vault_update_hint,
        // Profile commands
        commands::profile::profile_load,
        commands::vault::get_vault_stats,
        // File System commands
        commands::fs::fs_scan_directory,
        commands::fs::fs_get_file_size,
        commands::fs::fs_is_dir,
        commands::fs::fs_read_file_as_data_url,
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
        commands::attachment::attachment_list,
        commands::attachment::attachment_save,
        commands::attachment::attachment_soft_delete,
        commands::attachment::attachment_batch_soft_delete,
        commands::attachment::attachment_batch_restore,
        commands::attachment::attachment_batch_delete,
        commands::attachment::attachment_restore,
        commands::attachment::attachment_rename,
        commands::attachment::attachment_delete,
        commands::attachment::attachment_count_batch,
        commands::attachment::attachment_copy_to_vault,
        commands::attachment::attachment_list_all,
        commands::attachment::attachment_download,
        commands::attachment::attachment_open,
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

/// setup 初始化编排：将 Tauri Builder 的 setup 闭包体抽为命名函数，
/// 按步骤调用各 setup_* 助手（步骤注释与运行时日志路径完全一致）。
fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // ════════════════════════════════════════════════════════
    // 启动前检查：ERROR/WARN 记录问题，正常路径不输出噪音
    // 设 RUST_LOG=solo_soul=debug 可看到完整步骤级日志
    // ════════════════════════════════════════════════════════

    // 0. 解析日志目录并初始化 tracing
    setup_logging(app.handle())?;

    // 1. 检查数据目录是否可写
    setup_check_data_dir(app.handle())?;

    // 1.5 P209 迁移窗口诊断：统计仍为 legacy XOR 格式的生物识别凭证存量
    // （仅 tel 级日志，供大版本发布后评估关闭 legacy.rs XOR 路径的依据）
    setup_scan_legacy_biometric(app.handle());

    // 2. 检查资源目录与关键子目录
    setup_check_resource_dirs(app);

    // 3. 为当前进程设置 PDFium 动态库路径（OCR 与水印共用）— 桌面端先行
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    commands::ocr::ensure_pdfium_library_path(app.handle());

    // 4. 初始化 AppState（关键步骤，失败时中止启动）+ SAF 冷启动同步
    setup_init_state(app)?;

    // 5. 初始化发现服务状态（桌面端 mDNS / 移动端 NSD 共用同一命令签名）
    app.manage(commands::discovery::SharedDaemon::new());

    // 6. 初始化 RESOURCE_DIR
    setup_init_resource_dir(app);

    // 7. 后台静默刷新插件注册表（不阻塞启动，失败仅记录日志）— 桌面端先行
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    setup_spawn_registry_refresh(app.handle());

    // 8. 检测系统 locale（前端通过 IPC get_system_locale + navigator.language 获取）
    setup_detect_locale();

    // 9. 启动系统主题轮询任务 — 桌面端先行，移动端使用前端 CSS media query
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    setup_spawn_theme_polling(app.handle());

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── 第 0 步：注册 panic hook（在一切初始化之前）──
    // 注意：此时可能还没有正确的日志目录（移动端需进入 setup 后才能解析），
    // panic 信息会先写入 stderr；setup 中设置 LOG_DIR 后则可写入文件。
    setup_panic_hook();

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

    let result = builder
        .setup(setup_app)
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
    #[test]
    fn test_dispatch_cluster_prefixes_consistent() {
        // 各簇命令名与其路由前缀的映射（与 dispatch_ipc / register_*_commands 一一对应）
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
                    "object_restore",
                    "object_purge",
                    "trash_restore",
                    "trash_permanent_delete",
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
                    "template_save_from_object",
                    "template_check_field_usage",
                    "search_unified",
                    "export_get_scope_tree",
                    "export_estimate_size",
                    "export_get_attachments",
                    "export_execute",
                    "import_parse_package",
                    "import_decrypt_preview",
                    "import_execute_advanced",
                    "get_state",
                    "change_password",
                    "delete_account",
                    "vault_list_accounts",
                    "vault_update_hint",
                    "profile_load",
                    "get_vault_stats",
                    "fs_scan_directory",
                    "fs_get_file_size",
                    "fs_is_dir",
                    "fs_read_file_as_data_url",
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
                    "attachment_delete",
                    "attachment_count_batch",
                    "attachment_copy_to_vault",
                    "attachment_list_all",
                    "attachment_download",
                    "attachment_open",
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
        // 192 条命令全覆盖
        assert_eq!(total, 192);
    }
}
