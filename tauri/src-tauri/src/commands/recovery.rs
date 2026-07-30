//! 跨设备账户恢复命令。
//!
//! 主机端生成一个 6 位 PIN，把当前账户打包成 `.solosoul` 后通过 Noise_XX
//! 加密通道传送给新设备；新设备创建同名账户后导入数据，从而保证
//! `account_id` 一致，后续可直接使用 Device Sync。

use crate::commands::export_import::export::export_execute;
use crate::commands::export_import::import::import_execute;
use crate::commands::export_import::{ExportRequest, ExportScope};
use crate::state::AppState;
use solosoul_sync::recovery::{
    generate_recovery_password, push_to_receiver, recover_from_host, RecoveryHost,
    RecoveryReceiverServer,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Manager, State};

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

    let recovery_password = generate_recovery_password();

    // 临时导出文件路径
    let tmp_dir = std::env::temp_dir();
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let export_path = tmp_dir.join(format!(
        "solosoul_recovery_{}_{}.solosoul",
        account_id,
        nanoid()
    ));

    // 收集全部附件 ID，保证恢复包包含附件
    let all_attachment_ids = {
        let vault = crate::commands::vault_handle(&state)?;
        let objects = vault
            .list_objects(&account_id, None, None, None, false, false)
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
        ids
    };

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
        password: recovery_password.clone(),
        password_hint: Some("Recovery transfer".to_string()),
        save_path: export_path.to_string_lossy().to_string(),
    };

    // 复用现有导出命令生成加密恢复包（State 不可 Clone，从 AppHandle 重新获取）
    let export_state: State<'_, AppState> = app.state::<AppState>();
    export_execute(app.clone(), export_state, account_id.clone(), export_req).await?;

    // 取消并清理之前可能残留的主机（在锁外 join，避免阻塞）
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

    // 启动新的恢复主机（监听所有接口）
    let host = RecoveryHost::start(
        "0.0.0.0:0",
        export_path.clone(),
        recovery_password,
        account_id,
        account_name,
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

    {
        let mut rec = state.recovery_state.lock().map_err(|e| e.to_string())?;
        rec.host_cancel = host_cancel;
        rec.host_thread = Some(thread);
        rec.export_path = Some(export_path);
    }

    let qr_payload = serde_json::json!({
        "a": info.display_addr,
        "p": info.pin,
        "n": info.nonce.clone(),
        "f": info.fingerprint.clone()
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

/// 取消当前正在运行的恢复主机。
#[tauri::command]
pub async fn recovery_host_cancel(state: State<'_, AppState>) -> Result<(), String> {
    let (thread, path, receiver_thread) = {
        let mut rec = state.recovery_state.lock().map_err(|e| e.to_string())?;
        rec.host_cancel.store(true, Ordering::SeqCst);
        rec.receiver_cancel.store(true, Ordering::SeqCst);
        (
            rec.host_thread.take(),
            rec.export_path.take(),
            rec.receiver_thread.take(),
        )
    };
    if let Some(thread) = thread {
        let _ = thread.join();
    }
    if let Some(path) = path {
        let _ = std::fs::remove_file(&path);
    }
    if let Some(receiver_thread) = receiver_thread {
        let _ = receiver_thread.join();
    }
    Ok(())
}

/// 启动反向恢复接收端服务器，返回供移动端扫描的 QR payload。
#[tauri::command]
pub async fn recovery_receive_listen_start(
    state: State<'_, AppState>,
) -> Result<RecoveryHostInfo, String> {
    // 取消并清理之前可能残留的反向接收端（在锁外 join，避免阻塞）
    let old_receiver_thread = {
        let mut rec = state.recovery_state.lock().map_err(|e| e.to_string())?;
        rec.receiver_cancel.store(true, Ordering::SeqCst);
        rec.receiver_thread.take()
    };
    if let Some(old_thread) = old_receiver_thread {
        let _ = old_thread.join();
    }

    let server = RecoveryReceiverServer::start("0.0.0.0:0")?;
    let info = server.connection_info();
    let receiver_cancel = Arc::new(AtomicBool::new(false));
    let thread_cancel = receiver_cancel.clone();

    let thread = std::thread::spawn(move || server.run(thread_cancel));

    {
        let mut rec = state.recovery_state.lock().map_err(|e| e.to_string())?;
        rec.receiver_cancel = receiver_cancel;
        rec.receiver_thread = Some(thread);
    }

    let qr_payload = serde_json::json!({
        "t": "rev",
        "a": info.display_addr,
        "p": info.pin,
        "n": info.nonce.clone(),
        "f": info.fingerprint.clone()
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

/// 等待反向恢复接收完成，创建账户并导入数据。
#[tauri::command]
pub async fn recovery_receive_listen_wait(
    state: State<'_, AppState>,
    master_password: String,
) -> Result<ImportResultSummary, String> {
    if master_password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }

    let thread = {
        let mut rec = state.recovery_state.lock().map_err(|e| e.to_string())?;
        rec.receiver_thread.take()
    };

    let result: solosoul_sync::recovery::RecoveryTransferResult = if let Some(thread) = thread {
        tokio::task::spawn_blocking(move || match thread.join() {
            Ok(inner) => inner,
            Err(e) => Err(format!("Thread panicked: {:?}", e)),
        })
        .await
        .map_err(|e| e.to_string())??
    } else {
        return Err("No recovery listener is running".to_string());
    };

    // 使用主机的 account_id 和 account_name 创建本地账户
    {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        // 如果账户名冲突，让 create_account_with_id 返回错误，由前端提示
        svc.create_account_with_id(
            &result.account_id,
            &result.account_name,
            &master_password,
            None,
        )?;
    }

    // 导入恢复包
    let file_path = result.downloaded_path.to_string_lossy().to_string();
    let account_id = result.account_id;
    let import_result = import_execute(
        state.clone(),
        account_id.clone(),
        file_path.clone(),
        result.recovery_password,
    )
    .await;

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
            let _ = std::fs::remove_file(&file_path);
            return Err(e);
        }
    };

    let _ = std::fs::remove_file(&file_path);

    Ok(ImportResultSummary {
        object_count: import_result.object_count,
        attachment_count: import_result.attachment_count,
    })
}

/// 反向恢复模式：移动端扫描桌面端 QR 后，连接桌面并推送当前账户数据。
#[tauri::command]
pub async fn recovery_host_push(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    host_addr: String,
    pin: String,
    fingerprint: Option<String>,
    nonce: Option<String>,
) -> Result<(), String> {
    if host_addr.trim().is_empty() {
        return Err("Host address is required".to_string());
    }
    if pin.len() != 6 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err("PIN must be a 6-digit code".to_string());
    }

    let account_id = crate::commands::current_account_optional(&state)
        .ok_or("No account is currently unlocked")?;
    let account_name = get_current_account_name(&state, &account_id)?;

    let tmp_dir = std::env::temp_dir();
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let export_path = tmp_dir.join(format!(
        "solosoul_reverse_recovery_{}_{}.solosoul",
        account_id,
        nanoid()
    ));

    let recovery_password = generate_recovery_password();

    // 收集全部附件 ID
    let all_attachment_ids = {
        let vault = crate::commands::vault_handle(&state)?;
        let objects = vault
            .list_objects(&account_id, None, None, None, false, false)
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
        ids
    };

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
        password: recovery_password.clone(),
        password_hint: Some("Reverse recovery transfer".to_string()),
        save_path: export_path.to_string_lossy().to_string(),
    };

    // 复用现有导出命令生成加密恢复包
    let export_state: State<'_, AppState> = app.state::<AppState>();
    export_execute(app.clone(), export_state, account_id.clone(), export_req).await?;

    // 推送至接收端
    let export_path_for_push = export_path.clone();
    tokio::task::spawn_blocking(move || {
        push_to_receiver(
            &host_addr,
            &pin,
            fingerprint.as_deref(),
            nonce.as_deref(),
            &export_path_for_push,
            recovery_password,
            account_id,
            account_name,
        )
    })
    .await
    .map_err(|e| e.to_string())??;

    // 清理临时导出文件
    let _ = std::fs::remove_file(&export_path);

    Ok(())
}

/// 从恢复主机下载加密恢复包，创建与主机相同 account_id 的账户，并导入数据。
#[tauri::command]
pub async fn recovery_restore_from_host(
    state: State<'_, AppState>,
    master_password: String,
    host_addr: String,
    pin: String,
    fingerprint: Option<String>,
    nonce: Option<String>,
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

    let dest_dir = std::env::temp_dir().join("solosoul_recovery_downloads");
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    let result = tokio::task::spawn_blocking(move || {
        recover_from_host(
            &host_addr,
            &pin,
            &dest_dir,
            fingerprint.as_deref(),
            nonce.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("Recovery task failed: {}", e))?;
    let result = result?;

    let account_id = result.account_id;
    let file_path = result.downloaded_path.to_string_lossy().to_string();
    let recovery_password = result.recovery_password;

    // 使用主机的 account_id 和 account_name 创建本地账户
    {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        // 如果账户名冲突，让 create_account_with_id 返回错误，由前端提示
        svc.create_account_with_id(&account_id, &result.account_name, &master_password, None)?;
    }

    // 导入恢复包
    let import_result = import_execute(
        state.clone(),
        account_id.clone(),
        file_path.clone(),
        recovery_password,
    )
    .await;

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

    Ok(ImportResultSummary {
        object_count: import_result.object_count,
        attachment_count: import_result.attachment_count,
    })
}
