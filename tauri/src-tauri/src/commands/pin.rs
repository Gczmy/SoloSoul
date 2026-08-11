//! PIN 码解锁 Tauri 命令。
//!
//! 业务逻辑位于 `solosoul_core::pin::PinManager`；此文件仅包含薄包装层。
//! 错误统一使用 `__PIN_ERR__:<code>` 格式返回，便于前端国际化。

use crate::commands::object::trash::run_expired_trash_cleanup;
use crate::state::AppState;
use solosoul_core::pin::{PinError, PinManager, PinStatus};
use solosoul_core::AccountSummary;
use tauri::State;

const PIN_ERR_PREFIX: &str = "__PIN_ERR__:";

fn pin_err(code: &str) -> String {
    format!("{}{}", PIN_ERR_PREFIX, code)
}

fn map_pin_error(e: PinError) -> String {
    pin_err(e.code())
}

/// 检查账户的 PIN 配置状态。
#[tauri::command]
pub async fn pin_check_availability(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<PinStatus, String> {
    let vault_service = state.vault_service.clone();
    tokio::task::spawn_blocking(move || {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let manager = PinManager::new(svc.base_path().clone());
        Ok::<_, String>(manager.status(&account_id))
    })
    .await
    .map_err(|e| format!("pin_check_availability task failed: {}", e))?
}

/// 设置 PIN 码（需要验证主密码）。
/// P016: password/pin 在命令入口 Zeroizing 包装，避免明文残留堆内存。
#[tauri::command]
pub async fn pin_setup(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    pin: String,
) -> Result<(), String> {
    let password = zeroize::Zeroizing::new(password);
    let pin = zeroize::Zeroizing::new(pin);
    let vault_service = state.vault_service.clone();
    tokio::task::spawn_blocking(move || {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let manager = PinManager::new(svc.base_path().clone());
        manager
            .setup_pin(&account_id, &password, &pin, &svc)
            .map_err(map_pin_error)
    })
    .await
    .map_err(|e| format!("pin_setup task failed: {}", e))?
}

/// 使用 PIN 码解锁 Vault，返回账户信息（id + name），省去前端额外调用 vault_list_accounts。
/// P016: pin 在命令入口 Zeroizing 包装，避免明文残留堆内存。
#[tauri::command]
pub async fn pin_unlock(
    state: State<'_, AppState>,
    account_id: String,
    pin: String,
    location: Option<String>,
    action: Option<String>,
) -> Result<AccountSummary, String> {
    let pin = zeroize::Zeroizing::new(pin);
    let vault_service = state.vault_service.clone();
    let summary = tokio::task::spawn_blocking(move || {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let manager = PinManager::new(svc.base_path().clone());
        manager
            .unlock_with_pin(
                &account_id,
                &pin,
                &svc,
                location.as_deref(),
                action.as_deref(),
            )
            .map_err(map_pin_error)?;
        // 解锁成功后查找账户名返回
        let accounts = svc.list_accounts();
        accounts
            .into_iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| "Account not found after PIN unlock".to_string())
    })
    .await
    .map_err(|e| format!("pin_unlock task failed: {}", e))??;

    // PIN 解锁成功后自动清理过期回收站项目
    run_expired_trash_cleanup(&state);

    Ok(summary)
}

/// 禁用 PIN 码（需要验证主密码）。
/// P016: password 在命令入口 Zeroizing 包装，避免明文残留堆内存。
#[tauri::command]
pub async fn pin_disable(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    let password = zeroize::Zeroizing::new(password);
    let vault_service = state.vault_service.clone();
    tokio::task::spawn_blocking(move || {
        let svc = vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let manager = PinManager::new(svc.base_path().clone());
        manager
            .disable_pin(&account_id, &password, &svc)
            .map_err(map_pin_error)
    })
    .await
    .map_err(|e| format!("pin_disable task failed: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_err_format() {
        let err = pin_err("incorrect");
        assert_eq!(err, "__PIN_ERR__:incorrect");
    }

    #[test]
    fn test_map_pin_error() {
        let err = map_pin_error(PinError::Incorrect);
        assert_eq!(err, "__PIN_ERR__:incorrect");

        let err = map_pin_error(PinError::Locked);
        assert_eq!(err, "__PIN_ERR__:locked");

        let err = map_pin_error(PinError::NotConfigured);
        assert_eq!(err, "__PIN_ERR__:not_configured");
    }
}
