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

/// 分块格式魔数与版本（P105：头部纳入 GCM 认证）。
///
/// - v2（当前）：`magic(4="SOLC") || version(1=0x02) || nonce(12) || chunk_count(8) || chunks`，
///   每个 chunk 的 GCM 以 `nonce || chunk_count` 作为 AAD——篡改任意头部字节即整体解密失败。
/// - v1（遗留）：`nonce(12) || chunk_count(8) || chunks`，头部不参与认证，仅作向后兼容读取。
const CHUNKED_MAGIC: [u8; 4] = [0x53, 0x4F, 0x4C, 0x43]; // "SOLC"
const CHUNKED_VERSION: u8 = 0x02;
/// 新格式完整头部长度：magic(4)+version(1)+nonce(12)+chunk_count(8)=25。
const NEW_HEADER_LEN: usize = 25;
/// 旧格式头部长度：nonce(12)+chunk_count(8)=20。
const LEGACY_HEADER_LEN: usize = 20;

/// 构造新格式每个 chunk 的 AAD（`nonce || chunk_count`，共 20 字节）。
fn chunked_aad(base_nonce: &[u8; 12], chunk_count: u64) -> [u8; 20] {
    let mut aad = [0u8; 20];
    aad[..12].copy_from_slice(base_nonce);
    aad[12..].copy_from_slice(&chunk_count.to_be_bytes());
    aad
}

/// Encrypt a large stream in chunks, writing the chunked format (v2, header-authenticated)
/// directly to `writer`.
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
        .write_all(&CHUNKED_MAGIC)
        .map_err(|e| CipherError::Io(e.to_string()))?;
    writer
        .write_all(&[CHUNKED_VERSION])
        .map_err(|e| CipherError::Io(e.to_string()))?;
    writer
        .write_all(&base_nonce_bytes)
        .map_err(|e| CipherError::Io(e.to_string()))?;
    writer
        .write_all(&total_chunks.to_be_bytes())
        .map_err(|e| CipherError::Io(e.to_string()))?;

    // P105：头部（nonce||chunk_count）作为 AAD 纳入每个 chunk 的 GCM。
    let aad = chunked_aad(&base_nonce_bytes, total_chunks);

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
            .encrypt(
                nonce,
                Payload {
                    msg: &buf[..chunk_len],
                    aad: &aad,
                },
            )
            .map_err(|_| CipherError::EncryptionFailed)?;
        writer
            .write_all(&ct)
            .map_err(|e| CipherError::Io(e.to_string()))?;
    }
    Ok(())
}

/// Decrypt a chunked file.
///
/// Auto-detects the format by leading magic:
/// - v2（当前，P105）：`SOLC(4) || version(1) || nonce(12) || chunk_count(8) || chunks`，
///   每个 chunk 以 `nonce || chunk_count` 为 AAD 认证——篡改头部即解密失败。
/// - v1（遗留）：`nonce(12) || chunk_count(8) || chunks`，头部不参与认证，仅向后兼容读取。
pub fn decrypt_chunked_from_bytes(
    key: &[u8; 32],
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CipherError> {
    if ciphertext.len() < LEGACY_HEADER_LEN {
        return Err(CipherError::InvalidCiphertext);
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CipherError::InvalidKeyLength)?;

    let (is_v2, base_nonce, chunk_count, offset) =
        if ciphertext.len() >= NEW_HEADER_LEN && ciphertext[..4] == CHUNKED_MAGIC {
            if ciphertext[4] != CHUNKED_VERSION {
                return Err(CipherError::BlobFormat(format!(
                    "不支持的分块版本: {}",
                    ciphertext[4]
                )));
            }
            let mut nonce = [0u8; 12];
            nonce.copy_from_slice(&ciphertext[5..17]);
            let count = u64::from_be_bytes(
                ciphertext[17..25]
                    .try_into()
                    .map_err(|_| CipherError::InvalidCiphertext)?,
            ) as usize;
            (true, nonce, count, NEW_HEADER_LEN)
        } else {
            let mut nonce = [0u8; 12];
            nonce.copy_from_slice(&ciphertext[..12]);
            let count = u64::from_be_bytes(
                ciphertext[12..20]
                    .try_into()
                    .map_err(|_| CipherError::InvalidCiphertext)?,
            ) as usize;
            (false, nonce, count, LEGACY_HEADER_LEN)
        };

    // P105：v2 格式的 AAD = nonce || chunk_count，篡改头部任一字节即整体认证失败。
    let aad = chunked_aad(&base_nonce, chunk_count as u64);

    let mut result = Vec::new();
    let mut cursor = offset;

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
            ciphertext.len() - cursor
        } else {
            CHUNK_SIZE + 16
        };

        if cursor + expected_chunk_ct_len > ciphertext.len() {
            return Err(CipherError::InvalidCiphertext);
        }
        let chunk_ct = &ciphertext[cursor..cursor + expected_chunk_ct_len];
        let payload = if is_v2 {
            Payload {
                msg: chunk_ct,
                aad: &aad,
            }
        } else {
            Payload {
                msg: chunk_ct,
                aad: &[],
            }
        };
        let pt = cipher
            .decrypt(nonce, payload)
            .map_err(|_| CipherError::DecryptionFailed)?;
        result.extend_from_slice(&pt);
        cursor += expected_chunk_ct_len;
    }

    Ok(Zeroizing::new(result))
}

/// Decrypt a chunked stream, writing decrypted plaintext directly to `writer`.
///
/// Auto-detects the format by leading magic (v2 `SOLC` / v1 legacy), see
/// [`decrypt_chunked_from_bytes`].
pub fn decrypt_chunked_stream<R: Read, W: Write>(
    key: &[u8; 32],
    reader: &mut R,
    writer: &mut W,
) -> Result<(), CipherError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CipherError::InvalidKeyLength)?;

    // 先读 4 字节判断格式。
    let mut magic = [0u8; 4];
    reader
        .read_exact(&mut magic)
        .map_err(|_| CipherError::InvalidCiphertext)?;
    let is_v2 = magic == CHUNKED_MAGIC;

    let (base_nonce, chunk_count, aad): ([u8; 12], usize, Option<[u8; 20]>) = if is_v2 {
        let mut rest = [0u8; 21]; // version(1) + nonce(12) + chunk_count(8)
        reader
            .read_exact(&mut rest)
            .map_err(|_| CipherError::InvalidCiphertext)?;
        if rest[0] != CHUNKED_VERSION {
            return Err(CipherError::BlobFormat(format!(
                "不支持的分块版本: {}",
                rest[0]
            )));
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&rest[1..13]);
        let count = u64::from_be_bytes(
            rest[13..21]
                .try_into()
                .map_err(|_| CipherError::InvalidCiphertext)?,
        ) as usize;
        let aad = chunked_aad(&nonce, count as u64);
        (nonce, count, Some(aad))
    } else {
        // 遗留格式：已读的 4 字节是 nonce 的前 4 字节。
        let mut nonce = [0u8; 12];
        nonce[..4].copy_from_slice(&magic);
        reader
            .read_exact(&mut nonce[4..])
            .map_err(|_| CipherError::InvalidCiphertext)?;
        let mut count_buf = [0u8; 8];
        reader
            .read_exact(&mut count_buf)
            .map_err(|_| CipherError::InvalidCiphertext)?;
        let count = u64::from_be_bytes(count_buf) as usize;
        (nonce, count, None)
    };

    let mut chunk_buf = vec![0u8; CHUNK_SIZE + 16]; // plaintext + auth tag
    for i in 0..chunk_count {
        let mut nonce_bytes = base_nonce;
        let idx_bytes = (i as u64).to_be_bytes();
        for j in 0..8 {
            nonce_bytes[4 + j] ^= idx_bytes[j];
        }
        let nonce = Nonce::from_slice(&nonce_bytes);

        // For all but the last chunk, ciphertext is exactly CHUNK_SIZE + 16.
        // For the last chunk, it's whatever remains.
        let expected = if i == chunk_count - 1 {
            // Read remaining bytes
            let mut remaining = Vec::new();
            reader
                .read_to_end(&mut remaining)
                .map_err(|e| CipherError::Io(e.to_string()))?;
            if remaining.is_empty() {
                return Err(CipherError::InvalidCiphertext);
            }
            let payload = match &aad {
                Some(a) => Payload {
                    msg: remaining.as_slice(),
                    aad: a,
                },
                None => Payload {
                    msg: remaining.as_slice(),
                    aad: &[],
                },
            };
            let pt = cipher
                .decrypt(nonce, payload)
                .map_err(|_| CipherError::DecryptionFailed)?;
            writer
                .write_all(&pt)
                .map_err(|e| CipherError::Io(e.to_string()))?;
            continue;
        } else {
            CHUNK_SIZE + 16
        };

        reader
            .read_exact(&mut chunk_buf[..expected])
            .map_err(|_| CipherError::InvalidCiphertext)?;
        let payload = match &aad {
            Some(a) => Payload {
                msg: &chunk_buf[..expected],
                aad: a,
            },
            None => Payload {
                msg: &chunk_buf[..expected],
                aad: &[],
            },
        };
        let pt = cipher
            .decrypt(nonce, payload)
            .map_err(|_| CipherError::DecryptionFailed)?;
        writer
            .write_all(&pt)
            .map_err(|e| CipherError::Io(e.to_string()))?;
    }

    Ok(())
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
    #[error("Blob 格式错误: {0}")]
    BlobFormat(String),
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

    // ── P105 分块格式头部认证 ──────────────────────────────────────────

    #[test]
    fn test_chunked_v2_roundtrip_stream() {
        let key = [0x52u8; 32];
        let plaintext: Vec<u8> = (0..CHUNK_SIZE * 5 + 777).map(|i| (i % 251) as u8).collect();
        let mut enc = Vec::new();
        encrypt_chunked_stream(
            &key,
            plaintext.len() as u64,
            &mut plaintext.as_slice(),
            &mut enc,
        )
        .unwrap();
        let mut dec = Vec::new();
        decrypt_chunked_stream(&key, &mut enc.as_slice(), &mut dec).unwrap();
        assert_eq!(dec, plaintext);
    }

    #[test]
    fn test_chunked_header_tamper_detected_stream() {
        let key = [0x55u8; 32];
        let plaintext: Vec<u8> = (0..CHUNK_SIZE * 2 + 100).map(|i| (i % 251) as u8).collect();
        let mut enc = Vec::new();
        encrypt_chunked_stream(
            &key,
            plaintext.len() as u64,
            &mut plaintext.as_slice(),
            &mut enc,
        )
        .unwrap();
        // 篡改 chunk_count 头部（偏移 17..25）。
        enc[NEW_HEADER_LEN - 1] ^= 0x01;
        let mut dec = Vec::new();
        assert!(decrypt_chunked_stream(&key, &mut enc.as_slice(), &mut dec).is_err());
    }

    /// 手工构造遗留 v1 格式（无魔数、头部不认证），验证向后兼容解密。
    #[test]
    fn test_chunked_v1_legacy_backward_compat() {
        let key = [0x56u8; 32];
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let base_nonce = [0xAAu8; 12];
        let plaintext: Vec<u8> = (0..CHUNK_SIZE * 2 + 55).map(|i| (i % 251) as u8).collect();
        let chunk_count = plaintext.len().div_ceil(CHUNK_SIZE) as u64;

        let mut enc = Vec::new();
        enc.extend_from_slice(&base_nonce);
        enc.extend_from_slice(&chunk_count.to_be_bytes());
        for i in 0..chunk_count as usize {
            let start = i * CHUNK_SIZE;
            let end = ((i + 1) * CHUNK_SIZE).min(plaintext.len());
            let mut nonce_bytes = base_nonce;
            let idx_bytes = (i as u64).to_be_bytes();
            for j in 0..8 {
                nonce_bytes[4 + j] ^= idx_bytes[j];
            }
            let ct = cipher
                .encrypt(Nonce::from_slice(&nonce_bytes), &plaintext[start..end])
                .unwrap();
            enc.extend_from_slice(&ct);
        }

        let dec = decrypt_chunked_from_bytes(&key, &enc).unwrap();
        assert_eq!(dec.as_slice(), plaintext.as_slice());

        let mut dec2 = Vec::new();
        decrypt_chunked_stream(&key, &mut enc.as_slice(), &mut dec2).unwrap();
        assert_eq!(dec2, plaintext);
    }
}
