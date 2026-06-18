use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
    Aes256Gcm, Nonce,
};
use std::io::{Read, Write};
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

// ── Chunked encryption for large files (>10MB attachments) ──────────

const CHUNK_SIZE: usize = 64 * 1024; // 64 KB

/// Encrypt a large file in chunks.
/// Format: nonce(12) || chunk_count(8, big-endian u64) || chunk1_ct+tag || chunk2_ct+tag || ...
pub fn encrypt_chunked_to_bytes(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CipherError::InvalidKeyLength)?;
    let base_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let base_nonce_bytes: [u8; 12] = base_nonce
        .as_slice()
        .try_into()
        .map_err(|_| CipherError::NonceGenerationFailed)?;

    let total_chunks = plaintext.len().div_ceil(CHUNK_SIZE);
    if total_chunks > u64::MAX as usize {
        return Err(CipherError::EncryptionFailed);
    }

    let mut result = Vec::with_capacity(12 + 8 + plaintext.len() + total_chunks * 16);
    result.extend_from_slice(&base_nonce_bytes);
    result.extend_from_slice(&(total_chunks as u64).to_be_bytes());

    for i in 0..total_chunks {
        let start = i * CHUNK_SIZE;
        let end = ((i + 1) * CHUNK_SIZE).min(plaintext.len());
        let chunk = &plaintext[start..end];

        let mut nonce_bytes = base_nonce_bytes;
        let idx_bytes = (i as u64).to_be_bytes();
        for j in 0..8 {
            nonce_bytes[4 + j] ^= idx_bytes[j];
        }
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ct = cipher
            .encrypt(nonce, chunk)
            .map_err(|_| CipherError::EncryptionFailed)?;
        result.extend_from_slice(&ct);
    }

    Ok(result)
}

/// Encrypt a large stream in chunks, writing the same chunked format directly to `writer`.
/// `total_size` must be the exact number of plaintext bytes that `reader` will yield.
pub fn encrypt_chunked_stream<R: Read, W: Write>(
    key: &[u8; 32],
    total_size: u64,
    reader: &mut R,
    writer: &mut W,
) -> Result<(), CipherError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CipherError::InvalidKeyLength)?;
    let base_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let base_nonce_bytes: [u8; 12] = base_nonce
        .as_slice()
        .try_into()
        .map_err(|_| CipherError::NonceGenerationFailed)?;

    let total_chunks = total_size.div_ceil(CHUNK_SIZE as u64);
    writer
        .write_all(&base_nonce_bytes)
        .map_err(|e| CipherError::Io(e.to_string()))?;
    writer
        .write_all(&total_chunks.to_be_bytes())
        .map_err(|e| CipherError::Io(e.to_string()))?;

    let mut buf = vec![0u8; CHUNK_SIZE];
    for i in 0..total_chunks as usize {
        let chunk_len = if i == total_chunks as usize - 1 {
            (total_size as usize) - (i * CHUNK_SIZE)
        } else {
            CHUNK_SIZE
        };
        reader
            .read_exact(&mut buf[..chunk_len])
            .map_err(|e| CipherError::Io(e.to_string()))?;
        let mut nonce_bytes = base_nonce_bytes;
        let idx_bytes = (i as u64).to_be_bytes();
        for j in 0..8 {
            nonce_bytes[4 + j] ^= idx_bytes[j];
        }
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = cipher
            .encrypt(nonce, &buf[..chunk_len])
            .map_err(|_| CipherError::EncryptionFailed)?;
        writer
            .write_all(&ct)
            .map_err(|e| CipherError::Io(e.to_string()))?;
    }
    Ok(())
}

/// Decrypt a chunked file.
/// Expects format: nonce(12) || chunk_count(8) || chunk_ct+tag ...
pub fn decrypt_chunked_from_bytes(
    key: &[u8; 32],
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CipherError> {
    if ciphertext.len() < 20 {
        return Err(CipherError::InvalidCiphertext);
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CipherError::InvalidKeyLength)?;

    let mut base_nonce = [0u8; 12];
    base_nonce.copy_from_slice(&ciphertext[..12]);
    let chunk_count = u64::from_be_bytes(
        ciphertext[12..20]
            .try_into()
            .map_err(|_| CipherError::InvalidCiphertext)?,
    ) as usize;

    let mut result = Vec::new();
    let mut offset = 20;

    for i in 0..chunk_count {
        let mut nonce_bytes = base_nonce;
        let idx_bytes = (i as u64).to_be_bytes();
        for j in 0..8 {
            nonce_bytes[4 + j] ^= idx_bytes[j];
        }
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Each chunk is at least 16 bytes (auth tag). For all but the last,
        // plaintext was exactly CHUNK_SIZE, so ciphertext is CHUNK_SIZE + 16.
        let expected_chunk_ct_len = if i == chunk_count - 1 {
            ciphertext.len() - offset
        } else {
            CHUNK_SIZE + 16
        };

        if offset + expected_chunk_ct_len > ciphertext.len() {
            return Err(CipherError::InvalidCiphertext);
        }
        let chunk_ct = &ciphertext[offset..offset + expected_chunk_ct_len];
        let pt = cipher
            .decrypt(nonce, chunk_ct)
            .map_err(|_| CipherError::DecryptionFailed)?;
        result.extend_from_slice(&pt);
        offset += expected_chunk_ct_len;
    }

    Ok(Zeroizing::new(result))
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
    #[error("IO 错误: {0}")]
    Io(String),
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
