//! Authentication helpers shared by all SoloSoul hosts.

use crate::vault_service::AccountConfig;
use solosoul_crypto::kdf::{derive_key, KdfConfig};

/// Verify whether the given password matches the account's master password.
/// Does NOT modify any state (no unlocking, no session key storage).
pub fn verify_password_core(password: &str, config: &AccountConfig) -> Result<bool, String> {
    let salt_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &config.salt)
            .map_err(|_| "Invalid salt".to_string())?;
    let salt_arr: [u8; 16] = salt_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "Bad salt".to_string())?;

    let kdf_config = config.kdf_config();
    let master_key =
        derive_key(password, &salt_arr, &kdf_config).map_err(|_| "KDF failed".to_string())?;

    let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
    let verify_key = derive_key(
        &hex::encode(master_key.as_slice()),
        verify_data,
        &KdfConfig {
            memory_kb: 8192,
            iterations: 1,
            parallelism: 1,
        },
    )
    .map_err(|_| "Verify failed".to_string())?;

    let expected_hash =
        hex::decode(&config.verify_hash).map_err(|_| "Invalid verify hash".to_string())?;
    Ok(solosoul_crypto::secure::secure_compare(
        verify_key.as_slice(),
        &expected_hash,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use solosoul_crypto::kdf::{derive_key, KdfConfig};

    fn sample_account_config() -> AccountConfig {
        AccountConfig {
            account_id: "acc-1".to_string(),
            name: "Test".to_string(),
            salt: base64::engine::general_purpose::STANDARD.encode(b"1234567890123456"),
            verify_hash: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            crypto_version: 1,
            password_hint: None,
            last_login_at: None,
            last_operation_at: None,
            last_operation_desc: None,
            biometric_enabled: false,
            kdf_memory_kb: None,
            kdf_iterations: None,
            kdf_parallelism: None,
        }
    }

    fn compute_verify_hash(password: &str, salt: &[u8; 16]) -> String {
        let kdf_config = KdfConfig::balanced();
        let master_key = derive_key(password, salt, &kdf_config).unwrap();
        let verify_data = b"SOLOSOUL_VAULT_VERIFY_v1";
        let verify_key = derive_key(
            &hex::encode(master_key.as_slice()),
            verify_data,
            &KdfConfig {
                memory_kb: 8192,
                iterations: 1,
                parallelism: 1,
            },
        )
        .unwrap();
        hex::encode(verify_key.as_slice())
    }

    #[test]
    fn test_verify_password_core_correct_password() {
        let salt = b"1234567890123456";
        let mut config = sample_account_config();
        config.salt = base64::engine::general_purpose::STANDARD.encode(salt);
        config.verify_hash = compute_verify_hash("secret123", salt);

        assert!(verify_password_core("secret123", &config).unwrap());
    }

    #[test]
    fn test_verify_password_core_wrong_password() {
        let salt = b"1234567890123456";
        let mut config = sample_account_config();
        config.salt = base64::engine::general_purpose::STANDARD.encode(salt);
        config.verify_hash = compute_verify_hash("secret123", salt);

        assert!(!verify_password_core("wrongpassword", &config).unwrap());
    }

    #[test]
    fn test_verify_password_core_invalid_salt() {
        let mut config = sample_account_config();
        config.salt = "not-valid-base64!!!".to_string();
        assert!(verify_password_core("secret123", &config).is_err());
    }

    #[test]
    fn test_verify_password_core_bad_salt_length() {
        let mut config = sample_account_config();
        config.salt = base64::engine::general_purpose::STANDARD.encode(b"short");
        assert!(verify_password_core("secret123", &config).is_err());
    }
}
