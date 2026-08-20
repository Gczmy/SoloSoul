//! 应用启动 setup 初始化步骤（P025-② 从 lib.rs 抽出）。
//! 每步一个独立函数（setup_*），由 setup_app 按序编排。

use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::Emitter;
use tauri::Manager;

use crate::commands;
use crate::state::AppState;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub(crate) fn setup_panic_hook() {
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
    //
    // P015: 发布构建固定 `info,ort=warn` 上限——debug 级会落盘更多标识信息（如
    // biometric 的 account_id），本地威胁模型下风险低但无收益；仅 debug 构建响应
    // RUST_LOG，发布构建无视用户环境变量以防日志级别被静默提升。
    let release_override = !cfg!(debug_assertions);
    let env_filter = if release_override {
        tracing_subscriber::EnvFilter::new("info,ort=warn")
    } else {
        std::env::var("RUST_LOG")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(|s| tracing_subscriber::EnvFilter::new(&s))
            .unwrap_or_else(|| tracing_subscriber::EnvFilter::new("info,ort=warn"))
    };

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
fn setup_cleanup_import_temps(app: &tauri::AppHandle) {
    let Ok(data_dir) = resolve_app_data_dir(app) else {
        return;
    };
    let _ = commands::export_import::import::cleanup_orphan_import_temps(&data_dir);
}
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
pub(crate) fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // ════════════════════════════════════════════════════════
    // 启动前检查：ERROR/WARN 记录问题，正常路径不输出噪音
    // 设 RUST_LOG=solo_soul=debug 可看到完整步骤级日志
    // ════════════════════════════════════════════════════════

    // 0. 解析日志目录并初始化 tracing
    setup_logging(app.handle())?;

    // 1. 检查数据目录是否可写
    setup_check_data_dir(app.handle())?;

    // 1.5 清扫数据目录内崩溃残留的导入明文孤儿临时目录（P013）
    setup_cleanup_import_temps(app.handle());

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
