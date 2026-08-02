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
    /// 开发模式：低开销，适合本地开发（8 MiB / 2 iter / 4 par）
    pub fn development() -> Self {
        Self {
            memory_kb: 8 * 1024,
            iterations: 2,
            parallelism: 4,
        }
    }

    /// 平衡模式：旧有账户的默认参数，兼顾安全与性能（16 MiB / 3 iter / 4 par）
    /// 用于向后兼容未存储 KDF 参数的旧账户。
    pub fn balanced() -> Self {
        Self {
            memory_kb: 16 * 1024,
            iterations: 3,
            parallelism: 4,
        }
    }

    /// 生产模式：OWASP 推荐参数（64 MiB / 3 iter / 4 par）
    pub fn production() -> Self {
        Self {
            memory_kb: 64 * 1024,
            iterations: 3,
            parallelism: 4,
        }
    }

    /// 根据构建类型与环境变量选择 KDF 参数（P003）：
    /// - `SOLOSOUL_SECURE=1` → 强制 production (64 MiB / 3 iter)
    /// - release 构建（未设置 SOLOSOUL_SECURE）→ production，OWASP 推荐参数。
    ///   此前默认 development 而 `SOLOSOUL_SECURE` 在终端设备上从未被设置，
    ///   导致所有真实用户都以开发档参数派生主密钥（P003）。
    /// - debug 构建 → development (8 MiB / 2 iter)，保证本地开发/测试快速。
    pub fn from_env() -> Self {
        if std::env::var("SOLOSOUL_SECURE").as_deref() == Ok("1") {
            return Self::production();
        }
        if cfg!(debug_assertions) {
            Self::development()
        } else {
            Self::production()
        }
    }
}

impl Default for KdfConfig {
    fn default() -> Self {
        Self::from_env()
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
