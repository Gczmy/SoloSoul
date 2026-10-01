//! 跨设备账户恢复命令。
//!
//! 主机端生成一个 6 位 PIN，把当前账户打包成 `.solosoul` 后通过 Noise_XX
//! 加密通道传送给新设备；新设备创建同名账户后导入数据，从而保证
//! `account_id` 一致，后续可直接使用 Device Sync。

use crate::state::AppState;
use crate::sync::contracts::RecoveryProgress;
pub use crate::sync::contracts::{ImportResultSummary, RecoveryHostInfo};
use solosoul_core::export_import::export::{
    execute_encrypted_export, EncryptedExportRequest, EncryptedExportScope,
};
use solosoul_core::vault_service::{VaultService, VaultSession};
use solosoul_sync::recovery::{generate_recovery_password, recover_from_host, RecoveryHost};
use solosoul_vault::ImportSourceKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

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

    // 恢复包的位置由后端生成，走内部导出核心；用户导出 IPC 仍执行目录白名单校验。
    let export_file = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        export_recovery_package(&svc, &account_id, &recovery_password)?
    };
    let export_path = export_file.to_path_buf();

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

    // 注册恢复主机的 mDNS 广告，让局域网内的新设备能自动发现本机
    #[cfg(desktop)]
    let mdns_instance_name =
        advertise_recovery_mdns(&app, &info.fingerprint, &info.display_addr).await?;
    #[cfg(not(desktop))]
    let mdns_instance_name: Option<String> = None;

    {
        let mut rec = state.recovery_state.lock().map_err(|e| e.to_string())?;
        let thread = std::thread::spawn(move || {
            if let Err(e) = host.run(host_cancel_for_thread) {
                tracing::warn!("Recovery host session ended: {}", e);
            }
            // TempPath 由会话持有；正常结束、取消及线程退栈时均会删除临时包。
            drop(export_file);
        });
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

/// 创建仅供恢复传输使用的加密临时包，不接收前端传入的输出路径。
/// TempPath 在导出/启动失败时自动删除文件，成功后交给恢复会话持有。
fn export_recovery_package(
    svc: &VaultService,
    account_id: &str,
    recovery_password: &str,
) -> Result<tempfile::TempPath, String> {
    let _activity = solosoul_core::import_activity::begin_owned_root_activity(svc.root_owner())?;
    svc.get_vault_store().ok_or("Vault not unlocked")?;
    // NamedTempFile 原子创建随机文件（Unix 0600）。先关闭文件句柄，兼容 Windows 重开写入。
    let export_file = tempfile::Builder::new()
        .prefix("solosoul-recovery-")
        .suffix(".solosoul")
        .tempfile()
        .map_err(|e| format!("Create recovery package: {e}"))?
        .into_temp_path();
    let session = svc.capture_session(account_id)?;
    let scope = EncryptedExportScope::full_snapshot();
    let hint = Some("Recovery transfer".to_string());
    let req = EncryptedExportRequest {
        scope: &scope,
        password: recovery_password,
        password_hint: &hint,
        app_version: env!("CARGO_PKG_VERSION"),
    };
    execute_encrypted_export(svc, &session, &req, &export_file)
        .map_err(crate::services::encrypted_export::map_export_failure)?;
    Ok(export_file)
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

/// 创建同身份账户并导入恢复包；旧覆盖参数保留。
/// 业务提交前先准备完整账户密文 handoff；导入失败绝不自动删除账户。
/// 未接纳任务时需重新开启恢复主机，以新的认证包向已解锁原账户 Fresh 重试。
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
    let master_password = zeroize::Zeroizing::new(master_password);
    if master_password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }
    validate_recovery_connection(&host_addr, &pin)?;
    let download = download_recovery_package(
        &app,
        &host_addr,
        &pin,
        fingerprint.as_deref(),
        nonce.as_deref(),
    )
    .await?;
    let operation_id = uuid::Uuid::new_v4().to_string();
    let account_id = download.account_id.clone();
    let account_name = download.account_name.clone();
    let downloaded_path = download.downloaded_path.clone();
    let service = state.vault_service.clone();
    let (owner, maintenance) = {
        let svc = service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let owner = svc.root_owner();
        // 覆盖参数可能触发删除；即使当前尚无此账户，也先关闭检查后的创建竞态。
        let maintenance = if overwrite.unwrap_or(false) {
            let maintenance = solosoul_core::import_activity::begin_owned_root_maintenance(
                std::sync::Arc::clone(&owner),
            )?;
            solosoul_core::import_activity::ensure_imports_idle(
                owner.root(),
                Some(&download.account_id),
            )?;
            Some(maintenance)
        } else {
            None
        };
        (owner, maintenance)
    };
    if maintenance.is_some() {
        state.sync_service.disable_and_wait().await?;
    }
    let app_for_worker = app.clone();
    let operation_for_worker = operation_id.clone();
    let worker = tokio::task::spawn_blocking(move || {
        let svc = service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        if !std::sync::Arc::ptr_eq(&svc.root_owner(), &owner) {
            return Err("VAULT_ROOT_MISMATCH".to_string());
        }
        create_recovery_account_with_maintenance(
            &svc,
            &download.account_id,
            &download.account_name,
            &master_password,
            password_hint.as_deref(),
            overwrite,
            &|phase, percent| emit_recovery_progress(&app_for_worker, phase, percent),
            maintenance,
        )?;
        // 创建已建立解锁会话；后续准备/提交始终绑定此令牌，不重新捕获当前账户。
        let session = svc.capture_session(&download.account_id)?;
        let outcome = import_downloaded_recovery_for_session(
            &svc,
            &session,
            &download.account_id,
            download.downloaded_path,
            download.recovery_password,
            &operation_for_worker,
            Some(recovery_import_progress(
                app_for_worker,
                operation_for_worker.clone(),
            )),
        )?;
        Ok::<_, String>((session, outcome))
    })
    .await
    .map_err(|_| "Recovery task failed".to_string())?;
    // worker panic/Err 提交状态未知；也不能回滚删除已创建的账户。
    let (session, outcome) = worker?;
    if let Ok(svc) = state.vault_service.read() {
        notify_recovery_complete(&svc, &session, &outcome, || {
            state.auto_sync.trigger_debounce()
        });
    }
    finish_recovery_download(&app, &outcome, &downloaded_path);
    Ok(ImportResultSummary {
        outcome,
        account_id,
        account_name,
    })
}

/// 从新的已认证恢复主机会话 Fresh 导入到原账户；无创建、覆盖或重设密码分支。
/// 下载前固定原 Session；传输身份不同、期间锁定/换账户/换目录均拒绝业务写入。
#[tauri::command]
pub async fn recovery_restore_existing_from_host(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    host_addr: String,
    pin: String,
    fingerprint: Option<String>,
    nonce: Option<String>,
) -> Result<ImportResultSummary, String> {
    validate_recovery_connection(&host_addr, &pin)?;
    let session = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.capture_session(&account_id)?
    };
    let download = download_recovery_package(
        &app,
        &host_addr,
        &pin,
        fingerprint.as_deref(),
        nonce.as_deref(),
    )
    .await?;
    let account_name = download.account_name.clone();
    let downloaded_path = download.downloaded_path.clone();
    let original_session = session.clone();
    let service = state.vault_service.clone();
    let operation_id = uuid::Uuid::new_v4().to_string();
    let app_for_worker = app.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        let svc = service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        import_downloaded_recovery_for_session(
            &svc,
            &original_session,
            &download.account_id,
            download.downloaded_path,
            download.recovery_password,
            &operation_id,
            Some(recovery_import_progress(
                app_for_worker,
                operation_id.clone(),
            )),
        )
    })
    .await
    .map_err(|_| "Recovery task failed".to_string())??;
    if let Ok(svc) = state.vault_service.read() {
        notify_recovery_complete(&svc, &session, &outcome, || {
            state.auto_sync.trigger_debounce()
        });
    }
    finish_recovery_download(&app, &outcome, &downloaded_path);
    Ok(ImportResultSummary {
        outcome,
        account_id,
        account_name,
    })
}

fn validate_recovery_connection(host_addr: &str, pin: &str) -> Result<(), String> {
    if host_addr.trim().is_empty() {
        return Err("Host address is required".to_string());
    }
    if pin.len() != 6 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err("PIN must be a 6-digit code".to_string());
    }
    Ok(())
}

fn emit_recovery_progress(app: &tauri::AppHandle, phase: &'static str, percent: u8) {
    let _ = app.emit(
        "recovery-progress",
        RecoveryProgress {
            phase: phase.into(),
            percent,
            operation_id: None,
        },
    );
}

fn recovery_import_progress(
    app: tauri::AppHandle,
    operation_id: String,
) -> Arc<dyn Fn(u8) + Send + Sync> {
    Arc::new(move |pct| {
        let _ = app.emit(
            "recovery-progress",
            RecoveryProgress {
                phase: "import".into(),
                percent: (50 + u16::from(pct) * 45 / 100) as u8,
                operation_id: Some(operation_id.clone()),
            },
        );
    })
}

/// 两个 IPC 与回归共用真实导入入口；不允许通过重新 capture_session 借用新会话。
#[allow(clippy::too_many_arguments)]
fn import_downloaded_recovery_for_session(
    svc: &VaultService,
    session: &VaultSession,
    downloaded_account: &str,
    downloaded_path: std::path::PathBuf,
    recovery_password: zeroize::Zeroizing<String>,
    operation_id: &str,
    progress: Option<Arc<dyn Fn(u8) + Send + Sync>>,
) -> Result<crate::commands::export_import::ImportResult, String> {
    if downloaded_account != session.account_id() {
        return Err("RECOVERY_ACCOUNT_MISMATCH".to_string());
    }
    svc.with_session(session, |_| Ok(()))?;
    let request = solosoul_core::export_import::import::EncryptedImportRequest {
        source_path: downloaded_path.to_string_lossy().into_owned(),
        password: recovery_password,
        options: solosoul_core::export_import::import::ImportOptions {
            locale: "en-US".into(),
            ..Default::default()
        },
        operation: Some((operation_id.into(), ImportSourceKind::Recovery)),
    };
    solosoul_core::export_import::import::execute_encrypted_import(svc, session, request, progress)
        .map(Into::into)
        .map_err(crate::services::encrypted_import::map_import_failure)
}

fn notify_recovery_complete(
    svc: &VaultService,
    session: &VaultSession,
    outcome: &crate::commands::export_import::ImportResult,
    notify: impl FnOnce(),
) {
    if outcome.is_complete() {
        let _ = svc.with_session(session, |_| {
            notify();
            Ok(())
        });
    }
}

fn finish_recovery_download(
    app: &tauri::AppHandle,
    outcome: &crate::commands::export_import::ImportResult,
    path: &std::path::Path,
) {
    if outcome.is_complete() {
        let _ = std::fs::remove_file(path);
        emit_recovery_progress(app, "done", 100);
    } else {
        emit_recovery_progress(app, "incomplete", 95);
    }
}

struct RecoveryDownload {
    account_id: String,
    account_name: String,
    downloaded_path: std::path::PathBuf,
    recovery_password: zeroize::Zeroizing<String>,
}

async fn download_recovery_package(
    app: &tauri::AppHandle,
    host_addr: &str,
    pin: &str,
    fingerprint: Option<&str>,
    nonce: Option<&str>,
) -> Result<RecoveryDownload, String> {
    let dest_dir = std::env::temp_dir().join("solosoul_recovery_downloads");
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let app_for_download = app.clone();
    let host_addr = host_addr.to_owned();
    let pin = pin.to_owned();
    let fingerprint = fingerprint.map(str::to_owned);
    let nonce = nonce.map(str::to_owned);
    tokio::task::spawn_blocking(move || {
        let result = recover_from_host(
            &host_addr,
            &pin,
            &dest_dir,
            fingerprint.as_deref(),
            nonce.as_deref(),
            Some(Box::new(move |pct| {
                emit_recovery_progress(
                    &app_for_download,
                    "download",
                    (u16::from(pct) * 40 / 100) as u8,
                )
            })),
        )?;
        // 传输层返回后立刻包装，随机内部包口令不进入 DTO、日志或持久计划。
        Ok::<_, String>(RecoveryDownload {
            account_id: result.account_id,
            account_name: result.account_name,
            downloaded_path: result.downloaded_path,
            recovery_password: zeroize::Zeroizing::new(result.recovery_password),
        })
    })
    .await
    .map_err(|_| "Recovery task failed".to_string())?
}

/// 阶段 2：使用主机的 account_id/account_name 创建本地账户。
/// 覆盖模式下本机已存在相同 account_id 时先删除再创建（不可逆，前端已二次确认）。
#[cfg(test)]
fn create_recovery_account(
    svc: &solosoul_core::vault_service::VaultService,
    account_id: &str,
    account_name: &str,
    master_password: &str,
    password_hint: Option<&str>,
    overwrite: Option<bool>,
    emit_progress: &dyn Fn(&'static str, u8),
) -> Result<(), String> {
    create_recovery_account_with_maintenance(
        svc,
        account_id,
        account_name,
        master_password,
        password_hint,
        overwrite,
        emit_progress,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn create_recovery_account_with_maintenance(
    svc: &solosoul_core::vault_service::VaultService,
    account_id: &str,
    account_name: &str,
    master_password: &str,
    password_hint: Option<&str>,
    overwrite: Option<bool>,
    emit_progress: &dyn Fn(&'static str, u8),
    maintenance: Option<solosoul_core::import_activity::RootMaintenanceGuard>,
) -> Result<(), String> {
    if overwrite.unwrap_or(false) && svc.has_account(account_id) {
        emit_progress("overwrite", 45);
        if let Some(maintenance) = maintenance.as_ref() {
            svc.delete_account_with_maintenance(account_id, maintenance)?;
        } else {
            svc.delete_account(account_id)?;
        }
    }
    // Core 创建入口自行登记普通 Activity；不能与删除阶段的维护许可嵌套。
    drop(maintenance);
    emit_progress("create", 50);
    svc.create_account_with_id(account_id, account_name, master_password, password_hint)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::export_import::{
        decrypt_zip_entry_streaming, derive_export_key_cfg, read_manifest,
    };

    const MASTER_PASSWORD: &str = "recovery-test-master-password";

    fn setup_vault() -> (tempfile::TempDir, VaultService, String) {
        let dir = tempfile::tempdir().unwrap();
        let svc = VaultService::with_base_path(dir.path().to_path_buf());
        let account = svc
            .create_account("Recovery test", MASTER_PASSWORD, None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        svc.unlock(&account_id, MASTER_PASSWORD).unwrap();
        (dir, svc, account_id)
    }

    #[test]
    fn recovery_package_exports_to_temp_and_cleans_up() {
        let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
        let (_dir, svc, account_id) = setup_vault();
        let now = chrono::Utc::now().to_rfc3339();
        svc.get_vault_store()
            .unwrap()
            .save_object(&solosoul_vault::ObjectRecord {
                id: "recovery-test-object".to_string(),
                account_id: account_id.clone(),
                type_id: "note".to_string(),
                section_type: "identity".to_string(),
                name: "Recovery test note".to_string(),
                properties: serde_json::json!({ "note": "private-recovery-value" }),
                sensitivity_level: "internal".to_string(),
                created_at: now.clone(),
                updated_at: now,
                version: 1,
                ..Default::default()
            })
            .unwrap();

        let password = zeroize::Zeroizing::new(generate_recovery_password());
        let package = export_recovery_package(&svc, &account_id, &password).unwrap();
        let path = package.to_path_buf();
        assert!(path.starts_with(std::env::temp_dir()));
        assert!(path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }

        // 实际解密本次生成的包，确认共用核心仍完整导出对象内容。
        let manifest = read_manifest(path.to_str().unwrap()).unwrap();
        let salt = hex::decode(&manifest.salt_hex).unwrap();
        let key = derive_export_key_cfg(&password, &salt, &manifest.kdf_config()).unwrap();
        let mut payload = zeroize::Zeroizing::new(Vec::new());
        decrypt_zip_entry_streaming(path.to_str().unwrap(), "payload.enc", &key, &mut *payload)
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert!(payload["objects"].as_array().unwrap().iter().any(|obj| {
            obj["id"] == "recovery-test-object"
                && obj["properties"]["note"] == "private-recovery-value"
        }));
        drop(package);
        assert!(!path.exists());
    }

    #[test]
    fn recovery_package_requires_unlocked_vault_and_distinct_password() {
        let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
        let (_dir, svc, account_id) = setup_vault();
        let err = export_recovery_package(&svc, &account_id, MASTER_PASSWORD).unwrap_err();
        assert!(err.contains("SAME_AS_MASTER_PASSWORD"));
        svc.lock();
        let err = export_recovery_package(&svc, &account_id, "recovery-password").unwrap_err();
        assert_eq!(err, "Vault not unlocked");
    }
}

#[cfg(test)]
#[path = "recovery/rf022_tests.rs"]
mod rf022_tests;

#[cfg(test)]
#[path = "recovery/rf015_tests.rs"]
mod rf015_tests;
