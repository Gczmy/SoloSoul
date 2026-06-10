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
