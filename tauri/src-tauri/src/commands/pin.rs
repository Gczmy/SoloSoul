//! PIN 码解锁 Tauri 命令。
//!
//! 业务逻辑位于 `solosoul_core::pin::PinManager`；此文件仅包含薄包装层。
//! 错误统一使用 `__PIN_ERR__:<code>` 格式返回，便于前端国际化。

use crate::state::AppState;
use solosoul_core::pin::{PinError, PinManager, PinStatus};
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
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = PinManager::new(svc.base_path().clone());
    Ok(manager.status(&account_id))
}

/// 设置 PIN 码（需要验证主密码）。
#[tauri::command]
pub async fn pin_setup(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    pin: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = PinManager::new(svc.base_path().clone());
    // setup_pin 会调用 svc.unlock 验证主密码并获取会话密钥
    manager
        .setup_pin(&account_id, &password, &pin, &svc)
        .map_err(map_pin_error)
}

/// 使用 PIN 码解锁 Vault。
#[tauri::command]
pub async fn pin_unlock(
    state: State<'_, AppState>,
    account_id: String,
    pin: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = PinManager::new(svc.base_path().clone());
    manager
        .unlock_with_pin(&account_id, &pin, &svc)
        .map_err(map_pin_error)
}

/// 禁用 PIN 码（需要验证主密码）。
#[tauri::command]
pub async fn pin_disable(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = PinManager::new(svc.base_path().clone());
    manager
        .disable_pin(&account_id, &password, &svc)
        .map_err(map_pin_error)
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
