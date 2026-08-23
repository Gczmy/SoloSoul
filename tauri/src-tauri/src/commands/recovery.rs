//! 跨设备账户恢复命令。
//!
//! 主机端生成一个 6 位 PIN，把当前账户打包成 `.solosoul` 后通过 Noise_XX
//! 加密通道传送给新设备；新设备创建同名账户后导入数据，从而保证
//! `account_id` 一致，后续可直接使用 Device Sync。

use crate::commands::export_import::export::export_execute;
use crate::commands::export_import::import::import_execute_internal;
use crate::commands::export_import::{default_locale, ExportRequest, ExportScope, ImportStrategy};
use crate::state::AppState;
use solosoul_sync::recovery::{generate_recovery_password, recover_from_host, RecoveryHost};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryHostInfo {
    pub display_addr: String,
    pub bind_addr: String,
    pub pin: String,
    pub nonce: String,
    pub fingerprint: String,
    /// 供前端生成 QR 码的 JSON 字符串。
    pub qr_payload: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultSummary {
    pub object_count: usize,
    pub attachment_count: usize,
    /// 恢复包的账户 ID（与旧设备一致，用于在卡片上展示）。
    pub account_id: String,
    /// 恢复包的账户名。
    pub account_name: String,
}

fn nanoid() -> String {
    uuid::Uuid::new_v4().to_string().replace("-", "")
}

/// 从 VaultService 读取当前解锁账户的名称。
fn get_current_account_name(state: &AppState, account_id: &str) -> Result<String, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let accounts = svc.list_accounts();
    accounts
        .into_iter()
        .find(|a| a.id == account_id)
        .map(|a| a.name)
        .ok_or_else(|| "Current account not found in account list".to_string())
}

/// 启动恢复主机。返回显示地址、PIN 和 QR  payload。
#[tauri::command]
pub async fn recovery_host_start(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<RecoveryHostInfo, String> {
    let account_id = crate::commands::current_account_optional(&state)
        .ok_or("No account is currently unlocked")?;
    let account_name = get_current_account_name(&state, &account_id)?;

    // P015: IPC/生成边界立即 Zeroizing 包装——恢复密码在主机会话期（最长 5 分钟）
    // 驻留内存，普通 String 可被内存转储/交换分区还原，进而解密已导出的备份包。
    let recovery_password = zeroize::Zeroizing::new(generate_recovery_password());

    // 临时导出文件路径
    let tmp_dir = std::env::temp_dir();
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let export_path = tmp_dir.join(format!(
        "solosoul_recovery_{}_{}.solosoul",
        account_id,
        nanoid()
    ));

    // 收集全部附件 ID，保证恢复包包含附件
    let all_attachment_ids = collect_all_attachment_ids(&state, &account_id)?;

    let export_req = ExportRequest {
        scope: ExportScope {
            selected_page_ids: Vec::new(),
            selected_object_ids: Vec::new(),
            selected_tags: Vec::new(),
            include_attachments: true,
            selected_attachment_ids: all_attachment_ids,
            include_preferences: true,
            include_behavioral: false,
            include_all: true,
        },
        password: (*recovery_password).clone(),
        password_hint: Some("Recovery transfer".to_string()),
        save_path: export_path.to_string_lossy().to_string(),
    };

    // 复用现有导出命令生成加密恢复包（State 不可 Clone，从 AppHandle 重新获取）
    let export_state: State<'_, AppState> = app.state::<AppState>();
    export_execute(app.clone(), export_state, account_id.clone(), export_req).await?;

    // 取消并清理之前可能残留的主机（在锁外 join，避免阻塞）
    cancel_and_cleanup_old_host(&state)?;

    // 启动新的恢复主机（监听所有接口）
    let host = RecoveryHost::start(
        "0.0.0.0:0",
        export_path.clone(),
        recovery_password,
        account_id.clone(),
        account_name.clone(),
    )?;
    let info = host.connection_info();
    let host_cancel = Arc::new(AtomicBool::new(false));
    let host_cancel_for_thread = host_cancel.clone();

    let export_path_for_thread = export_path.clone();
    let thread = std::thread::spawn(move || {
        if let Err(e) = host.run(host_cancel_for_thread) {
            tracing::warn!("Recovery host session ended: {}", e);
        }
        // 会话结束后清理临时导出文件
        let _ = std::fs::remove_file(&export_path_for_thread);
    });

    // 注册恢复主机的 mDNS 广告，让局域网内的新设备能自动发现本机
    #[cfg(desktop)]
    // 注册恢复主机的 mDNS 广告，让局域网内的新设备能自动发现本机
    #[cfg(desktop)]
    let mdns_instance_name =
        advertise_recovery_mdns(&app, &info.fingerprint, &info.display_addr).await?;
    #[cfg(not(desktop))]
    let mdns_instance_name: Option<String> = None;
    #[cfg(not(desktop))]
    let mdns_instance_name: Option<String> = None;

    {
        let mut rec = state.recovery_state.lock().map_err(|e| e.to_string())?;
        rec.host_cancel = host_cancel;
        rec.host_thread = Some(thread);
        rec.export_path = Some(export_path);
        rec.mdns_instance_name = mdns_instance_name;
    }

    let qr_payload = serde_json::json!({
        "t": "rec",
        "a": info.display_addr,
        "p": info.pin,
        "n": info.nonce.clone(),
        "f": info.fingerprint.clone(),
        "u": account_id.clone(),
        "m": account_name.clone()
    })
    .to_string();

    Ok(RecoveryHostInfo {
        display_addr: info.display_addr,
        bind_addr: info.bind_addr,
        pin: info.pin,
        nonce: info.nonce,
        fingerprint: info.fingerprint,
        qr_payload,
    })
}
/// 收集全部附件 ID（未删除项），保证恢复包包含附件。
fn collect_all_attachment_ids(
    state: &State<'_, AppState>,
    account_id: &str,
) -> Result<Vec<String>, String> {
    let vault = crate::commands::vault_handle(state)?;
    let objects = vault
        .list_objects(account_id, None, None, None, false, false)
        .map_err(|e| e.to_string())?;
    let mut ids = Vec::new();
    for obj in objects {
        let atts = crate::commands::export_import::load_attachments(&obj.properties);
        for att in atts {
            if att.deleted_at.is_none() {
                ids.push(att.id);
            }
        }
    }
    Ok(ids)
}

/// 取消并清理之前可能残留的恢复主机（在锁外 join，避免阻塞）。
fn cancel_and_cleanup_old_host(state: &State<'_, AppState>) -> Result<(), String> {
    let (old_thread, old_path) = {
        let mut rec = state.recovery_state.lock().map_err(|e| e.to_string())?;
        rec.host_cancel.store(true, Ordering::SeqCst);
        (rec.host_thread.take(), rec.export_path.take())
    };
    if let Some(thread) = old_thread {
        let _ = thread.join();
    }
    if let Some(path) = old_path {
        let _ = std::fs::remove_file(&path);
    }
    Ok(())
}

/// 注册恢复主机的 mDNS 广告，让局域网内的新设备能自动发现本机。
#[cfg(desktop)]
async fn advertise_recovery_mdns(
    app: &tauri::AppHandle,
    fingerprint: &str,
    display_addr: &str,
) -> Result<Option<String>, String> {
    let daemon_state = app.state::<crate::commands::discovery::SharedDaemon>();
    let daemon_arc = daemon_state.get().await?;
    let guard = daemon_arc.lock().await;
    if let Some(daemon) = guard.as_ref() {
        let instance_name = format!("recovery-{}", &fingerprint[..fingerprint.len().min(8)]);
        if let Err(e) = crate::commands::discovery::recovery_advertise(
            daemon,
            &instance_name,
            display_addr
                .split(':')
                .next_back()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(0),
            fingerprint,
            display_addr,
        ) {
            tracing::warn!("Recovery mDNS advertise failed (non-fatal): {}", e);
            Ok(None)
        } else {
            Ok(Some(instance_name))
        }
    } else {
        Ok(None)
    }
}

#[cfg(not(desktop))]
async fn advertise_recovery_mdns(
    _app: &tauri::AppHandle,
    _fingerprint: &str,
    _display_addr: &str,
) -> Result<Option<String>, String> {
    Ok(None)
}

/// 取消当前正在运行的恢复主机。
#[tauri::command]
pub async fn recovery_host_cancel(state: State<'_, AppState>) -> Result<(), String> {
    let (thread, path, mdns_name) = {
        let mut rec = state.recovery_state.lock().map_err(|e| e.to_string())?;
        rec.host_cancel.store(true, Ordering::SeqCst);
        (
            rec.host_thread.take(),
            rec.export_path.take(),
            rec.mdns_instance_name.take(),
        )
    };

    // 取消 mDNS 广告
    if let Some(instance_name) = mdns_name {
        #[cfg(desktop)]
        {
            use tauri::Manager;
            if let Some(daemon_state) = state
                .handle
                .try_state::<crate::commands::discovery::SharedDaemon>()
            {
                if let Ok(daemon_arc) = daemon_state.get().await {
                    let guard = daemon_arc.lock().await;
                    if let Some(daemon) = guard.as_ref() {
                        let _ = crate::commands::discovery::recovery_stop_advertise(
                            daemon,
                            &instance_name,
                        );
                    }
                }
            }
        }
    }

    if let Some(thread) = thread {
        let _ = thread.join();
    }
    if let Some(path) = path {
        let _ = std::fs::remove_file(&path);
    }
    Ok(())
}

/// 从恢复主机下载加密恢复包，创建与主机相同 account_id 的账户，并导入数据。
///
/// 当本机已存在相同 `account_id` 的账户时：
/// - `overwrite == true`：先删除本机该账户，再用旧设备数据覆盖（覆盖恢复，可用于重设本端密码）。
/// - `overwrite` 为 false/None：由 `create_account_with_id` 返回 "Account ID already exists"，前端据此提示冲突。
///
/// 覆盖仅发生在恢复包下载成功之后，网络/握手失败不会损毁本地数据。
///
/// 执行期间向 `recovery-progress` 事件发射分阶段进度：
/// `download`(0-40) → `overwrite`(45，仅覆盖模式) → `create`(50) → `import`(50-95) → `done`(100)。
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn recovery_restore_from_host(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    master_password: String,
    host_addr: String,
    pin: String,
    fingerprint: Option<String>,
    nonce: Option<String>,
    password_hint: Option<String>,
    overwrite: Option<bool>,
) -> Result<ImportResultSummary, String> {
    if master_password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }
    if host_addr.trim().is_empty() {
        return Err("Host address is required".to_string());
    }
    if pin.len() != 6 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err("PIN must be a 6-digit code".to_string());
    }

    // 进度事件发射器（phase + 0-100 全局百分比）。
    let emit_progress = |phase: &'static str, percent: u8| {
        let _ = app.emit(
            "recovery-progress",
            serde_json::json!({ "phase": phase, "percent": percent }),
        );
    };

    let dest_dir = std::env::temp_dir().join("solosoul_recovery_downloads");
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    // 阶段 1：从主机下载恢复包（0-40，下载进度按字节数换算）。
    let (account_id, account_name, downloaded_path, recovery_password) = download_recovery_package(
        &app,
        &host_addr,
        &pin,
        &dest_dir,
        fingerprint.as_deref(),
        nonce.as_deref(),
    )
    .await?;
    let file_path = downloaded_path.to_string_lossy().to_string();

    // 阶段 2：使用主机的 account_id 和 account_name 创建本地账户。
    // 恢复场景允许同名账户共存（身份是 account_id）；
    // 覆盖模式下若本机已存在相同 account_id，先删除再创建（不可逆，前端已二次确认）。
    {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        create_recovery_account(
            &svc,
            &account_id,
            &account_name,
            &master_password,
            password_hint.as_deref(),
            overwrite,
            &emit_progress,
        )?;
    }

    // 阶段 3：导入恢复包（50-95，导入进度按对象/附件条目数换算）。
    // P015: 恢复密码在导入链路同样 Zeroizing 管理。
    let app_for_import = app.clone();
    let import_result = import_execute_internal(
        state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?,
        account_id.clone(),
        file_path.clone(),
        zeroize::Zeroizing::new(recovery_password),
        ImportStrategy::SkipExisting,
        None,
        None,
        HashMap::new(),
        &default_locale(),
        Some(Arc::new(move |pct: u8| {
            let _ = app_for_import.emit(
                "recovery-progress",
                serde_json::json!({
                    "phase": "import",
                    "percent": (50 + u16::from(pct) * 45 / 100) as u8,
                }),
            );
        }) as Arc<dyn Fn(u8) + Send + Sync>),
    );

    // N-101：恢复导入成功后触发 SAF 自动同步（原 import 内部行为，重构后由调用方负责）
    if import_result.is_ok() {
        state.auto_sync.trigger_debounce();
    }

    // 清理下载的临时文件
    let _ = std::fs::remove_file(&file_path);

    let import_result = match import_result {
        Ok(r) => r,
        Err(e) => {
            // 导入失败时回滚已创建的账户，避免留下空账户
            if let Err(del_err) = state
                .vault_service
                .read()
                .map_err(|_| "Vault service lock poisoned".to_string())
                .and_then(|svc| svc.delete_account(&account_id))
            {
                tracing::warn!(
                    "Failed to roll back partially created account during recovery: {}",
                    del_err
                );
            }
            return Err(e);
        }
    };

    emit_progress("done", 100);
    Ok(ImportResultSummary {
        object_count: import_result.object_count,
        attachment_count: import_result.attachment_count,
        account_id,
        account_name,
    })
}
/// 阶段 1：从主机下载恢复包（0-40 进度按字节数换算）。
async fn download_recovery_package(
    app: &tauri::AppHandle,
    host_addr: &str,
    pin: &str,
    dest_dir: &std::path::Path,
    fingerprint: Option<&str>,
    nonce: Option<&str>,
) -> Result<(String, String, std::path::PathBuf, String), String> {
    let app_for_download = app.clone();
    let host_addr = host_addr.to_string();
    let pin = pin.to_string();
    let dest_dir = dest_dir.to_path_buf();
    let fingerprint = fingerprint.map(|s| s.to_string());
    let nonce = nonce.map(|s| s.to_string());
    let result = tokio::task::spawn_blocking(move || {
        recover_from_host(
            &host_addr,
            &pin,
            &dest_dir,
            fingerprint.as_deref(),
            nonce.as_deref(),
            Some(Box::new(move |pct: u8| {
                let _ = app_for_download.emit(
                    "recovery-progress",
                    serde_json::json!({
                        "phase": "download",
                        "percent": (u16::from(pct) * 40 / 100) as u8,
                    }),
                );
            })),
        )
    })
    .await
    .map_err(|e| format!("Recovery task failed: {}", e))?;
    let result = result?;
    Ok((
        result.account_id,
        result.account_name,
        result.downloaded_path,
        result.recovery_password,
    ))
}

/// 阶段 2：使用主机的 account_id/account_name 创建本地账户。
/// 覆盖模式下本机已存在相同 account_id 时先删除再创建（不可逆，前端已二次确认）。
fn create_recovery_account(
    svc: &solosoul_core::vault_service::VaultService,
    account_id: &str,
    account_name: &str,
    master_password: &str,
    password_hint: Option<&str>,
    overwrite: Option<bool>,
    emit_progress: &dyn Fn(&'static str, u8),
) -> Result<(), String> {
    if overwrite.unwrap_or(false) && svc.has_account(account_id) {
        emit_progress("overwrite", 45);
        svc.delete_account(account_id)?;
    }
    emit_progress("create", 50);
    svc.create_account_with_id(account_id, account_name, master_password, password_hint)?;
    Ok(())
}
