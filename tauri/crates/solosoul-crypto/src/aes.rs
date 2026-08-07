//! AES-256-GCM blob encryption (SOLO format)
//!
//! Provides authenticated encryption with AES-256-GCM in SOLO blob format:
//! - v2: Magic(4) + Version(1) + Nonce(12) + Ciphertext+Tag(16)
//! - v3: Magic(4) + Version(1) + Header(16) + [Nonce(12) + Ciphertext+Tag(16)]*N

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use std::io::{Read, Seek, SeekFrom, Write};
use zeroize::Zeroizing;

use crate::cipher::CipherError;

pub const NONCE_SIZE: usize = 12;
pub const KEY_SIZE: usize = 32;
pub const TAG_SIZE: usize = 16;

pub const BLOB_MAGIC: [u8; 4] = [0x53, 0x4F, 0x4C, 0x4F]; // "SOLO"
pub const BLOB_VERSION: u8 = 0x02;
pub const BLOB_VERSION_V3: u8 = 0x03;
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// P106：校验 v3 分块头部字段的自洽性（全部由输入控制，杜绝巨额分配 DoS）。
///
/// 规则：`chunk_size != 0`；`original_size` 必须落在分块网格内，即
/// `(count-1)*chunk_size < original_size <= count*chunk_size`
/// （`count == 0` 时 `original_size` 必须为 0）。
fn validate_chunked_header(
    original_size: u64,
    chunk_size: usize,
    chunk_count: u32,
) -> Result<(), CipherError> {
    if chunk_size == 0 {
        return Err(fmt_err("无效的分块大小（0）"));
    }
    let chunks = chunk_count as u64;
    let chunk_sz = chunk_size as u64;
    if chunks > 0 {
        let max_plain = chunks.saturating_mul(chunk_sz);
        let min_plain = (chunks - 1).saturating_mul(chunk_sz);
        if original_size > max_plain || original_size <= min_plain {
            return Err(fmt_err("头部声明的明文大小与分块参数不符"));
        }
    } else if original_size != 0 {
        return Err(fmt_err("头部声明的明文大小与分块参数不符"));
    }
    Ok(())
}

fn key_err(e: impl std::fmt::Display) -> CipherError {
    CipherError::BlobFormat(format!("密钥无效: {}", e))
}
fn enc_err(e: impl std::fmt::Display) -> CipherError {
    CipherError::BlobFormat(format!("加密失败: {}", e))
}
fn fmt_err(msg: impl Into<String>) -> CipherError {
    CipherError::BlobFormat(msg.into())
}

/// Encrypt data using AES-256-GCM with SOLO blob format (v2)
pub fn encrypt_blob(key: &[u8; 32], plaintext: &[u8]) -> Result<Zeroizing<Vec<u8>>, CipherError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(key_err)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher.encrypt(&nonce, plaintext).map_err(enc_err)?;

    let mut blob = Vec::with_capacity(4 + 1 + NONCE_SIZE + ciphertext.len());
    blob.extend_from_slice(&BLOB_MAGIC);
    blob.push(BLOB_VERSION);
    blob.extend_from_slice(nonce.as_slice());
    blob.extend_from_slice(&ciphertext);
    Ok(Zeroizing::new(blob))
}

/// Decrypt SOLO blob (v2) format
pub fn decrypt_blob(key: &[u8; 32], blob: &[u8]) -> Result<Zeroizing<Vec<u8>>, CipherError> {
    if blob.len() < 33 {
        return Err(fmt_err("密文 Blob 过短"));
    }
    if blob[0..4] != BLOB_MAGIC {
        return Err(fmt_err("无效的 Blob 魔数"));
    }
    if blob[4] != BLOB_VERSION {
        return Err(fmt_err(format!("不支持的 Blob 版本: {}", blob[4])));
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(key_err)?;
    let nonce = Nonce::from_slice(&blob[5..17]);
    let plaintext = cipher.decrypt(nonce, &blob[17..]).map_err(enc_err)?;
    Ok(Zeroizing::new(plaintext))
}

/// Encrypt a stream using chunked AES-256-GCM (SOLO blob v3).
/// Reads from `reader`, writes the v3 blob to `writer`, processing at most
/// `chunk_size` bytes at a time so the whole file never has to fit in memory.
pub fn encrypt_chunked_stream<R: Read + Seek, W: Write>(
    key: &[u8; 32],
    reader: &mut R,
    writer: &mut W,
    chunk_size: usize,
) -> Result<(), CipherError> {
    let chunk_size = if chunk_size == 0 {
        DEFAULT_CHUNK_SIZE
    } else {
        chunk_size
    };
    let original_size = reader
        .seek(SeekFrom::End(0))
        .map_err(|e| CipherError::Io(format!("Seek 失败: {}", e)))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|e| CipherError::Io(format!("Seek 失败: {}", e)))?;
    let chunk_count = original_size.div_ceil(chunk_size as u64) as u32;

    writer
        .write_all(&BLOB_MAGIC)
        .map_err(|e| CipherError::Io(format!("写入失败: {}", e)))?;
    writer
        .write_all(&[BLOB_VERSION_V3])
        .map_err(|e| CipherError::Io(format!("写入失败: {}", e)))?;
    writer
        .write_all(&original_size.to_be_bytes())
        .map_err(|e| CipherError::Io(format!("写入失败: {}", e)))?;
    writer
        .write_all(&(chunk_size as u32).to_be_bytes())
        .map_err(|e| CipherError::Io(format!("写入失败: {}", e)))?;
    writer
        .write_all(&chunk_count.to_be_bytes())
        .map_err(|e| CipherError::Io(format!("写入失败: {}", e)))?;

    let cipher = Aes256Gcm::new_from_slice(key).map_err(key_err)?;
    let mut buffer = vec![0u8; chunk_size];

    for i in 0..chunk_count as usize {
        let is_last = i == chunk_count as usize - 1;
        let to_read = if is_last {
            (original_size - (i as u64 * chunk_size as u64)) as usize
        } else {
            chunk_size
        };
        reader
            .read_exact(&mut buffer[..to_read])
            .map_err(|e| CipherError::Io(format!("读取失败: {}", e)))?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, &buffer[..to_read])
            .map_err(|e| enc_err(format!("分块 {} 加密失败: {}", i, e)))?;
        writer
            .write_all(nonce.as_slice())
            .map_err(|e| CipherError::Io(format!("写入失败: {}", e)))?;
        writer
            .write_all(&ciphertext)
            .map_err(|e| CipherError::Io(format!("写入失败: {}", e)))?;
    }
    Ok(())
}

/// Decrypt a SOLO blob stream (v2 or v3) to a writer.
pub fn decrypt_chunked_stream<R: Read, W: Write>(
    key: &[u8; 32],
    reader: &mut R,
    writer: &mut W,
) -> Result<(), CipherError> {
    let mut header = [0u8; 21];
    reader
        .read_exact(&mut header)
        .map_err(|e| CipherError::Io(format!("读取头部失败: {}", e)))?;

    if header[0..4] != BLOB_MAGIC {
        return Err(fmt_err("无效的 Blob 魔数"));
    }

    // v2 blob: read the rest into memory and decrypt as one block.
    if header[4] == BLOB_VERSION {
        let mut blob = header.to_vec();
        reader
            .read_to_end(&mut blob)
            .map_err(|e| CipherError::Io(format!("读取失败: {}", e)))?;
        let plaintext = decrypt_blob(key, &blob)?;
        writer
            .write_all(&plaintext)
            .map_err(|e| CipherError::Io(format!("写入失败: {}", e)))?;
        return Ok(());
    }

    if header[4] != BLOB_VERSION_V3 {
        return Err(fmt_err(format!("不支持的 Blob 版本: {}", header[4])));
    }

    let mut orig_size = [0u8; 8];
    orig_size.copy_from_slice(&header[5..13]);
    let original_size = u64::from_be_bytes(orig_size);
    let mut chunk_size_raw = [0u8; 4];
    chunk_size_raw.copy_from_slice(&header[13..17]);
    let chunk_size = u32::from_be_bytes(chunk_size_raw) as usize;
    let mut chunk_count_raw = [0u8; 4];
    chunk_count_raw.copy_from_slice(&header[17..21]);
    let chunk_count = u32::from_be_bytes(chunk_count_raw);

    // P106：流式版与 blob 版同类的头部校验（chunk_size 非 0 + 分块网格自洽）；
    // 每块密文改为按实际读取增量扩展，而非按头部声明的 chunk_size 预分配，
    // 防止攻击者以巨值头字段触发巨额分配（若流中并无对应数据，read_exact
    // 会在分配显著增长前即失败）。
    validate_chunked_header(original_size, chunk_size, chunk_count)?;

    let cipher = Aes256Gcm::new_from_slice(key).map_err(key_err)?;
    let mut nonce = [0u8; NONCE_SIZE];

    for i in 0..chunk_count as usize {
        reader
            .read_exact(&mut nonce)
            .map_err(|e| CipherError::Io(format!("读取 Nonce 失败: {}", e)))?;
        let expected_plain = if i == chunk_count as usize - 1 {
            let consumed = (i as u64).checked_mul(chunk_size as u64);
            match consumed {
                Some(c) if c <= original_size => (original_size - c) as usize,
                _ => return Err(fmt_err("头部声明的明文大小与分块参数不符")),
            }
        } else {
            chunk_size
        };
        let expected_cipher = expected_plain + TAG_SIZE;
        // P106：增量读取，避免按攻击者控制的 expected_cipher 巨额预分配。
        let mut ciphertext = Vec::new();
        let mut remaining = expected_cipher;
        let mut buf = [0u8; 64 * 1024];
        while remaining > 0 {
            let n = remaining.min(buf.len());
            reader
                .read_exact(&mut buf[..n])
                .map_err(|e| CipherError::Io(format!("读取密文失败: {}", e)))?;
            ciphertext.extend_from_slice(&buf[..n]);
            remaining -= n;
        }
        let decrypted = cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_slice())
            .map_err(|e| enc_err(format!("分块 {} 解密失败: {}", i, e)))?;
        writer
            .write_all(&decrypted)
            .map_err(|e| CipherError::Io(format!("写入失败: {}", e)))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_blob() {
        let key = [0u8; 32];
        let plaintext = b"Hello, SoloSoul!";
        let blob = encrypt_blob(&key, plaintext).unwrap();
        let decrypted = decrypt_blob(&key, &blob).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt_empty() {
        let key = [0u8; 32];
        let blob = encrypt_blob(&key, b"").unwrap();
        let decrypted = decrypt_blob(&key, &blob).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_blob_format() {
        let key = [0u8; 32];
        let blob = encrypt_blob(&key, b"test").unwrap();
        assert_eq!(&blob[0..4], &BLOB_MAGIC);
        assert_eq!(blob[4], BLOB_VERSION);
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let key1 = [0u8; 32];
        let key2 = [1u8; 32];
        let blob = encrypt_blob(&key1, b"secret").unwrap();
        assert!(decrypt_blob(&key2, &blob).is_err());
    }

    #[test]
    fn test_chunked_stream_roundtrip() {
        let key = [0x42u8; 32];
        let data = vec![0xABu8; 5 * 1024 * 1024];
        let mut encrypted = Vec::new();
        encrypt_chunked_stream(
            &key,
            &mut std::io::Cursor::new(&data),
            &mut encrypted,
            1024 * 1024,
        )
        .unwrap();

        let mut decrypted = Vec::new();
        decrypt_chunked_stream(&key, &mut encrypted.as_slice(), &mut decrypted).unwrap();
        assert_eq!(decrypted.as_slice(), data.as_slice());
    }

    #[test]
    fn test_chunked_stream_roundtrip_small_file() {
        let key = [0x42u8; 32];
        let data = b"tiny".to_vec();
        let mut encrypted = Vec::new();
        encrypt_chunked_stream(
            &key,
            &mut std::io::Cursor::new(&data),
            &mut encrypted,
            1024 * 1024,
        )
        .unwrap();

        let mut decrypted = Vec::new();
        decrypt_chunked_stream(&key, &mut encrypted.as_slice(), &mut decrypted).unwrap();
        assert_eq!(decrypted.as_slice(), data.as_slice());
    }

    // ── P106 头部驱动巨额分配 DoS 防护 ────────────────────────────────

    /// 流式版：chunk_size = 0 直接拒绝。
    #[test]
    fn test_decrypt_chunked_stream_rejects_zero_chunk_size() {
        let key = [0x42u8; 32];
        let data = vec![0xABu8; 5000];
        let mut blob = Vec::new();
        encrypt_chunked_stream(&key, &mut std::io::Cursor::new(&data), &mut blob, 1024).unwrap();
        blob[13..17].copy_from_slice(&0u32.to_be_bytes());
        let mut out = Vec::new();
        assert!(decrypt_chunked_stream(&key, &mut blob.as_slice(), &mut out).is_err());
    }

    /// 流式版：chunk_count = 0 但 original_size 非零（自相矛盾）——增量读取在
    /// 首个 read_exact 即失败，不得分配巨量内存。
    #[test]
    fn test_decrypt_chunked_stream_rejects_inconsistent_header() {
        let key = [0x42u8; 32];
        let data = vec![0xABu8; 5000];
        let mut blob = Vec::new();
        encrypt_chunked_stream(&key, &mut std::io::Cursor::new(&data), &mut blob, 1024).unwrap();
        blob[17..21].copy_from_slice(&0u32.to_be_bytes()); // chunk_count = 0
        let mut out = Vec::new();
        assert!(decrypt_chunked_stream(&key, &mut blob.as_slice(), &mut out).is_err());
    }
}
