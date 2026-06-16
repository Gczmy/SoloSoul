use argon2::{self, Algorithm, Argon2, Params, Version};
use zeroize::Zeroizing;

/// KDF 参数配置
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KdfConfig {
    pub memory_kb: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl KdfConfig {
    pub fn development() -> Self {
        Self {
            memory_kb: 8 * 1024,
            iterations: 2,
            parallelism: 4,
        }
    }

    pub fn production() -> Self {
        Self {
            memory_kb: 64 * 1024,
            iterations: 3,
            parallelism: 4,
        }
    }

    pub fn balanced() -> Self {
        Self {
            memory_kb: 16 * 1024,
            iterations: 3,
            parallelism: 4,
        }
    }
}

impl Default for KdfConfig {
    fn default() -> Self {
        Self::development()
    }
}

/// 使用 Argon2id 派生 256-bit 密钥
pub fn derive_key(
    password: &str,
    salt: &[u8],
    config: &KdfConfig,
) -> Result<Zeroizing<Vec<u8>>, KdfError> {
    if salt.len() < 16 {
        return Err(KdfError::InvalidSaltLength(salt.len()));
    }

    let params = Params::new(
        config.memory_kb,
        config.iterations,
        config.parallelism,
        Some(32),
    )
    .map_err(KdfError::InvalidParams)?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut output = Zeroizing::new(vec![0u8; 32]);
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut output)
        .map_err(|e| KdfError::HashFailed(e.to_string()))?;

    Ok(output)
}

/// 生成随机 Salt（直接使用操作系统 CSPRNG）
pub fn generate_salt() -> [u8; 16] {
    use rand::rngs::OsRng;
    use rand::RngCore;
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}

#[derive(Debug, thiserror::Error)]
pub enum KdfError {
    #[error("Invalid salt length: {0} (need at least 16 bytes)")]
    InvalidSaltLength(usize),
    #[error("Invalid KDF params: {0}")]
    InvalidParams(argon2::Error),
    #[error("Hash computation failed: {0}")]
    HashFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_deterministic() {
        let password = "test_password_123";
        let salt = b"1234567890123456";
        let config = KdfConfig::development();
        let key1 = derive_key(password, salt, &config).unwrap();
        let key2 = derive_key(password, salt, &config).unwrap();
        assert_eq!(key1.len(), 32);
        assert_eq!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    fn test_derive_key_different_salts() {
        let password = "test_password_123";
        let config = KdfConfig::development();
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        let key1 = derive_key(password, &salt1, &config).unwrap();
        let key2 = derive_key(password, &salt2, &config).unwrap();
        assert_ne!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    fn test_invalid_salt_length() {
        let salt = b"short";
        let config = KdfConfig::development();
        assert!(derive_key("test", salt, &config).is_err());
    }
}
