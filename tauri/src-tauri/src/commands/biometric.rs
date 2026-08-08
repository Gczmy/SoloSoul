//! Biometric (Touch ID/Face ID/Windows Hello) commands.
//!
//! Business logic lives in `solosoul_core::biometric::BiometricManager`; this
//! file only contains the thin `#[tauri::command]` wrappers and audit logging
//! that depends on the unlocked vault store.

use crate::commands::object::trash::run_expired_trash_cleanup;
use crate::state::AppState;
use solosoul_core::biometric::{BiometricAvailability, BiometricError, BiometricManager};
use tauri::State;

#[cfg(desktop)]
use solosoul_core::biometric::trigger_system_biometric;

const BIO_ERR_PREFIX: &str = "__BIO_ERR__:";

fn bio_err(code: &str) -> String {
    format!("{}{}", BIO_ERR_PREFIX, code)
}

/// 将 Keystore 插件返回的错误字符串映射为 BiometricError。
#[cfg(target_os = "android")]
fn map_keystore_error(e: String, operation: &str) -> String {
    if e == "BIOMETRIC_KEY_INVALIDATED" || e == "BIOMETRIC_KEY_NOT_FOUND" {
        map_bio_error(BiometricError::KeychainItemNotFound, operation)
    } else if e == "BIOMETRIC_CANCELLED" {
        map_bio_error(BiometricError::UserPresenceCancelled, operation)
    } else if e == "BIOMETRIC_NOT_ENROLLED" {
        map_bio_error(BiometricError::UserPresenceUnavailable, operation)
    } else if e == "BIOMETRIC_LOCKOUT" {
        // 临时锁定：保留独立错误码，让前端/调用方可以区分
        bio_err("lockout")
    } else if e == "BIOMETRIC_UNAVAILABLE" {
        map_bio_error(BiometricError::UserPresenceUnavailable, operation)
    } else if e.starts_with("BIOMETRIC_ERROR:") {
        map_bio_error(BiometricError::Other(e), operation)
    } else if operation == "save" {
        map_bio_error(BiometricError::KeychainWriteFailed(e), operation)
    } else {
        map_bio_error(BiometricError::KeychainReadFailed(e), operation)
    }
}

/// 将 BiometricError 映射为前端可国际化的短代码。
fn map_bio_error(e: BiometricError, operation: &str) -> String {
    let code = match &e {
        BiometricError::PlatformNotSupported => "platform_not_supported",
        BiometricError::UserPresenceCancelled => "cancelled",
        BiometricError::UserPresenceUnavailable => "user_presence_unavailable",
        BiometricError::KeychainItemNotFound => "not_configured",
        BiometricError::MissingKeychainEntitlement => "keychain_write_failed",
        BiometricError::InvalidKeyFormat => "invalid_key_format",
        BiometricError::LegacyMigrationFailed(_) => "stale_credential",
        BiometricError::Other(msg) => {
            let lower = msg.to_lowercase();
            if lower.contains("invalid password") {
                return bio_err("invalid_password");
            }
            match operation {
                "save" => "save_failed",
                "unlock" => "unlock_failed",
                "delete" => "delete_failed",
                _ => "unknown",
            }
        }
        BiometricError::KeychainWriteFailed(_) => match operation {
            "save" => "keychain_write_failed",
            "delete" => "delete_failed",
            _ => "keychain_write_failed",
        },
        BiometricError::KeychainReadFailed(_) => match operation {
            "unlock" => "keychain_read_failed",
            _ => "keychain_read_failed",
        },
    };
    bio_err(code)
}

/// 读取 keystore_data.json 双槽凭证（兼容旧版扁平格式，解析失败/不存在返回 None）。
#[cfg(target_os = "android")]
fn read_keystore_credentials(
    base: &std::path::Path,
    account_id: &str,
) -> Option<crate::keystore_plugin::KeystoreCredentials> {
    let path = base.join(account_id).join("keystore_data.json");
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

#[cfg(desktop)]
#[tauri::command]
pub async fn biometric_check_availability(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<BiometricAvailability, String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());
    // 旧版本地文件凭证在查询时自动迁移到 Keychain；失败也不影响显示。
    if !account_id.is_empty() {
        let _ = manager.migrate_legacy_if_needed(&account_id);
    }
    let result = manager.availability(&account_id);
    if !account_id.is_empty() {
        tracing::debug!(
            "biometric_check_availability account_id={} available={} configured={} biometry_type={:?}",
            account_id,
            result.available,
            result.configured,
            result.biometry_type
        );
    }
    Ok(result)
}

#[cfg(all(mobile, any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn biometric_check_availability(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
) -> Result<BiometricAvailability, String> {
    use tauri_plugin_biometric::{BiometricExt, BiometryType};

    // 插件 status() 失败不中断检测（部分设备/系统版本会报错），
    // Android 可用性判定以下方自有 Keystore 插件检测为准
    let (status_available, status_biometry, status_error) = match app.biometric().status() {
        Ok(s) => (s.is_available, Some(s.biometry_type), s.error),
        Err(_) => (false, None, None),
    };
    let plugin_biometry_type = match status_biometry {
        Some(BiometryType::TouchID) => Some("touchId".to_string()),
        Some(BiometryType::FaceID) => Some("faceId".to_string()),
        _ => None,
    };

    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;

    // Android 使用 Keystore 双槽存储（strong/weak 各自独立），iOS 沿用 FileBiometricStorage
    #[cfg(target_os = "android")]
    let (configured, strong_configured, weak_configured) = {
        prune_stale_keystore_slots(&svc, &app, &account_id);
        let creds = read_keystore_credentials(svc.base_path(), &account_id);
        let strong_configured = creds.as_ref().and_then(|c| c.strong.as_ref()).is_some();
        let weak_configured = creds.as_ref().and_then(|c| c.weak.as_ref()).is_some();
        (
            strong_configured || weak_configured,
            strong_configured,
            weak_configured,
        )
    };

    #[cfg(target_os = "ios")]
    let (configured, strong_configured, weak_configured) = {
        let manager = BiometricManager::new(svc.base_path().clone());
        let configured = manager.is_configured(&account_id);
        (configured, configured, false)
    };

    // Android：以自有 Keystore 插件检测为准（区分 Class 3 / Class 2）。
    // 不再以 tauri-plugin-biometric 的 status 为唯一依据——旧 API 级别
    // （<30）其弱生物识别检查退化为指纹检查，会漏检 Class 2 人脸。
    #[cfg(target_os = "android")]
    let (available, weak_available, strong_available, effective_type, debug, system_lockout) =
        resolve_android_availability(&app, status_available, plugin_biometry_type.clone());

    #[cfg(target_os = "ios")]
    let (available, weak_available, strong_available, effective_type, debug) = (
        status_available,
        false,
        status_available,
        plugin_biometry_type.clone(),
        None,
    );
    #[cfg(target_os = "ios")]
    let system_lockout = false;

    let lockout = state.is_biometric_locked_out() || system_lockout;
    let lockout_until = if lockout {
        state.biometric_lockout_until_ts()
    } else {
        None
    };

    Ok(BiometricAvailability {
        available,
        configured,
        biometry_type: effective_type,
        error: status_error,
        weak_available,
        debug,
        strong_available,
        strong_configured,
        weak_configured,
        lockout,
        lockout_until,
    })
}

/// 卸载/换机后 Keystore 密钥已被系统擦除，但 keystore_data.json 可能从 SAF
/// 远端同步残留（陈旧凭证）。校验密钥真实存在并清理失效槽位，避免安全设置
/// 显示"幽灵开启"；桥接失败时保守保留槽位，不误删有效凭证。
#[cfg(target_os = "android")]
fn prune_stale_keystore_slots(
    svc: &solosoul_core::vault_service::VaultService,
    app: &tauri::AppHandle,
    account_id: &str,
) {
    use crate::keystore_plugin::KeystorePluginHandle;
    use tauri::Manager;

    let mut creds = read_keystore_credentials(svc.base_path(), account_id);
    let keystore = app.try_state::<KeystorePluginHandle<tauri::Wry>>();

    let mut pruned = false;
    if let (Some(c), Some(ks)) = (creds.as_mut(), keystore.as_ref()) {
        if c.strong.is_some() && ks.key_exists(account_id, None) == Ok(false) {
            c.strong = None;
            pruned = true;
        }
        if c.weak.is_some() && ks.key_exists(account_id, Some("weak")) == Ok(false) {
            c.weak = None;
            pruned = true;
        }
    }
    if pruned {
        if let Some(c) = creds.as_ref() {
            let path = svc.base_path().join(account_id).join("keystore_data.json");
            if c.is_empty() {
                let _ = std::fs::remove_file(&path);
            } else if let Ok(json) = serde_json::to_string(c) {
                let _ = std::fs::write(&path, json);
            }
            tracing::info!(
                "biometric_check_availability: pruned stale keystore slots for account={}",
                account_id
            );
        }
    }
}

/// Android 可用性判定：以自有 Keystore 插件检测为准（区分 Class 3 / Class 2），
/// 记录完整调用结果以便区分 state 未注册（None）/ 桥接失败（Err）/ 正常。
#[cfg(target_os = "android")]
fn resolve_android_availability(
    app: &tauri::AppHandle,
    status_available: bool,
    plugin_biometry_type: Option<String>,
) -> (bool, bool, bool, Option<String>, Option<String>, bool) {
    use crate::keystore_plugin::KeystorePluginHandle;
    use tauri::Manager;

    let keystore_result = app
        .try_state::<KeystorePluginHandle<tauri::Wry>>()
        .map(|keystore| keystore.check_biometric_availability());
    tracing::info!(
        "biometric_check_availability: keystore_result={:?}",
        keystore_result
    );
    // 诊断字符串无条件生成：桥接失败时也要让前端有内容可显示，
    // 否则无法区分「旧构建」与「桥接失败」
    let (bridge_desc, info) = match keystore_result {
        None => ("missing".to_string(), None),
        Some(Err(e)) => (format!("err:{e}"), None),
        Some(Ok(i)) => (
            format!(
                "ok strong={} weak={} lockout={} sdk={:?} faceFeature={:?} strongRaw={:?} weakRaw={:?}",
                i.strong_available,
                i.weak_available,
                i.lockout,
                i.sdk_int,
                i.face_feature,
                i.strong_raw,
                i.weak_raw
            ),
            Some(i),
        ),
    };
    let strong = info.as_ref().map(|i| i.strong_available).unwrap_or(false);
    let weak = info.as_ref().map(|i| i.weak_available).unwrap_or(false);
    let system_lockout = info.as_ref().map(|i| i.lockout).unwrap_or(false);
    tracing::info!(
        "biometric_check_availability: strong={}, weak={}, lockout={}, plugin_status={}",
        strong,
        weak,
        system_lockout,
        status_available
    );
    let available = strong || weak || status_available;
    // weak 独立上报：设备同时有 Class 3 时，Face ID（Class 2）也作为独立开关显示
    let weak_available = weak;
    let effective_type = if strong {
        plugin_biometry_type.clone().or(Some("touchId".to_string()))
    } else if weak || status_available {
        Some("faceId".to_string())
    } else {
        plugin_biometry_type.clone()
    };
    let debug = Some(format!(
        "pluginStatus={} pluginType={:?} bridge={}",
        status_available, plugin_biometry_type, bridge_desc
    ));
    (
        available,
        weak_available,
        strong,
        effective_type,
        debug,
        system_lockout,
    )
}

#[cfg(desktop)]
#[tauri::command]
pub async fn biometric_save_credential(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
    // 桌面端无 Class 2 概念，仅保持与移动端一致的参数签名
    _authenticator: Option<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());

    // 保存前校验即将写入的密钥与当前 Vault 会话密钥一致，避免钥匙串/文件访问问题导致回读失败。
    // P204: 会话密钥 hex 与派生密钥 hex 均以 Zeroizing<String> 持有，函数返回即安全擦除，
    // 避免主密钥明文 hex 长期残留在普通 String 堆内存中。
    if let Some(session_key) = svc.get_session_key() {
        let expected = zeroize::Zeroizing::new(hex::encode(session_key.as_slice()));
        let derived = zeroize::Zeroizing::new(
            manager
                .derive_key_hex(&password, &account_id)
                .map_err(|e| map_bio_error(e, "save"))?,
        );
        if *derived != *expected {
            return Err(bio_err("credential_mismatch"));
        }
    }

    // reason 根据 biometry_type 动态生成，避免在 Windows Hello 设备上显示 Touch ID
    let save_reason = match biometry_type.as_deref() {
        Some("windowsHello") => "verify your identity to enable Windows Hello for SoloSoul",
        _ => "verify your identity to enable Touch ID / Face ID for SoloSoul",
    };
    manager
        .save_credential(&account_id, &password, save_reason)
        .map_err(|e| map_bio_error(e, "save"))?;

    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        let loc = location.unwrap_or_else(|| "unknown".to_string());
        let act = action.unwrap_or_else(|| "enable".to_string());
        let bio_type = biometry_type.as_deref().unwrap_or("unknown");
        let _ = vault.log_structured(
            "biometric_saved",
            "biometric",
            Some(&account_id),
            None,
            "user",
            Some(&format!(
                "location={} action={} type={}",
                loc, act, bio_type
            )),
        );
    }
    Ok(())
}

#[cfg(all(mobile, any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn biometric_save_credential(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
    // "weak" 强制写入 weak 槽（Face ID Class 2）；缺省写 strong 槽（Touch ID）
    authenticator: Option<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());

    // 1. 验证主密码
    manager
        .verify_password(&password, &account_id)
        .map_err(|e| map_bio_error(e, "save"))?;

    // 2. 派生主密钥并使用平台安全存储保存
    let key_hex = manager
        .derive_key_hex(&password, &account_id)
        .map_err(|e| map_bio_error(e, "save"))?;

    #[cfg(target_os = "android")]
    {
        use crate::keystore_plugin::{
            BiometricPromptInfo, KeystoreCredentials, KeystorePluginHandle,
        };
        use tauri::Manager;

        let keystore = app.state::<KeystorePluginHandle<tauri::Wry>>();
        let slot = keystore
            .authenticate_and_save(
                &account_id,
                &key_hex,
                BiometricPromptInfo {
                    title: "SoloSoul",
                    subtitle: "Enable biometric authentication",
                    cancel_title: "Cancel",
                },
                authenticator.as_deref(),
            )
            .map_err(|e| {
                if e == "BIOMETRIC_LOCKOUT" {
                    state.set_biometric_lockout(std::time::Duration::from_secs(30));
                }
                map_keystore_error(e, "save")
            })?;

        // 生物识别验证成功，清除可能存在的临时锁定状态
        state.clear_biometric_lockout();

        // 双槽读改写：只更新本次选择的槽，保留另一种方式的凭证
        let path = svc.base_path().join(&account_id).join("keystore_data.json");
        let mut creds = read_keystore_credentials(svc.base_path(), &account_id).unwrap_or(
            KeystoreCredentials {
                strong: None,
                weak: None,
            },
        );
        if authenticator.as_deref() == Some("weak") {
            creds.weak = Some(slot);
        } else {
            creds.strong = Some(slot);
        }
        let json = serde_json::to_string(&creds)
            .map_err(|e| map_bio_error(BiometricError::Other(format!("serialize: {e}")), "save"))?;
        std::fs::write(&path, json).map_err(|e| {
            map_bio_error(BiometricError::KeychainWriteFailed(e.to_string()), "save")
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)
                .map_err(|e| map_bio_error(BiometricError::Other(format!("stat: {e}")), "save"))?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&path, perms)
                .map_err(|e| map_bio_error(BiometricError::Other(format!("chmod: {e}")), "save"))?;
        }

        // 删除旧版 FileBiometricStorage 凭证，避免弱加密文件残留
        let legacy_path = svc.base_path().join(&account_id).join("biometric_key");
        let _ = std::fs::remove_file(&legacy_path);
    }

    #[cfg(target_os = "ios")]
    {
        use solosoul_core::biometric::BiometricStorage;
        let storage =
            solosoul_core::biometric::ios::IosBiometricStorage::new(svc.base_path().clone());
        storage
            .save(
                &account_id,
                &key_hex,
                "verify your identity to enable biometric authentication for SoloSoul",
            )
            .map_err(|e| map_bio_error(e, "save"))?;
    }

    // 3. 更新配置标记
    manager
        .set_config_flag(&account_id, true)
        .map_err(|e| map_bio_error(e, "save"))?;

    // 4. 审计日志
    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        let loc = location.unwrap_or_else(|| "unknown".to_string());
        let act = action.unwrap_or_else(|| "enable".to_string());
        let bio_type = biometry_type.as_deref().unwrap_or("unknown");
        let _ = vault.log_structured(
            "biometric_saved",
            "biometric",
            Some(&account_id),
            None,
            "user",
            Some(&format!(
                "location={} action={} type={}",
                loc, act, bio_type
            )),
        );
    }

    Ok(())
}

#[cfg(desktop)]
#[tauri::command]
pub async fn biometric_unlock(
    state: State<'_, AppState>,
    account_id: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());
    let used_bio_type = manager
        .unlock(&account_id, &svc, "unlock SoloSoul")
        .map_err(|e| map_bio_error(e, "unlock"))?;

    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        let loc = location.unwrap_or_else(|| "unknown".to_string());
        let act = action.unwrap_or_else(|| "unlock".to_string());
        let bio_type = biometry_type.as_deref().unwrap_or(&used_bio_type);
        // Critical field access produces a more detailed frontend-side audit
        // entry (critical_field_touch_id / critical_field_face_id), so skip
        // the generic biometric unlock entry to avoid duplicates.
        if loc != "critical_data_access" {
            let action_type = match bio_type {
                "touchId" => "touch_id_unlock",
                "faceId" => "face_id_unlock",
                "windowsHello" => "windows_hello_unlock",
                _ => "biometric_unlock",
            };
            let _ = vault.log_structured(
                action_type,
                "biometric",
                Some(&account_id),
                None,
                "user",
                Some(&format!(
                    "location={} action={} type={}",
                    loc, act, bio_type
                )),
            );
        }
    }

    // 生物识别解锁成功后自动清理过期回收站项目
    run_expired_trash_cleanup(&state);

    Ok(())
}

#[cfg(all(mobile, any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn biometric_unlock(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    // 1. 读取已保存的主密钥（Android 通过生物识别提示保护）
    let key_hex = {
        #[cfg(target_os = "android")]
        {
            use crate::keystore_plugin::{BiometricPromptInfo, KeystorePluginHandle};
            use tauri::Manager;

            let creds = read_keystore_credentials(svc.base_path(), &account_id)
                .ok_or_else(|| map_bio_error(BiometricError::KeychainItemNotFound, "unlock"))?;

            let keystore = app.state::<KeystorePluginHandle<tauri::Wry>>();
            let prompt = BiometricPromptInfo {
                title: "SoloSoul",
                subtitle: "Unlock with biometric authentication",
                cancel_title: "Cancel",
            };
            // weak 槽存在：普通提示同时接受指纹与人脸（用户任选），用 weak 密钥解密；
            // 仅 strong 槽：Class 3 CryptoObject 提示（仅指纹/强人脸）
            if let Some(slot) = &creds.weak {
                keystore
                    .authenticate_and_read(
                        &account_id,
                        &slot.iv,
                        &slot.ciphertext,
                        prompt,
                        Some("any"),
                    )
                    .map_err(|e| {
                        if e == "BIOMETRIC_LOCKOUT" {
                            state.set_biometric_lockout(std::time::Duration::from_secs(30));
                        }
                        map_keystore_error(e, "unlock")
                    })?
            } else if let Some(slot) = &creds.strong {
                keystore
                    .authenticate_and_read(&account_id, &slot.iv, &slot.ciphertext, prompt, None)
                    .map_err(|e| {
                        if e == "BIOMETRIC_LOCKOUT" {
                            state.set_biometric_lockout(std::time::Duration::from_secs(30));
                        }
                        map_keystore_error(e, "unlock")
                    })?
            } else {
                return Err(map_bio_error(
                    BiometricError::KeychainItemNotFound,
                    "unlock",
                ));
            }
        }

        #[cfg(target_os = "ios")]
        {
            use solosoul_core::biometric::BiometricStorage;
            let storage =
                solosoul_core::biometric::ios::IosBiometricStorage::new(svc.base_path().clone());
            storage
                .read(&account_id, "unlock SoloSoul")
                .map_err(|e| map_bio_error(e, "unlock"))?
        }
    };

    // 生物识别验证成功，清除可能存在的临时锁定状态
    state.clear_biometric_lockout();

    let key_bytes = hex::decode(&key_hex)
        .map_err(|_| map_bio_error(BiometricError::InvalidKeyFormat, "unlock"))?;
    let key: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| map_bio_error(BiometricError::InvalidKeyFormat, "unlock"))?;

    // 2. 解锁 Vault
    svc.unlock_with_session_key(&account_id, &key)
        .map_err(|e| map_bio_error(BiometricError::Other(format!("{:#}", e)), "unlock"))?;

    // 3. 审计日志
    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        let loc = location.unwrap_or_else(|| "unknown".to_string());
        let act = action.unwrap_or_else(|| "unlock".to_string());
        let bio_type = biometry_type.as_deref().unwrap_or("unknown");
        if loc != "critical_data_access" {
            let action_type = match bio_type {
                "touchId" => "touch_id_unlock",
                "faceId" => "face_id_unlock",
                _ => "biometric_unlock",
            };
            let _ = vault.log_structured(
                action_type,
                "biometric",
                Some(&account_id),
                None,
                "user",
                Some(&format!(
                    "location={} action={} type={}",
                    loc, act, bio_type
                )),
            );
        }
    }

    // 生物识别解锁成功后自动清理过期回收站项目
    run_expired_trash_cleanup(&state);

    Ok(())
}

#[cfg(desktop)]
#[tauri::command]
pub async fn biometric_delete_credential(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
    // 桌面端无 Class 2 概念，仅保持与移动端一致的参数签名
    _authenticator: Option<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());
    manager
        .delete_credential(&account_id, &password)
        .map_err(|e| map_bio_error(e, "delete"))?;

    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        let loc = location.unwrap_or_else(|| "unknown".to_string());
        let act = action.unwrap_or_else(|| "disable".to_string());
        let bio_type = biometry_type.as_deref().unwrap_or("unknown");
        let _ = vault.log_structured(
            "biometric_deleted",
            "biometric",
            Some(&account_id),
            None,
            "user",
            Some(&format!(
                "location={} action={} type={}",
                loc, act, bio_type
            )),
        );
    }
    Ok(())
}

#[cfg(all(mobile, any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn biometric_delete_credential(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
    // "weak" 只清 weak 槽（Face ID Class 2）；缺省只清 strong 槽（Touch ID）
    authenticator: Option<String>,
) -> Result<(), String> {
    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let manager = BiometricManager::new(svc.base_path().clone());

    // 1. 验证主密码
    manager
        .verify_password(&password, &account_id)
        .map_err(|e| map_bio_error(e, "delete"))?;

    // 2. 删除移动端安全存储中的对应槽凭证
    #[cfg(target_os = "android")]
    let remaining_any = {
        use crate::keystore_plugin::{KeystoreCredentials, KeystorePluginHandle};
        use tauri::Manager;

        let keystore = app.state::<KeystorePluginHandle<tauri::Wry>>();
        let _ = keystore.delete(&account_id, authenticator.as_deref());

        let path = svc.base_path().join(&account_id).join("keystore_data.json");
        let mut creds = read_keystore_credentials(svc.base_path(), &account_id).unwrap_or(
            KeystoreCredentials {
                strong: None,
                weak: None,
            },
        );
        if authenticator.as_deref() == Some("weak") {
            creds.weak = None;
        } else {
            creds.strong = None;
        }
        let remaining = !creds.is_empty();
        if remaining {
            if let Ok(json) = serde_json::to_string(&creds) {
                let _ = std::fs::write(&path, json);
            }
        } else {
            let _ = std::fs::remove_file(&path);
            // 同时清理可能存在的旧版 FileBiometricStorage 凭证
            let legacy_path = svc.base_path().join(&account_id).join("biometric_key");
            let _ = std::fs::remove_file(&legacy_path);
        }
        remaining
    };

    #[cfg(target_os = "ios")]
    let remaining_any = {
        use solosoul_core::biometric::BiometricStorage;
        let storage =
            solosoul_core::biometric::ios::IosBiometricStorage::new(svc.base_path().clone());
        match storage.delete(&account_id) {
            Ok(()) | Err(BiometricError::KeychainItemNotFound) => false,
            Err(e) => return Err(map_bio_error(e, "delete")),
        }
    };

    // 3. 更新配置标记（另一槽仍有凭证时保持启用）
    manager
        .set_config_flag(&account_id, remaining_any)
        .map_err(|e| map_bio_error(e, "delete"))?;

    // 4. 审计日志
    if let Some(vg) = svc.get_vault_store() {
        let vault = vg.as_ref();
        let loc = location.unwrap_or_else(|| "unknown".to_string());
        let act = action.unwrap_or_else(|| "disable".to_string());
        let bio_type = biometry_type.as_deref().unwrap_or("unknown");
        let _ = vault.log_structured(
            "biometric_deleted",
            "biometric",
            Some(&account_id),
            None,
            "user",
            Some(&format!(
                "location={} action={} type={}",
                loc, act, bio_type
            )),
        );
    }

    Ok(())
}

#[cfg(desktop)]
#[tauri::command]
pub async fn biometric_test(account_id: String) -> Result<bool, String> {
    let _ = account_id;
    if !solosoul_core::biometric::is_macos() && std::env::consts::OS != "windows" {
        return Ok(false);
    }
    // 使用严格策略确保实际触发生物识别，不回落到设备密码。
    let reason = if solosoul_core::biometric::is_macos() {
        "test biometric authentication for SoloSoul"
    } else {
        "Test Windows Hello for SoloSoul"
    };
    trigger_system_biometric(reason, true).map_err(|e| map_bio_error(e, "test"))?;
    Ok(true)
}

#[cfg(all(mobile, any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn biometric_test(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
) -> Result<bool, String> {
    let _ = account_id;
    use tauri_plugin_biometric::{AuthOptions, BiometricExt};

    match app.biometric().authenticate(
        "Test biometric authentication for SoloSoul".to_string(),
        AuthOptions {
            allow_device_credential: false,
            cancel_title: Some("Cancel".to_string()),
            fallback_title: None,
            title: Some("SoloSoul".to_string()),
            subtitle: Some("Test biometric authentication".to_string()),
            confirmation_required: Some(false),
        },
    ) {
        Ok(()) => {
            state.clear_biometric_lockout();
            Ok(true)
        }
        Err(e) => {
            let msg = e.to_string();
            // tauri-plugin-biometric 在 Android/iOS 上的锁定错误通常包含 lockout/locked 字样
            if msg.to_lowercase().contains("lockout") || msg.to_lowercase().contains("locked") {
                state.set_biometric_lockout(std::time::Duration::from_secs(30));
            }
            Err(msg)
        }
    }
}

#[cfg(all(test, desktop))]
mod tests {
    use super::*;

    #[test]
    fn test_bio_err_format() {
        let err = bio_err("not_configured");
        assert_eq!(err, "__BIO_ERR__:not_configured");
    }

    #[test]
    fn test_map_bio_error_platform_not_supported() {
        let err = map_bio_error(BiometricError::PlatformNotSupported, "save");
        assert_eq!(err, "__BIO_ERR__:platform_not_supported");
    }

    #[test]
    fn test_map_bio_error_cancelled() {
        let err = map_bio_error(BiometricError::UserPresenceCancelled, "unlock");
        assert_eq!(err, "__BIO_ERR__:cancelled");
    }

    #[test]
    fn test_map_bio_error_unavailable() {
        let err = map_bio_error(BiometricError::UserPresenceUnavailable, "save");
        assert_eq!(err, "__BIO_ERR__:user_presence_unavailable");
    }

    #[test]
    fn test_map_bio_error_not_configured() {
        let err = map_bio_error(BiometricError::KeychainItemNotFound, "unlock");
        assert_eq!(err, "__BIO_ERR__:not_configured");
    }

    #[test]
    fn test_map_bio_error_other_invalid_password() {
        let err = map_bio_error(
            BiometricError::Other("Invalid password".to_string()),
            "unlock",
        );
        assert_eq!(err, "__BIO_ERR__:invalid_password");
    }

    #[test]
    fn test_map_bio_error_other_save_operation() {
        let err = map_bio_error(BiometricError::Other("disk full".to_string()), "save");
        assert_eq!(err, "__BIO_ERR__:save_failed");
    }

    #[test]
    fn test_map_bio_error_other_unlock_operation() {
        let err = map_bio_error(BiometricError::Other("timeout".to_string()), "unlock");
        assert_eq!(err, "__BIO_ERR__:unlock_failed");
    }

    #[test]
    fn test_map_bio_error_other_delete_operation() {
        let err = map_bio_error(
            BiometricError::Other("permission denied".to_string()),
            "delete",
        );
        assert_eq!(err, "__BIO_ERR__:delete_failed");
    }

    #[test]
    fn test_map_bio_error_other_unknown_operation() {
        let err = map_bio_error(
            BiometricError::Other("something else".to_string()),
            "unknown_op",
        );
        assert_eq!(err, "__BIO_ERR__:unknown");
    }

    #[test]
    fn test_map_bio_error_keychain_write_failed_save() {
        let err = map_bio_error(
            BiometricError::KeychainWriteFailed("write error".to_string()),
            "save",
        );
        assert_eq!(err, "__BIO_ERR__:keychain_write_failed");
    }

    #[test]
    fn test_map_bio_error_keychain_write_failed_delete() {
        let err = map_bio_error(
            BiometricError::KeychainWriteFailed("write error".to_string()),
            "delete",
        );
        assert_eq!(err, "__BIO_ERR__:delete_failed");
    }

    #[test]
    fn test_map_bio_error_keychain_read_failed() {
        let err = map_bio_error(
            BiometricError::KeychainReadFailed("read error".to_string()),
            "unlock",
        );
        assert_eq!(err, "__BIO_ERR__:keychain_read_failed");
    }

    #[test]
    fn test_map_bio_error_legacy_migration_failed() {
        let err = map_bio_error(
            BiometricError::LegacyMigrationFailed("mig error".to_string()),
            "save",
        );
        assert_eq!(err, "__BIO_ERR__:stale_credential");
    }

    #[test]
    fn test_map_bio_error_invalid_key_format() {
        let err = map_bio_error(BiometricError::InvalidKeyFormat, "save");
        assert_eq!(err, "__BIO_ERR__:invalid_key_format");
    }

    #[test]
    fn test_map_bio_error_missing_keychain_entitlement() {
        let err = map_bio_error(BiometricError::MissingKeychainEntitlement, "save");
        assert_eq!(err, "__BIO_ERR__:keychain_write_failed");
    }
}
