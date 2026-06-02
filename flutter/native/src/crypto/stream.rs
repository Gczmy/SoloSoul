//! Streaming file encryption/decryption for large files.
//!
//! Uses chunked AES-256-GCM encryption (SOLO blob v3 format).
//! Each chunk is independently encrypted with a unique nonce.
//! Memory usage is O(chunk_size), allowing GB-scale files.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use super::aes::{BLOB_MAGIC, BLOB_VERSION, NONCE_SIZE, TAG_SIZE};

/// SOLO blob v3 version marker
const BLOB_VERSION_V3: u8 = 0x03;

/// Default chunk size: 1 MiB
const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Size of v3 header: Magic(4) + Version(1) + OriginalSize(8) + ChunkSize(4) + ChunkCount(4) = 21 bytes
const V3_HEADER_SIZE: usize = 4 + 1 + 8 + 4 + 4;

/// Custom error type for streaming operations
#[derive(Debug)]
pub enum StreamError {
    Io(io::Error),
    Crypto(String),
    Cancelled,
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::Io(e) => write!(f, "IO error: {}", e),
            StreamError::Crypto(e) => write!(f, "Crypto error: {}", e),
            StreamError::Cancelled => write!(f, "Operation cancelled"),
        }
    }
}

impl From<io::Error> for StreamError {
    fn from(e: io::Error) -> Self {
        StreamError::Io(e)
    }
}

impl From<String> for StreamError {
    fn from(e: String) -> Self {
        StreamError::Crypto(e)
    }
}

impl From<&str> for StreamError {
    fn from(e: &str) -> Self {
        StreamError::Crypto(e.to_string())
    }
}

/// Helper: check if cancel flag file exists
fn is_cancelled(cancel_path: &str) -> bool {
    Path::new(cancel_path).exists()
}

/// Helper: write progress to file
fn write_progress(progress_path: &str, progress: f64) -> io::Result<()> {
    let mut file = File::create(progress_path)?;
    write!(file, "{:.4}", progress)?;
    Ok(())
}

/// Helper: delete file if it exists (cleanup)
fn delete_if_exists(path: &str) {
    let _ = fs::remove_file(path);
}

/// Encrypt a file using chunked AES-256-GCM (SOLO blob v3).
///
/// Reads [src_path] in chunks of [chunk_size], encrypts each chunk independently,
/// and writes the v3 format to [dst_path].
///
/// Progress is written to [progress_path] as a float "0.0" ~ "1.0".
/// If [cancel_path] file is created during operation, encryption stops and
/// partial output is cleaned up.
pub fn encrypt_file_stream(
    key: &[u8; 32],
    src_path: &str,
    dst_path: &str,
    chunk_size: usize,
    progress_path: &str,
    cancel_path: &str,
) -> Result<(), StreamError> {
    let chunk_size = if chunk_size == 0 {
        DEFAULT_CHUNK_SIZE
    } else {
        chunk_size
    };

    // Open source file
    let mut src_file = File::open(src_path)?;
    let original_size = src_file.metadata()?.len() as u64;
    let chunk_count = ((original_size + chunk_size as u64 - 1) / chunk_size as u64) as u32;

    // Create destination file
    let mut dst_file = File::create(dst_path)?;

    // Write v3 header
    let mut header = Vec::with_capacity(V3_HEADER_SIZE);
    header.extend_from_slice(&BLOB_MAGIC);
    header.push(BLOB_VERSION_V3);
    header.extend_from_slice(&original_size.to_be_bytes());
    header.extend_from_slice(&(chunk_size as u32).to_be_bytes());
    header.extend_from_slice(&chunk_count.to_be_bytes());
    dst_file.write_all(&header)?;

    // Create cipher (reused for all chunks — AES key is same, nonce changes)
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {}", e))?;

    let mut buffer = vec![0u8; chunk_size];
    let mut chunk_index: u32 = 0;

    loop {
        // Check cancellation
        if is_cancelled(cancel_path) {
            drop(dst_file);
            delete_if_exists(dst_path);
            delete_if_exists(progress_path);
            return Err(StreamError::Cancelled);
        }

        // Read chunk
        let bytes_read = src_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);

        // Encrypt chunk
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), &buffer[..bytes_read])
            .map_err(|e| format!("Chunk {} encryption failed: {}", chunk_index, e))?;

        // Write nonce + ciphertext
        dst_file.write_all(&nonce_bytes)?;
        dst_file.write_all(&ciphertext)?;

        chunk_index += 1;

        // Update progress
        let progress = chunk_index as f64 / chunk_count as f64;
        let _ = write_progress(progress_path, progress);
    }

    // Finalize
    drop(dst_file);
    delete_if_exists(progress_path);
    delete_if_exists(cancel_path);

    Ok(())
}

/// Decrypt a SOLO blob file (supports both v2 single-chunk and v3 chunked).
///
/// Automatically detects the blob version from the header and uses the
/// appropriate decryption strategy.
///
/// - v3: Streams chunk-by-chunk to keep memory low.
/// - v2: Loads the entire file into memory (v2 files are typically small).
pub fn decrypt_file_stream(
    key: &[u8; 32],
    src_path: &str,
    dst_path: &str,
    progress_path: &str,
    cancel_path: &str,
) -> Result<(), StreamError> {
    // Read just the header to detect version
    let mut version_check = [0u8; 5];
    {
        let mut src_file = File::open(src_path)?;
        src_file.read_exact(&mut version_check)?;
    }

    // Verify magic
    if &version_check[0..4] != &BLOB_MAGIC {
        return Err("Invalid blob magic".into());
    }

    let version = version_check[4];

    if version == BLOB_VERSION_V3 {
        decrypt_v3_file_stream(key, src_path, dst_path, progress_path, cancel_path)
    } else if version == BLOB_VERSION {
        // v2: read entire file and decrypt in-memory
        let mut blob = Vec::new();
        File::open(src_path)?.read_to_end(&mut blob)?;

        let plaintext = super::aes::decrypt_blob(key, &blob)
            .map_err(StreamError::Crypto)?;

        let mut dst_file = File::create(dst_path)?;
        dst_file.write_all(&plaintext)?;
        drop(dst_file);
        delete_if_exists(progress_path);
        delete_if_exists(cancel_path);
        Ok(())
    } else {
        Err(format!("Unsupported blob version: {}", version).into())
    }
}

/// Decrypt a v3 SOLO blob file (chunked stream).
fn decrypt_v3_file_stream(
    key: &[u8; 32],
    src_path: &str,
    dst_path: &str,
    progress_path: &str,
    cancel_path: &str,
) -> Result<(), StreamError> {
    let mut src_file = File::open(src_path)?;
    let mut dst_file = File::create(dst_path)?;

    // Read and validate v3 header
    let mut header = vec![0u8; V3_HEADER_SIZE];
    src_file.read_exact(&mut header)?;

    // Verify magic and version (already checked above, but be safe)
    if &header[0..4] != &BLOB_MAGIC || header[4] != BLOB_VERSION_V3 {
        return Err("Invalid v3 blob header".into());
    }

    // Parse header fields
    let original_size = u64::from_be_bytes([
        header[5], header[6], header[7], header[8],
        header[9], header[10], header[11], header[12],
    ]);
    let chunk_size = u32::from_be_bytes([header[13], header[14], header[15], header[16]]) as usize;
    let chunk_count = u32::from_be_bytes([header[17], header[18], header[19], header[20]]);

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {}", e))?;

    let mut chunk_index: u32 = 0;
    let mut total_decrypted: u64 = 0;

    while chunk_index < chunk_count {
        // Check cancellation
        if is_cancelled(cancel_path) {
            drop(dst_file);
            delete_if_exists(dst_path);
            delete_if_exists(progress_path);
            return Err(StreamError::Cancelled);
        }

        // Read nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        match src_file.read_exact(&mut nonce_bytes) {
            Ok(()) => {},
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }

        // Determine expected ciphertext size for this chunk
        let is_last_chunk = chunk_index == chunk_count - 1;
        let expected_plain_size = if is_last_chunk {
            (original_size - total_decrypted) as usize
        } else {
            chunk_size
        };
        let expected_cipher_size = expected_plain_size + TAG_SIZE;

        // Read ciphertext
        let mut ciphertext = vec![0u8; expected_cipher_size];
        src_file.read_exact(&mut ciphertext)?;

        // Decrypt chunk
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
            .map_err(|e| format!("Chunk {} decryption failed: {}", chunk_index, e))?;

        dst_file.write_all(&plaintext)?;
        total_decrypted += plaintext.len() as u64;
        chunk_index += 1;

        // Update progress
        let progress = chunk_index as f64 / chunk_count as f64;
        let _ = write_progress(progress_path, progress);
    }

    // Verify total size
    if total_decrypted != original_size {
        return Err(format!(
            "Size mismatch: expected {}, got {}",
            original_size, total_decrypted
        ).into());
    }

    // Finalize
    drop(dst_file);
    delete_if_exists(progress_path);
    delete_if_exists(cancel_path);

    Ok(())
}

/// Check if a blob file is in v3 (chunked) format by reading the header.
pub fn is_chunked_blob_file(path: &str) -> Result<bool, io::Error> {
    let mut file = File::open(path)?;
    let mut header = [0u8; 5];
    match file.read_exact(&mut header) {
        Ok(()) => {},
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(false),
        Err(e) => return Err(e),
    }

    if &header[0..4] != &BLOB_MAGIC {
        return Ok(false);
    }

    Ok(header[4] == BLOB_VERSION_V3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_encrypt_decrypt_file_stream_roundtrip() {
        let key = [0x42u8; 32];
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("src.bin");
        let dst_path = temp_dir.path().join("dst.solo");
        let out_path = temp_dir.path().join("out.bin");
        let progress_path = temp_dir.path().join("progress.txt");
        let cancel_path = temp_dir.path().join("cancel.txt");

        // Create source file with 5MB of data
        let data = vec![0xABu8; 5 * 1024 * 1024];
        {
            let mut f = File::create(&src_path).unwrap();
            f.write_all(&data).unwrap();
        }

        // Encrypt
        encrypt_file_stream(
            &key,
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
            1024 * 1024, // 1MB chunks
            progress_path.to_str().unwrap(),
            cancel_path.to_str().unwrap(),
        ).unwrap();

        // Verify v3 format
        assert!(is_chunked_blob_file(dst_path.to_str().unwrap()).unwrap());

        // Decrypt
        decrypt_file_stream(
            &key,
            dst_path.to_str().unwrap(),
            out_path.to_str().unwrap(),
            progress_path.to_str().unwrap(),
            cancel_path.to_str().unwrap(),
        ).unwrap();

        // Verify roundtrip
        let decrypted = fs::read(&out_path).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_cancel_during_encryption() {
        let key = [0x42u8; 32];
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("src.bin");
        let dst_path = temp_dir.path().join("dst.solo");
        let progress_path = temp_dir.path().join("progress.txt");
        let cancel_path = temp_dir.path().join("cancel.txt");

        // Create large source file
        let data = vec![0xCDu8; 10 * 1024 * 1024];
        {
            let mut f = File::create(&src_path).unwrap();
            f.write_all(&data).unwrap();
        }

        // Create cancel flag immediately
        File::create(&cancel_path).unwrap();

        // Encryption should fail with Cancelled
        let result = encrypt_file_stream(
            &key,
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
            1024 * 1024,
            progress_path.to_str().unwrap(),
            cancel_path.to_str().unwrap(),
        );

        assert!(matches!(result, Err(StreamError::Cancelled)));
        // Partial output should be cleaned up
        assert!(!dst_path.exists());
    }
}
