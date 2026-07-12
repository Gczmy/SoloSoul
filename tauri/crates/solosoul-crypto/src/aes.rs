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

/// Encrypt data using chunked AES-256-GCM (SOLO blob v3)
pub fn encrypt_chunked_blob(
    key: &[u8; 32],
    plaintext: &[u8],
    chunk_size: usize,
) -> Result<Vec<u8>, CipherError> {
    let chunk_size = if chunk_size == 0 {
        DEFAULT_CHUNK_SIZE
    } else {
        chunk_size
    };
    let original_size = plaintext.len() as u64;
    let chunk_count = original_size.div_ceil(chunk_size as u64) as u32;

    let mut blob = Vec::new();
    blob.extend_from_slice(&BLOB_MAGIC);
    blob.push(BLOB_VERSION_V3);
    blob.extend_from_slice(&original_size.to_be_bytes());
    blob.extend_from_slice(&(chunk_size as u32).to_be_bytes());
    blob.extend_from_slice(&chunk_count.to_be_bytes());

    let cipher = Aes256Gcm::new_from_slice(key).map_err(key_err)?;

    for i in 0..chunk_count as usize {
        let start = i * chunk_size;
        let end = (start + chunk_size).min(plaintext.len());
        let chunk = &plaintext[start..end];

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, chunk)
            .map_err(|e| enc_err(format!("分块 {} 加密失败: {}", i, e)))?;

        blob.extend_from_slice(nonce.as_slice());
        blob.extend_from_slice(&ciphertext);
    }
    Ok(blob)
}

/// Decrypt v3 chunked blob
pub fn decrypt_chunked_blob(
    key: &[u8; 32],
    blob: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CipherError> {
    if blob.len() < 21 {
        return Err(fmt_err("v3 头部过短"));
    }
    if blob[0..4] != BLOB_MAGIC || blob[4] != BLOB_VERSION_V3 {
        return Err(fmt_err("无效的 v3 密文"));
    }

    let original_size = u64::from_be_bytes(
        blob[5..13]
            .try_into()
            .expect("v3 头部长度已校验为 >= 21，切片长度必然为 8"),
    );
    let chunk_size = u32::from_be_bytes(
        blob[13..17]
            .try_into()
            .expect("v3 头部长度已校验为 >= 21，切片长度必然为 4"),
    ) as usize;
    let chunk_count = u32::from_be_bytes(
        blob[17..21]
            .try_into()
            .expect("v3 头部长度已校验为 >= 21，切片长度必然为 4"),
    );

    let cipher = Aes256Gcm::new_from_slice(key).map_err(key_err)?;
    let mut plaintext = Vec::with_capacity(original_size as usize);
    let mut offset = 21;

    for i in 0..chunk_count as usize {
        let nonce = Nonce::from_slice(&blob[offset..offset + NONCE_SIZE]);
        offset += NONCE_SIZE;

        let is_last = i == chunk_count as usize - 1;
        let expected_plain = if is_last {
            original_size as usize - plaintext.len()
        } else {
            chunk_size
        };
        let expected_cipher = expected_plain + TAG_SIZE;

        if offset + expected_cipher > blob.len() {
            return Err(fmt_err(format!("分块 {}: 密文被截断", i)));
        }

        let decrypted = cipher
            .decrypt(nonce, &blob[offset..offset + expected_cipher])
            .map_err(|e| enc_err(format!("分块 {} 解密失败: {}", i, e)))?;
        offset += expected_cipher;
        plaintext.extend_from_slice(&decrypted);
    }
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

    let original_size = u64::from_be_bytes(
        header[5..13]
            .try_into()
            .expect("v3 头部固定为 21 字节，切片长度必然为 8"),
    );
    let chunk_size = u32::from_be_bytes(
        header[13..17]
            .try_into()
            .expect("v3 头部固定为 21 字节，切片长度必然为 4"),
    ) as usize;
    let chunk_count = u32::from_be_bytes(
        header[17..21]
            .try_into()
            .expect("v3 头部固定为 21 字节，切片长度必然为 4"),
    );

    let cipher = Aes256Gcm::new_from_slice(key).map_err(key_err)?;
    let mut nonce = [0u8; NONCE_SIZE];

    for i in 0..chunk_count as usize {
        reader
            .read_exact(&mut nonce)
            .map_err(|e| CipherError::Io(format!("读取 Nonce 失败: {}", e)))?;
        let expected_plain = if i == chunk_count as usize - 1 {
            (original_size - (i as u64 * chunk_size as u64)) as usize
        } else {
            chunk_size
        };
        let expected_cipher = expected_plain + TAG_SIZE;
        let mut ciphertext = vec![0u8; expected_cipher];
        reader
            .read_exact(&mut ciphertext)
            .map_err(|e| CipherError::Io(format!("读取密文失败: {}", e)))?;
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
    fn test_chunked_roundtrip() {
        let key = [0x42u8; 32];
        let data = vec![0xABu8; 5 * 1024 * 1024];
        let blob = encrypt_chunked_blob(&key, &data, 1024 * 1024).unwrap();
        let decrypted = decrypt_chunked_blob(&key, &blob).unwrap();
        assert_eq!(decrypted.as_slice(), data.as_slice());
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
}
