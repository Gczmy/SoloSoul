//! HKDF-SHA256 密钥扩展
//!
//! 用于从主导出密钥派生子密钥（如 preferences.enc 的独立加密密钥）。

use hkdf::Hkdf;
use sha2::Sha256;

/// 使用 HKDF-SHA256 从主密钥派生 32 字节子密钥。
///
/// # 参数
/// - `master_key`: 主导出密钥（如 Argon2id 派生的 32 字节密钥）
/// - `salt`: salt，可与主密钥派生时使用的 salt 相同
/// - `info`: 上下文信息字符串（如 `b"solosoul:preferences:v1"`），确保不同用途的密钥在密码学上独立
///
/// # 返回
/// 32 字节子密钥
pub fn derive_hkdf_key(
    master_key: &[u8; 32],
    salt: &[u8],
    info: &[u8],
) -> Result<[u8; 32], String> {
    let hk = Hkdf::<Sha256>::new(Some(salt), master_key);
    let mut okm = [0u8; 32];
    hk.expand(info, &mut okm)
        .map_err(|e| format!("HKDF expand failed: {:?}", e))?;
    Ok(okm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hkdf_deterministic() {
        let master = [0xABu8; 32];
        let salt = b"test_salt_12345678";
        let info = b"solosoul:test:v1";
        let key1 = derive_hkdf_key(&master, salt, info).unwrap();
        let key2 = derive_hkdf_key(&master, salt, info).unwrap();
        assert_eq!(key1.len(), 32);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_hkdf_different_info_produces_different_keys() {
        let master = [0xABu8; 32];
        let salt = b"test_salt_12345678";
        let key_read = derive_hkdf_key(&master, salt, b"solosoul:read:v1").unwrap();
        let key_write = derive_hkdf_key(&master, salt, b"solosoul:write:v1").unwrap();
        assert_ne!(key_read, key_write);
    }

    #[test]
    fn test_hkdf_different_salt_produces_different_keys() {
        let master = [0xABu8; 32];
        let info = b"solosoul:test:v1";
        let key1 = derive_hkdf_key(&master, b"salt_a_1234567890", info).unwrap();
        let key2 = derive_hkdf_key(&master, b"salt_b_1234567890", info).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_hkdf_different_master_produces_different_keys() {
        let master_a = [0xAAu8; 32];
        let master_b = [0xBBu8; 32];
        let salt = b"test_salt_12345678";
        let key_a = derive_hkdf_key(&master_a, salt, b"info").unwrap();
        let key_b = derive_hkdf_key(&master_b, salt, b"info").unwrap();
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn test_hkdf_output_is_32_bytes() {
        let master = [0xABu8; 32];
        let key = derive_hkdf_key(&master, b"salt", b"info").unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_hkdf_empty_info() {
        let master = [0xABu8; 32];
        let key = derive_hkdf_key(&master, b"salt", b"").unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_hkdf_empty_salt() {
        let master = [0xABu8; 32];
        let key = derive_hkdf_key(&master, b"", b"info").unwrap();
        assert_eq!(key.len(), 32);
    }
}
