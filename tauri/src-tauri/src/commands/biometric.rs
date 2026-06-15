//! Biometric (Touch ID/Face ID/Windows Hello) commands.
//!
//! Business logic lives in `solosoul_core::biometric::BiometricManager`; this
//! file only contains the thin `#[tauri::command]` wrappers and audit logging
//! that depends on the unlocked vault store.

use crate::state::AppState;
use solosoul_core::biometric::{trigger_system_biometric, BiometricAvailability, BiometricManager};
use tauri::State;

#[tauri::command]
pub async fn biometric_check_availability(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<BiometricAvailability, String> {
    let svc = state.vault_service.read().unwrap();
    let manager = BiometricManager::new(svc.base_path().clone());
    let bt = if solosoul_core::biometric::is_macos() {
        Some("touchId".into())
    } else {
        None
    };
    let configured = manager.is_configured(&account_id);
    let available = bt.is_some();
    if !account_id.is_empty() {
        tracing::debug!(
            "biometric_check_availability account_id={} available={} configured={} biometry_type={:?}",
            account_id,
            available,
            configured,
            bt
        );
    }
    Ok(BiometricAvailability {
        available,
        configured,
        biometry_type: bt,
        error: if solosoul_core::biometric::is_macos() {
            None
        } else {
            Some("platform not supported".into())
        },
    })
}

#[tauri::command]
pub async fn biometric_save_credential(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    let manager = BiometricManager::new(svc.base_path().clone());
    manager.save_credential(
        &account_id,
        &password,
        "verify your identity to enable Touch ID for SoloSoul",
    )?;

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

#[tauri::command]
pub async fn biometric_unlock(
    state: State<'_, AppState>,
    account_id: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    let manager = BiometricManager::new(svc.base_path().clone());
    let used_bio_type = manager.unlock(&account_id, &svc, "unlock SoloSoul")?;

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
    Ok(())
}

#[tauri::command]
pub async fn biometric_delete_credential(
    state: State<'_, AppState>,
    account_id: String,
    password: String,
    location: Option<String>,
    action: Option<String>,
    biometry_type: Option<String>,
) -> Result<(), String> {
    let svc = state.vault_service.read().unwrap();
    let manager = BiometricManager::new(svc.base_path().clone());
    manager.delete_credential(&account_id, &password)?;

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

#[tauri::command]
pub async fn biometric_test(_account_id: String) -> Result<bool, String> {
    if !solosoul_core::biometric::is_macos() {
        return Ok(false);
    }
    trigger_system_biometric("test biometric authentication for SoloSoul")?;
    Ok(true)
}
