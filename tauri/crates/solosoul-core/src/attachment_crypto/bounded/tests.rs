use super::*;
use crate::attachment_crypto::read_file_decrypted;
use solosoul_crypto::cipher::encrypt_chunked_stream;
use tempfile::TempDir;

fn bounded_cipher(data: &[u8]) -> Vec<u8> {
    let mut input = data;
    let mut bytes = Vec::new();
    encrypt_chunked_stream(&[42; 32], data.len() as u64, &mut input, &mut bytes).unwrap();
    bytes
}

#[test]
fn bounded_file_read_enforces_plaintext_limits_for_both_formats_and_exact_chunk_boundary() {
    let dir = TempDir::new().unwrap();
    for limit in [0u64, 32, 65_536, 10 * 1024 * 1024] {
        for size in [limit.saturating_sub(1), limit, limit + 1] {
            let plaintext = vec![b'p'; size as usize];
            for encrypted in [false, true] {
                let bytes = if encrypted {
                    bounded_cipher(&plaintext)
                } else {
                    plaintext.clone()
                };
                let path = dir
                    .path()
                    .join(format!("public-{limit}-{size}-{encrypted}"));
                std::fs::write(&path, &bytes).unwrap();
                let result = read_file_decrypted(&[42; 32], &path, limit);
                if size <= limit {
                    assert_eq!(result.unwrap(), plaintext);
                } else {
                    assert!(result.is_err());
                }
                assert_eq!(std::fs::read(&path).unwrap(), bytes);
            }
        }
    }
}

struct GrowingAttachment {
    file: File,
    path: std::path::PathBuf,
    append: Option<Vec<u8>>,
}

impl Read for GrowingAttachment {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if let Some(bytes) = self.append.take() {
            std::fs::OpenOptions::new()
                .append(true)
                .open(&self.path)?
                .write_all(&bytes)?;
        }
        self.file.read(buffer)
    }
}

#[test]
fn bounded_read_rejects_real_file_growth_after_open_handle_metadata_was_captured() {
    let dir = TempDir::new().unwrap();
    for encrypted in [false, true] {
        let complete = if encrypted {
            bounded_cipher(&[b'p'; 33])
        } else {
            vec![b'p'; 33]
        };
        let split = if encrypted { 25 } else { 32 };
        let path = dir.path().join(format!("growing-{encrypted}"));
        std::fs::write(&path, &complete[..split]).unwrap();
        let file = File::open(&path).unwrap();
        let advertised_size = file.metadata().unwrap().len();
        let reader = GrowingAttachment {
            file,
            path: path.clone(),
            append: Some(complete[split..].to_vec()),
        };
        assert!(read_attachment_stream(reader, advertised_size, 32, || Ok([42; 32])).is_err());
        // 唯一写入来自测试安排的 append；读取核心没有截断或改写文件。
        assert_eq!(std::fs::read(&path).unwrap(), complete);
    }
}

struct ShortMagicReader(std::io::Cursor<Vec<u8>>);
impl Read for ShortMagicReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let length = buffer.len().min(1);
        self.0.read(&mut buffer[..length])
    }
}

#[test]
fn bounded_read_handles_short_magic_reads_and_keeps_plaintext_key_access_lazy() {
    for encrypted in [false, true] {
        let data = b"public contents";
        let source = if encrypted {
            bounded_cipher(data)
        } else {
            data.to_vec()
        };
        let size = source.len() as u64;
        let key_calls = std::cell::Cell::new(0);
        let bytes = read_attachment_stream(
            ShortMagicReader(std::io::Cursor::new(source)),
            size,
            32,
            || {
                key_calls.set(key_calls.get() + 1);
                Ok([42; 32])
            },
        )
        .unwrap();
        assert_eq!(bytes, data);
        assert_eq!(key_calls.get(), usize::from(encrypted));
    }
}

struct CountingAttachment {
    source: std::io::Cursor<Vec<u8>>,
    count: std::rc::Rc<std::cell::Cell<usize>>,
}
impl Read for CountingAttachment {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.source.read(buffer)?;
        self.count.set(self.count.get() + count);
        Ok(count)
    }
}

#[test]
fn bounded_read_caps_actual_plain_and_cipher_input_when_metadata_is_smaller() {
    for encrypted in [false, true] {
        let plain = vec![b'p'; 1024 * 1024];
        let source = if encrypted {
            bounded_cipher(&plain)
        } else {
            plain
        };
        let count = std::rc::Rc::new(std::cell::Cell::new(0));
        let reader = CountingAttachment {
            source: std::io::Cursor::new(source),
            count: count.clone(),
        };
        assert!(read_attachment_stream(reader, 0, 32, || Ok([42; 32])).is_err());
        assert!(count.get() <= if encrypted { 32 + 25 + 16 + 1 } else { 33 });
    }
}

#[test]
fn bounded_read_rejects_trailing_ciphertext_and_invalid_limits() {
    let mut cipher = bounded_cipher(&[]);
    cipher.extend_from_slice(b"trailing bytes");
    let size = cipher.len() as u64;
    assert!(read_attachment_stream(cipher.as_slice(), size, 32, || Ok([42; 32])).is_err());
    assert!(read_attachment_stream(&b""[..], 0, u64::MAX, || panic!("No key needed")).is_err());
    let cipher = bounded_cipher(b"public");
    assert!(
        read_attachment_stream(cipher.as_slice(), cipher.len() as u64, 32, || Ok([43; 32]))
            .is_err()
    );
    let mut damaged = cipher;
    *damaged.last_mut().unwrap() ^= 1;
    assert!(read_attachment_stream(
        damaged.as_slice(),
        damaged.len() as u64,
        32,
        || Ok([42; 32])
    )
    .is_err());
}
