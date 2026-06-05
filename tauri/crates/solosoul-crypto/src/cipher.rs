use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
    Aes256Gcm, Nonce,
};
use zeroize::Zeroizing;

/// 加密后的数据格式：nonce (12 bytes) || ciphertext || tag (16 bytes)
pub struct EncryptedData {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

impl EncryptedData {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(12 + self.ciphertext.len());
        result.extend_from_slice(&self.nonce);
        result.extend_from_slice(&self.ciphertext);
        result
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CipherError> {
        if bytes.len() < 12 {
            return Err(CipherError::InvalidCiphertext);
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&bytes[..12]);
        let ciphertext = bytes[12..].to_vec();
        Ok(Self { nonce, ciphertext })
    }
}

/// AES-256-GCM 加密
pub fn encrypt(
    key: &[u8; 32],
    plaintext: &[u8],
    aad: Option<&[u8]>,
) -> Result<EncryptedData, CipherError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CipherError::InvalidKeyLength)?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let nonce_bytes: [u8; 12] = nonce
        .as_slice()
        .try_into()
        .map_err(|_| CipherError::NonceGenerationFailed)?;

    let payload = match aad {
        Some(aad_data) => Payload {
            msg: plaintext,
            aad: aad_data,
        },
        None => Payload {
            msg: plaintext,
            aad: &[],
        },
    };

    let ciphertext = cipher
        .encrypt(&nonce, payload)
        .map_err(|_| CipherError::EncryptionFailed)?;

    Ok(EncryptedData {
        nonce: nonce_bytes,
        ciphertext,
    })
}

/// AES-256-GCM 解密
pub fn decrypt(
    key: &[u8; 32],
    encrypted: &EncryptedData,
    aad: Option<&[u8]>,
) -> Result<Zeroizing<Vec<u8>>, CipherError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CipherError::InvalidKeyLength)?;

    let nonce = Nonce::from_slice(&encrypted.nonce);

    let payload = match aad {
        Some(aad_data) => Payload {
            msg: &encrypted.ciphertext,
            aad: aad_data,
        },
        None => Payload {
            msg: &encrypted.ciphertext,
            aad: &[],
        },
    };

    let plaintext = cipher
        .decrypt(nonce, payload)
        .map_err(|_| CipherError::DecryptionFailed)?;

    Ok(Zeroizing::new(plaintext))
}

/// 便捷函数：直接加密为字节数组
pub fn encrypt_to_bytes(
    key: &[u8; 32],
    plaintext: &[u8],
    aad: Option<&[u8]>,
) -> Result<Vec<u8>, CipherError> {
    encrypt(key, plaintext, aad).map(|e| e.to_bytes())
}

/// 便捷函数：直接从字节数组解密
pub fn decrypt_from_bytes(
    key: &[u8; 32],
    ciphertext: &[u8],
    aad: Option<&[u8]>,
) -> Result<Zeroizing<Vec<u8>>, CipherError> {
    let encrypted = EncryptedData::from_bytes(ciphertext)?;
    decrypt(key, &encrypted, aad)
}

#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    #[error("无效的密钥长度（需要 32 字节）")]
    InvalidKeyLength,
    #[error("Nonce 生成失败")]
    NonceGenerationFailed,
    #[error("加密失败")]
    EncryptionFailed,
    #[error("解密失败：密文可能已损坏或被篡改")]
    DecryptionFailed,
    #[error("无效的密文格式")]
    InvalidCiphertext,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [0x42u8; 32];
        let plaintext = b"Hello, SoloSoul!";
        let encrypted = encrypt(&key, plaintext, None).unwrap();
        let decrypted = decrypt(&key, &encrypted, None).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn test_encrypt_with_aad() {
        let key = [0x42u8; 32];
        let plaintext = b"Test with AAD";
        let aad = b"authenticated_data";
        let encrypted = encrypt(&key, plaintext, Some(aad)).unwrap();
        let decrypted = decrypt(&key, &encrypted, Some(aad)).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let key1 = [0x42u8; 32];
        let key2 = [0x43u8; 32];
        let encrypted = encrypt(&key1, b"hello", None).unwrap();
        assert!(decrypt(&key2, &encrypted, None).is_err());
    }

    #[test]
    fn test_decrypt_tampered_ciphertext() {
        let key = [0x42u8; 32];
        let mut encrypted = encrypt(&key, b"hello", None).unwrap();
        encrypted.ciphertext[0] ^= 0xFF;
        assert!(decrypt(&key, &encrypted, None).is_err());
    }

    #[test]
    fn test_encrypt_to_bytes_roundtrip() {
        let key = [0x42u8; 32];
        let plaintext = b"bytes roundtrip";
        let bytes = encrypt_to_bytes(&key, plaintext, None).unwrap();
        let decrypted = decrypt_from_bytes(&key, &bytes, None).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext);
    }
}
