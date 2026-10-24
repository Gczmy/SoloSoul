use super::*;
use std::io::{self, Cursor};

const MAX_CIPHERTEXT: usize = CHUNK_SIZE + 16;

fn encrypted(plaintext: &[u8], legacy: bool) -> Vec<u8> {
    let key = [42; 32];
    if !legacy {
        let mut result = Vec::new();
        encrypt_chunked_stream(
            &key,
            plaintext.len() as u64,
            &mut &plaintext[..],
            &mut result,
        )
        .unwrap();
        return result;
    }
    let nonce = [7; 12];
    let count = plaintext.len().div_ceil(CHUNK_SIZE) as u64;
    let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
    let mut result = nonce.to_vec();
    result.extend_from_slice(&count.to_be_bytes());
    for (index, chunk) in plaintext.chunks(CHUNK_SIZE).enumerate() {
        let mut chunk_nonce = nonce;
        for (target, byte) in chunk_nonce[4..]
            .iter_mut()
            .zip((index as u64).to_be_bytes())
        {
            *target ^= byte;
        }
        result.extend_from_slice(
            &cipher
                .encrypt(Nonce::from_slice(&chunk_nonce), chunk)
                .unwrap(),
        );
    }
    result
}

fn assert_rejected(ciphertext: &[u8]) {
    for key in [[42; 32], [9; 32]] {
        assert!(decrypt_chunked_from_bytes(&key, ciphertext).is_err());
        let mut output = b"untouched".to_vec();
        assert!(decrypt_chunked_stream(&key, &mut &ciphertext[..], &mut output).is_err());
        assert_eq!(output, b"untouched");
    }
}

#[test]
fn p058_single_chunk_cannot_be_changed_to_zero() {
    for legacy in [false, true] {
        let mut ciphertext = encrypted(b"public attachment", legacy);
        let offset = if legacy { 12 } else { 17 };
        assert_eq!(&ciphertext[offset..offset + 8], &1u64.to_be_bytes());
        ciphertext[offset..offset + 8].copy_from_slice(&0u64.to_be_bytes());
        assert_rejected(&ciphertext);
    }
}

#[test]
fn p058_zero_chunks_require_exact_eof() {
    for legacy in [false, true] {
        let mut ciphertext = encrypted(b"", legacy);
        ciphertext.push(42);
        assert_rejected(&ciphertext);
    }
}

#[test]
fn p058_empty_legacy_and_current_files_remain_readable() {
    for legacy in [false, true] {
        let ciphertext = encrypted(b"", legacy);
        assert!(decrypt_chunked_from_bytes(&[42; 32], &ciphertext)
            .unwrap()
            .is_empty());
        let mut output = Vec::new();
        decrypt_chunked_stream(&[42; 32], &mut ciphertext.as_slice(), &mut output).unwrap();
        assert!(output.is_empty());
    }
}

#[test]
fn p058_chunk_boundary_roundtrips_both_formats_and_readers() {
    for legacy in [false, true] {
        for length in [
            1,
            CHUNK_SIZE - 1,
            CHUNK_SIZE,
            CHUNK_SIZE + 1,
            2 * CHUNK_SIZE,
        ] {
            let plaintext: Vec<_> = (0..length).map(|i| (i % 251) as u8).collect();
            let ciphertext = encrypted(&plaintext, legacy);
            assert_eq!(
                decrypt_chunked_from_bytes(&[42; 32], &ciphertext)
                    .unwrap()
                    .as_slice(),
                plaintext
            );
            let mut output = Vec::new();
            decrypt_chunked_stream(&[42; 32], &mut ciphertext.as_slice(), &mut output).unwrap();
            assert_eq!(output, plaintext);
        }
    }
}

#[test]
fn p058_even_authenticated_oversized_final_chunks_are_rejected() {
    for legacy in [false, true] {
        let mut ciphertext = encrypted(b"x", legacy);
        let header_len = if legacy {
            LEGACY_HEADER_LEN
        } else {
            NEW_HEADER_LEN
        };
        ciphertext.truncate(header_len);
        let mut nonce = [0; 12];
        let nonce_offset = if legacy { 0 } else { 5 };
        nonce.copy_from_slice(&ciphertext[nonce_offset..nonce_offset + 12]);
        let aad = chunked_aad(&nonce, 1);
        let plaintext = vec![42; CHUNK_SIZE + 1];
        let cipher = Aes256Gcm::new_from_slice(&[42; 32]).unwrap();
        ciphertext.extend_from_slice(
            &cipher
                .encrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: &plaintext,
                        aad: if legacy { &[] } else { &aad },
                    },
                )
                .unwrap(),
        );
        assert_rejected(&ciphertext);
    }
}

#[test]
fn p058_final_chunk_reads_only_one_bounded_chunk_and_probe_byte() {
    for legacy in [false, true] {
        let mut ciphertext = encrypted(b"x", legacy);
        let header_len = if legacy {
            LEGACY_HEADER_LEN
        } else {
            NEW_HEADER_LEN
        };
        ciphertext.resize(header_len + 4 * MAX_CIPHERTEXT, 42);
        let mut reader = Cursor::new(&ciphertext);
        let mut output = Vec::new();
        assert!(decrypt_chunked_stream(&[42; 32], &mut reader, &mut output).is_err());
        assert!(output.is_empty());
        assert_eq!(reader.position(), (header_len + MAX_CIPHERTEXT + 1) as u64);
    }
}

#[test]
fn p058_large_counts_and_truncated_blocks_fail_without_output() {
    for legacy in [false, true] {
        let ciphertext = encrypted(b"public", legacy);
        let header_len = if legacy {
            LEGACY_HEADER_LEN
        } else {
            NEW_HEADER_LEN
        };
        let offset = if legacy { 12 } else { 17 };
        for count in [1, 1u64 << 32, u64::MAX] {
            let mut truncated = ciphertext[..header_len].to_vec();
            truncated[offset..offset + 8].copy_from_slice(&count.to_be_bytes());
            assert_rejected(&truncated);
        }
    }
}

struct ShortReader<'a> {
    input: &'a [u8],
    interrupt: bool,
    fail_at_eof: bool,
}

impl Read for ShortReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.interrupt = !self.interrupt;
        if self.interrupt {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.input.is_empty() && self.fail_at_eof {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let length = buffer.len().min(self.input.len()).min(3);
        buffer[..length].copy_from_slice(&self.input[..length]);
        self.input = &self.input[length..];
        Ok(length)
    }
}

#[test]
fn p058_short_reads_retry_interrupts_and_do_not_hide_eof_io_errors() {
    for legacy in [false, true] {
        for plaintext in [b"".as_slice(), b"public fixture".as_slice()] {
            let ciphertext = encrypted(plaintext, legacy);
            for fail_at_eof in [false, true] {
                let mut reader = ShortReader {
                    input: &ciphertext,
                    interrupt: false,
                    fail_at_eof,
                };
                let mut output = Vec::new();
                let result = decrypt_chunked_stream(&[42; 32], &mut reader, &mut output);
                if fail_at_eof {
                    assert!(matches!(result, Err(CipherError::Io(_))));
                    assert!(output.is_empty());
                } else {
                    result.unwrap();
                    assert_eq!(output, plaintext);
                }
            }
        }
    }
}
