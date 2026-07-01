use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::Emitter;
use tauri::Manager;

pub mod commands;
pub mod db;
pub mod ipc;
pub mod local_embed;
pub mod plugin;
pub mod services;
pub mod state;

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

        // Windows：弹出友好的错误消息框
        #[cfg(target_os = "windows")]
        {
            let box_text = format!(
                "SoloSoul 启动时发生致命错误。\n\n\
                 错误: {}\n\
                 位置: {}\n\n\
                 请将日志文件 %APPDATA%\\com.solosoul.app\\logs\\app.log\n\
                 发送给开发团队以协助排查。",
                payload, location
            );
            // SAFETY: show_message_box 封装了 Windows MessageBoxW API，
            // 使用从 panic 信息生成的字符串；在 panic hook 中调用是安全的最佳实践。
            // 该函数接收的字符串在此处分配且在调用期间保持有效。
            unsafe {
                show_message_box("SoloSoul 错误", &box_text);
            }
        }

        // 调用之前的 hook（默认行为，会打印 backtrace 到 stderr）
        previous_hook(panic_info);
    }));
}

/// Windows MessageBox 辅助函数（仅 Windows 编译）
#[cfg(target_os = "windows")]
unsafe fn show_message_box(title: &str, text: &str) {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let _ = MessageBoxW(
        None,
        PCWSTR::from_raw(text_wide.as_ptr()),
        PCWSTR::from_raw(title_wide.as_ptr()),
        MB_OK | MB_ICONERROR,
    );
}

/// Windows: 检查 WebView2 运行时是否安装
/// 通过检查 Edge WebView2 Runtime 文件系统路径
#[cfg(target_os = "windows")]
fn check_webview2_installed() -> bool {
    // 检查常见 WebView2 Runtime 安装路径
    let candidate_dirs = [
        "C:\\Program Files (x86)\\Microsoft\\EdgeWebView\\Application",
        "C:\\Program Files\\Microsoft\\EdgeWebView\\Application",
    ];
    for dir in &candidate_dirs {
        let p = std::path::Path::new(dir);
        if p.exists() {
            // 检查目录中有子目录（版本号），确认不是空目录
            if let Ok(entries) = std::fs::read_dir(p) {
                if entries.count() > 0 {
                    tracing::info!("[check] WebView2 安装路径存在: {}", dir);
                    return true;
                }
            }
        }
    }

    // 检查 System32 中的 WebView2Loader.dll
    if let Ok(root) = std::env::var("SystemRoot") {
        let dll_path = std::path::PathBuf::from(root)
            .join("System32")
            .join("WebView2Loader.dll");
        if dll_path.exists() {
            tracing::info!("[check] WebView2Loader.dll 存在: {}", dll_path.display());
            return true;
        }
    }

    false
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── 第 0 步：确定日志目录 ──
    let log_dir = dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("com.solosoul.app")
        .join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    LOG_DIR.set(log_dir.clone()).ok();

    // ── 第 0.5 步：注册 panic hook（在一切初始化之前）──
    setup_panic_hook();

    // ── 第 1 步：初始化 tracing（文件 + stderr）──
    let file_appender = tracing_appender::rolling::never(&log_dir, "app.log");
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

    tracing::info!("[init] SoloSoul v{} 启动", env!("CARGO_PKG_VERSION"));
    tracing::info!("[init] 日志目录: {}", log_dir.display());
    tracing::info!("[init] 目标平台: {}", std::env::consts::OS);

    // ── 第 2 步：构建 Tauri 应用 ──
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // ════════════════════════════════════════════════════════
            // 启动前检查：ERROR/WARN 记录问题，正常路径不输出噪音
            // 设 RUST_LOG=solo_soul=debug 可看到完整步骤级日志
            // ════════════════════════════════════════════════════════

            // 1. 检查数据目录是否可写
            {
                let data_dir = dirs::data_dir()
                    .unwrap_or_else(std::env::temp_dir)
                    .join("com.solosoul.app");
                if let Err(e) = std::fs::create_dir_all(&data_dir) {
                    tracing::error!(
                        "[setup] ❌ 数据目录不可写: {} 错误: {}",
                        data_dir.display(),
                        e
                    );
                }
            }

            // 2. 检查 WebView2 是否安装（仅 Windows）
            #[cfg(target_os = "windows")]
            {
                if !check_webview2_installed() {
                    tracing::error!(
                        "[setup] ❌ WebView2 运行时未检测到，应用窗口可能无法创建。\n\
                         请访问 https://go.microsoft.com/fwlink/p/?LinkId=2124703 下载安装。"
                    );
                }
            }

            // 3. 检查资源目录与关键子目录
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

            // 4. 为当前进程设置 PDFium 动态库路径（OCR 与水印共用）。
            commands::ocr::ensure_pdfium_library_path(app.handle());

            // 5. 初始化 AppState（关键步骤，失败时中止启动）
            tracing::debug!("[setup] 正在创建 AppState...");
            let app_state = match AppState::new(app.handle().clone()) {
                Ok(state) => state,
                Err(e) => {
                    tracing::error!("[setup] ❌ AppState 创建失败: {:#}", e);
                    return Err(format!("AppState 创建失败: {:#}", e).into());
                }
            };
            app.manage(app_state);
            app.manage(commands::discovery::SharedDaemon::new());

            // 6. 初始化 RESOURCE_DIR
            if let Ok(resource_dir) = app.path().resource_dir() {
                let _ = commands::llm::RESOURCE_DIR.set(resource_dir);
            } else {
                tracing::error!("[setup] ❌ 无法获取 resource_dir，RESOURCE_DIR 未设置");
            }

            // 7. 应用启动时后台静默刷新插件注册表（不阻塞启动，失败仅记录日志）
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
                    let data_dir = dirs::data_dir()
                        .unwrap_or_else(std::env::temp_dir)
                        .join("com.solosoul.app");
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
            //     此前通过 window.eval 注入 __SOLOSOUL_LOCALE__ 的方式已被移除（P005），
            //     改为前端通过 IPC 调用 get_system_locale 获取，无需后端提前注入。
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

            // 9. 恢复窗口大小（通过 Tauri API 设置窗口尺寸）
            //     此前通过 window.eval 注入 localStorage 的方式已被移除（P005），
            //     前端已通过 IPC ui_get_preferences + localStorage 自管理窗口尺寸。
            let managed_state = app.state::<AppState>();
            if let Ok(svc) = managed_state.vault_service.read() {
                let prefs_path = svc.base_path().join("ui_preferences.json");
                if let Ok(content) = std::fs::read_to_string(&prefs_path) {
                    if let Ok(prefs) =
                        serde_json::from_str::<commands::settings::UiPreferences>(&content)
                    {
                        if let Some(ws) = prefs.window_size {
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.set_size(tauri::Size::Physical(
                                    tauri::PhysicalSize::new(ws.width, ws.height),
                                ));
                            }
                        }
                    }
                }
            }

            // 10. 启动系统主题轮询任务，当检测到主题变化时通过 Tauri Event 通知前端
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
            commands::discovery::mdns_advertise,
            commands::vault::lock,
            // Object commands
            commands::object::object_list,
            commands::object::object_get,
            commands::object::object_create,
            commands::object::object_update,
            commands::object::object_delete,
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
            commands::object::snapshot_get,
            commands::object::snapshot_count_batch,
            commands::object::snapshot_get_data,
            commands::object::snapshot_rollback,
            commands::template::template_create,
            commands::template::template_update,
            commands::template::template_delete,
            commands::template::template_restore,
            commands::template::template_get,
            commands::template::template_list,
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
            // Discovery commands
            commands::discovery::mdns_discover,
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
            commands::embed_model::llm_get_embed_models,
            commands::embed_model::llm_download_embed_model,
            commands::embed_model::llm_delete_embed_model,
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
            // Window chrome commands
            commands::window::set_titlebar_color,
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
