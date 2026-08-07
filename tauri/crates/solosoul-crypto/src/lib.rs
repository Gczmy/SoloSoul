//! SoloSoul 密码学核心库
//!
//! 提供：
//! - Argon2id 密钥派生
//! - AES-256-GCM 加密/解密（SOLO blob v2/v3 格式）
//! - 流式文件加解密
//! - 安全内存管理（自动擦除）

pub mod aes;
pub mod cipher;
pub mod hkdf_ext;
pub mod kdf;
pub mod secure;

pub use aes::{decrypt_blob, encrypt_blob};
pub use cipher::{
    decrypt, decrypt_from_bytes, encrypt, encrypt_to_bytes, CipherError, EncryptedData,
};
pub use kdf::{derive_key, generate_salt, KdfConfig, KdfError};
pub use secure::secure_compare;
