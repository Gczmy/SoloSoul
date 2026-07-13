use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::Emitter;
use tauri::Manager;

pub mod commands;
pub mod local_embed;
pub mod plugin;
pub mod services;
pub mod state;
pub mod status_bar_plugin;

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
fn resolve_app_data_dir(#[allow(unused_variables)] app: &tauri::AppHandle) -> Result<PathBuf, String> {
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
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(status_bar_plugin::init());

    // 桌面端专属插件
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder
            .plugin(tauri_plugin_window_state::Builder::new().build())
            .plugin(tauri_plugin_updater::Builder::new().build());
    }

    let result = builder
        .setup(|app| {
            // ════════════════════════════════════════════════════════
            // 启动前检查：ERROR/WARN 记录问题，正常路径不输出噪音
            // 设 RUST_LOG=solo_soul=debug 可看到完整步骤级日志
            // ════════════════════════════════════════════════════════

            // 0. 解析日志目录并初始化 tracing
            let log_dir = match resolve_log_dir(app.handle()) {
                Ok(dir) => dir,
                Err(e) => {
                    eprintln!("[fatal] 无法解析日志目录: {e}");
                    return Err(format!("无法解析日志目录: {e}").into());
                }
            };
            let _ = std::fs::create_dir_all(&log_dir);
            LOG_DIR.set(log_dir.clone()).ok();
            init_tracing(&log_dir);

            tracing::info!("[init] SoloSoul v{} 启动", env!("CARGO_PKG_VERSION"));
            tracing::info!("[init] 日志目录: {}", log_dir.display());
            tracing::info!("[init] 目标平台: {}", std::env::consts::OS);

            // 1. 检查数据目录是否可写
            {
                let data_dir = match resolve_app_data_dir(app.handle()) {
                    Ok(dir) => dir,
                    Err(e) => {
                        tracing::error!("[setup] ❌ 无法解析数据目录: {}", e);
                        return Err(format!("无法解析数据目录: {e}").into());
                    }
                };
                if let Err(e) = std::fs::create_dir_all(&data_dir) {
                    tracing::error!(
                        "[setup] ❌ 数据目录不可写: {} 错误: {}",
                        data_dir.display(),
                        e
                    );
                }
            }

            // 2. 检查资源目录与关键子目录
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

            // 3. 为当前进程设置 PDFium 动态库路径（OCR 与水印共用）— 桌面端先行
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            commands::ocr::ensure_pdfium_library_path(app.handle());

            // 4. 初始化 AppState（关键步骤，失败时中止启动）
            tracing::debug!("[setup] 正在创建 AppState...");
            let app_state = match AppState::new(app.handle().clone()) {
                Ok(state) => state,
                Err(e) => {
                    tracing::error!("[setup] ❌ AppState 创建失败: {:#}", e);
                    return Err(format!("AppState 创建失败: {:#}", e).into());
                }
            };
            app.manage(app_state);

            // 5. 初始化桌面端发现服务（mDNS）— 移动端暂不提供
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                app.manage(commands::discovery::SharedDaemon::new());
            }

            // 6. 初始化 RESOURCE_DIR
            // Android 上 Tauri 的 resource_dir 返回 asset:// URL，std::fs 无法直接读取。
            // MainActivity 已在 onCreate 中将所需资源复制到 files/resources/，这里优先使用它。
            #[cfg(target_os = "android")]
            let resource_dir: Result<PathBuf, String> = {
                let data_dir = resolve_app_data_dir(app.handle())?;
                Ok(data_dir.join("resources"))
            };
            #[cfg(not(target_os = "android"))]
            let resource_dir = app.path().resource_dir();

            match resource_dir {
                Ok(dir) => {
                    tracing::info!("[setup] RESOURCE_DIR set to: {}", dir.display());
                    let _ = commands::llm::RESOURCE_DIR.set(dir.clone());
                }
                Err(e) => {
                    tracing::error!("[setup] ❌ 无法获取 resource_dir，RESOURCE_DIR 未设置: {}", e);
                }
            }

            // 7. 应用启动时后台静默刷新插件注册表（不阻塞启动，失败仅记录日志）— 桌面端先行
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // 若未配置公钥则跳过，避免每次启动都报错
                    if std::env::var("SOLOSOUL_REGISTRY_PUBKEY").is_err() {
                        tracing::debug!(
                            "[plugin] SOLOSOUL_REGISTRY_PUBKEY 未配置，跳过启动时注册表刷新"
                        );
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

            // 8. 检测系统 locale（前端通过 IPC get_system_locale + navigator.language 获取）
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

            // 9. 启动系统主题轮询任务 — 桌面端先行，移动端使用前端 CSS media query
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                let app_handle = app.handle().clone();
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
            commands::auth::unlock_with_password,
            // Vault commands
            commands::vault::unlock,
            commands::vault::lock,
            // Object commands
            commands::object::object_list,
            commands::object::object_get,
            commands::object::object_create,
            commands::object::object_update,
            commands::object::object_delete,
            commands::object::object_get_template_sync_status,
            commands::object::object_sync_with_template,
            commands::object::object_ignore_template_sync,
            commands::object::object_list_deprecated_fields,
            commands::object::object_backfill_property_labels,
            commands::object::object_backfill_property_fields,
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
            commands::object::snapshot_count_batch,
            commands::object::snapshot_get_data,
            commands::object::snapshot_rollback,
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
            commands::search::search_advanced,
            // Export/Import commands
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
            commands::vault::vault_list_accounts,
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
            commands::vault::get_vault_stats,
            // File System commands
            commands::fs::inspect_backup,
            commands::fs::fs_scan_directory,
            commands::fs::fs_get_file_size,
            commands::fs::fs_is_dir,
            commands::fs::fs_read_file_as_data_url,
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
            commands::llm::llm_find_guides,
            commands::llm::llm_get_stats,
            commands::llm::llm_reset_stats,
            commands::llm::llm_send_message_stream,
            commands::llm::llm_chat,
            commands::llm::llm_persist_stats,
            commands::llm::guide_load_index,
            commands::llm::guide_load_content,
            commands::llm::guide_search,
            commands::llm::guide_load_search_index,
            commands::llm::llm_search_guide_chunks,
            commands::llm::llm_rebuild_guide_embeddings,
            commands::llm::llm_check_embedding_available,
            commands::llm::llm_set_local_embedding,
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
            // OCR commands
            commands::ocr::ocr_scan_image,
            commands::ocr::ocr_scan_mrz,
            commands::ocr::ocr_get_supported_languages,
            commands::ocr::ocr_list_available_tiers,
            commands::ocr::ocr_get_active_tier,
            commands::ocr::ocr_set_active_tier,
            commands::ocr::ocr_get_model_status,
            commands::ocr::ocr_install_bundled_model,
            commands::ocr::ocr_install_bundled_model_with_progress,
            commands::ocr::ocr_download_model,
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
            commands::attachment::attachment_cleanup_orphans,
            commands::attachment::attachment_download,
            commands::attachment::attachment_open,
            // Sync commands
            commands::sync::sync_discover,
            commands::sync::sync_get_status,
            commands::sync::sync_enable,
            commands::sync::sync_with_device,
            commands::sync::sync_trust_peer,
            commands::sync::sync_forget_peer,
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
            // Discovery commands
            commands::discovery::mdns_advertise,
            commands::discovery::mdns_discover,
            // Window chrome commands
            commands::window::set_titlebar_color,
            status_bar_plugin::set_status_bar_style,
            // Embedding model commands
            commands::embed_model::llm_get_embed_models,
            commands::embed_model::llm_download_embed_model,
            commands::embed_model::llm_delete_embed_model,
        ])
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
