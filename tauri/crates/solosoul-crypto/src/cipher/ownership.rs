//! 只认证非空 SOLC v2 的完整流，不返回明文，也不做文件删除决策。
//!
//! v2 格式沿用 cipher.rs：magic/version 是结构约束，所有块的 AAD 为
//! base_nonce || chunk_count。块索引按大端 u64 与 nonce 的后 8 字节 XOR。

use super::{chunked_aad, CHUNKED_MAGIC, CHUNKED_VERSION, CHUNK_SIZE, NEW_HEADER_LEN};
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use std::io::{self, Read};
use zeroize::Zeroizing;

const TAG_LEN: usize = 16;
const MAX_CHUNK_CIPHERTEXT: usize = CHUNK_SIZE + TAG_LEN;

/// 全部块通过同一密钥认证、长度符合 v2 布局且读到精确 EOF 的证据。
///
/// 这只证明所读取字节属于该密钥域；不证明账户、路径、文件身份或引用状态。
/// 后四项必须由调用方使用同一打开文件和原会话另行检查。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedChunkedV2 {
    pub header: [u8; NEW_HEADER_LEN],
    pub chunk_count: u64,
    pub ciphertext_length: u64,
    pub plaintext_length: u64,
}

/// 不能证明密钥归属的保留原因。任何此结果都不允许作为删除依据。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkedOwnershipRetention {
    LegacyOrPlaintext,
    UnsupportedVersion {
        version: u8,
    },
    MalformedHeader,
    ZeroChunks,
    InvalidLayout,
    Truncated,
    TrailingData,
    /// AEAD 无法区分其他密钥与损坏/篡改；不得把这个结果写成已确认的其他账户。
    Unauthenticated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChunkedOwnership {
    Authenticated(AuthenticatedChunkedV2),
    Retained(ChunkedOwnershipRetention),
}

/// 从字节 0 开始，认证同一个已打开 reader 内完整、非空的 SOLC v2 文件。
///
/// `expected_ciphertext_length` 必须来自该打开文件的冻结 metadata.len()，不能
/// 来自路径再次打开、明文 header 声明或其他文件。调用方负责 reader 位于字节 0、
/// 文件身份未改变以及返回后仍属于原扫描/会话窗口。
///
/// 头仅读取一次并冻结；每块密文最大 65552 字节，解密明文至多 65536 字节并
/// 用 Zeroizing 及时擦除。不使用 read_to_end、不按块数分配、不输出任何明文。
/// 块数保持 u64，所有布局运算检查溢出；认证成功前必须额外读 1 字节确认 EOF。
///
/// 截断/格式错误返回 Retained；非 EOF I/O 错误原样返回 Err，由扫描器保留。
pub fn authenticate_nonempty_chunked_v2_stream<R: Read>(
    key: &[u8; 32],
    reader: &mut R,
    expected_ciphertext_length: u64,
) -> io::Result<ChunkedOwnership> {
    use ChunkedOwnershipRetention as Retention;

    let mut header = [0u8; NEW_HEADER_LEN];
    if read_fully_or_eof(reader, &mut header[..4])? != 4 || header[..4] != CHUNKED_MAGIC {
        return Ok(ChunkedOwnership::Retained(Retention::LegacyOrPlaintext));
    }
    if read_fully_or_eof(reader, &mut header[4..5])? != 1 {
        return Ok(ChunkedOwnership::Retained(Retention::MalformedHeader));
    }
    if header[4] != CHUNKED_VERSION {
        return Ok(ChunkedOwnership::Retained(Retention::UnsupportedVersion {
            version: header[4],
        }));
    }
    if read_fully_or_eof(reader, &mut header[5..])? != NEW_HEADER_LEN - 5 {
        return Ok(ChunkedOwnership::Retained(Retention::MalformedHeader));
    }

    let mut base_nonce = [0u8; 12];
    base_nonce.copy_from_slice(&header[5..17]);
    let mut count_bytes = [0u8; 8];
    count_bytes.copy_from_slice(&header[17..25]);
    let chunk_count = u64::from_be_bytes(count_bytes);
    if chunk_count == 0 {
        // 零块没有 AEAD tag，连合法加密器产生的空文件也不能认证归属。
        return Ok(ChunkedOwnership::Retained(Retention::ZeroChunks));
    }

    let prefix_length = (chunk_count - 1)
        .checked_mul(MAX_CHUNK_CIPHERTEXT as u64)
        .and_then(|length| length.checked_add(NEW_HEADER_LEN as u64));
    let final_length = prefix_length
        .and_then(|length| expected_ciphertext_length.checked_sub(length))
        .filter(|length| (TAG_LEN as u64 + 1..=MAX_CHUNK_CIPHERTEXT as u64).contains(length));
    let Some(final_length) = final_length else {
        return Ok(ChunkedOwnership::Retained(Retention::InvalidLayout));
    };

    let aad = chunked_aad(&base_nonce, chunk_count);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid AES-256 key"))?;
    let mut ciphertext = vec![0u8; MAX_CHUNK_CIPHERTEXT];
    let mut plaintext_length = 0u64;
    for index in 0..chunk_count {
        let length = if index == chunk_count - 1 {
            final_length as usize // 上面已约束至固定单块上限，包含 32 位平台。
        } else {
            MAX_CHUNK_CIPHERTEXT
        };
        if read_fully_or_eof(reader, &mut ciphertext[..length])? != length {
            return Ok(ChunkedOwnership::Retained(Retention::Truncated));
        }

        let mut nonce_bytes = base_nonce;
        for (nonce_byte, index_byte) in nonce_bytes[4..].iter_mut().zip(index.to_be_bytes()) {
            *nonce_byte ^= index_byte;
        }
        let plaintext = match cipher.decrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: &ciphertext[..length],
                aad: &aad,
            },
        ) {
            Ok(plaintext) => Zeroizing::new(plaintext),
            Err(_) => return Ok(ChunkedOwnership::Retained(Retention::Unauthenticated)),
        };
        if plaintext.len() != length - TAG_LEN {
            return Ok(ChunkedOwnership::Retained(Retention::InvalidLayout));
        }
        let Some(total) = plaintext_length.checked_add(plaintext.len() as u64) else {
            return Ok(ChunkedOwnership::Retained(Retention::InvalidLayout));
        };
        plaintext_length = total;
        // plaintext 在每次循环末立即 Drop/zeroize，不汇集全文件明文。
    }

    let mut eof_probe = [0u8; 1];
    if read_fully_or_eof(reader, &mut eof_probe)? != 0 {
        return Ok(ChunkedOwnership::Retained(Retention::TrailingData));
    }
    Ok(ChunkedOwnership::Authenticated(AuthenticatedChunkedV2 {
        header,
        chunk_count,
        ciphertext_length: expected_ciphertext_length,
        plaintext_length,
    }))
}

// 对短读和 Interrupted 保持 Read 的标准语义；非 EOF 错误不能降级为已认证。
fn read_fully_or_eof<R: Read>(reader: &mut R, mut target: &mut [u8]) -> io::Result<usize> {
    let expected = target.len();
    while !target.is_empty() {
        match reader.read(target) {
            Ok(0) => break,
            Ok(length) => target = &mut target[length..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(expected - target.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cipher::encrypt_chunked_stream;
    use std::io::{Cursor, Seek, SeekFrom};

    fn encrypt_bytes(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
        let mut ciphertext = Vec::new();
        encrypt_chunked_stream(
            key,
            plaintext.len() as u64,
            &mut Cursor::new(plaintext),
            &mut ciphertext,
        )
        .unwrap();
        ciphertext
    }

    fn check(key: &[u8; 32], ciphertext: &[u8]) -> ChunkedOwnership {
        authenticate_nonempty_chunked_v2_stream(
            key,
            &mut Cursor::new(ciphertext),
            ciphertext.len() as u64,
        )
        .unwrap()
    }

    fn retained(key: &[u8; 32], ciphertext: &[u8], reason: ChunkedOwnershipRetention) {
        assert_eq!(check(key, ciphertext), ChunkedOwnership::Retained(reason));
    }

    struct FragmentedReader {
        cursor: Cursor<Vec<u8>>,
        max_request: usize,
        fragment: usize,
        interrupt_once: bool,
        fail_at: Option<u64>,
    }

    impl Read for FragmentedReader {
        fn read(&mut self, target: &mut [u8]) -> io::Result<usize> {
            self.max_request = self.max_request.max(target.len());
            if self.interrupt_once {
                self.interrupt_once = false;
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            if self.fail_at == Some(self.cursor.position()) {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            let mut length = target.len().min(self.fragment);
            if let Some(fail_at) = self.fail_at {
                length = length.min(fail_at.saturating_sub(self.cursor.position()) as usize);
            }
            self.cursor.read(&mut target[..length])
        }
    }

    #[test]
    fn rf903_real_multichunk_authenticates_frozen_header_and_bounded_reads() {
        let key = [0x71; 32];
        let plaintext: Vec<u8> = (0..CHUNK_SIZE * 3 + 777).map(|i| (i % 251) as u8).collect();
        let ciphertext = encrypt_bytes(&key, &plaintext);
        let mut reader = FragmentedReader {
            cursor: Cursor::new(ciphertext.clone()),
            max_request: 0,
            fragment: 4093,
            interrupt_once: true,
            fail_at: None,
        };
        let result =
            authenticate_nonempty_chunked_v2_stream(&key, &mut reader, ciphertext.len() as u64)
                .unwrap();
        let ChunkedOwnership::Authenticated(metadata) = result else {
            panic!("real four-chunk ciphertext must authenticate");
        };
        assert_eq!(metadata.header.as_slice(), &ciphertext[..NEW_HEADER_LEN]);
        assert_eq!(metadata.chunk_count, 4);
        assert_eq!(metadata.plaintext_length, plaintext.len() as u64);
        assert_eq!(metadata.ciphertext_length, ciphertext.len() as u64);
        assert_eq!(reader.cursor.position(), ciphertext.len() as u64);
        assert!(reader.max_request <= MAX_CHUNK_CIPHERTEXT);
    }

    #[test]
    fn rf903_same_open_file_reader_authenticates_without_reopening_path() {
        struct OwnedTempFile {
            path: std::path::PathBuf,
            file: Option<std::fs::File>,
        }
        impl Drop for OwnedTempFile {
            fn drop(&mut self) {
                drop(self.file.take());
                let _ = std::fs::remove_file(&self.path);
            }
        }
        let path = std::env::temp_dir().join(format!(
            "ss-rf903-ownership-{}-{:016x}.solc",
            std::process::id(),
            rand::random::<u64>()
        ));
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut owned = OwnedTempFile {
            path,
            file: Some(file),
        };
        let file = owned.file.as_mut().unwrap();
        let key = [0x35; 32];
        let plaintext = vec![0x42; CHUNK_SIZE + 513];
        encrypt_chunked_stream(
            &key,
            plaintext.len() as u64,
            &mut Cursor::new(&plaintext),
            file,
        )
        .unwrap();
        file.sync_all().unwrap();
        let frozen_length = file.metadata().unwrap().len();
        file.seek(SeekFrom::Start(0)).unwrap();
        let result = authenticate_nonempty_chunked_v2_stream(&key, file, frozen_length).unwrap();
        assert!(matches!(result, ChunkedOwnership::Authenticated(meta)
            if meta.chunk_count == 2 && meta.plaintext_length == plaintext.len() as u64));
        assert_eq!(file.stream_position().unwrap(), frozen_length);
    }

    #[test]
    fn rf903_nonempty_boundary_lengths_authenticate() {
        let key = [0x42; 32];
        for length in [1, CHUNK_SIZE, CHUNK_SIZE + 1, CHUNK_SIZE * 2] {
            let ciphertext = encrypt_bytes(&key, &vec![0x55; length]);
            assert!(
                matches!(check(&key, &ciphertext), ChunkedOwnership::Authenticated(meta)
                if meta.plaintext_length == length as u64)
            );
        }
    }

    #[test]
    fn rf903_other_key_retained_without_claiming_foreign_account() {
        let ciphertext = encrypt_bytes(&[0x42; 32], &vec![0x55; CHUNK_SIZE + 13]);
        retained(
            &[0x43; 32],
            &ciphertext,
            ChunkedOwnershipRetention::Unauthenticated,
        );
    }

    #[test]
    fn rf903_each_chunk_tag_is_verified_including_final_chunk() {
        let key = [0x42; 32];
        let ciphertext = encrypt_bytes(&key, &vec![0x55; CHUNK_SIZE * 2 + 13]);
        for index in 0..3 {
            let mut damaged = ciphertext.clone();
            let tag_byte = if index == 2 {
                damaged.len() - 1
            } else {
                NEW_HEADER_LEN + (index + 1) * MAX_CHUNK_CIPHERTEXT - 1
            };
            damaged[tag_byte] ^= 1;
            retained(&key, &damaged, ChunkedOwnershipRetention::Unauthenticated);
        }
    }

    #[test]
    fn rf903_frozen_nonce_is_authenticated() {
        let key = [0x42; 32];
        let mut ciphertext = encrypt_bytes(&key, b"nonempty");
        ciphertext[5] ^= 1;
        retained(
            &key,
            &ciphertext,
            ChunkedOwnershipRetention::Unauthenticated,
        );
    }

    #[test]
    fn rf903_chunk_count_is_authenticated_even_with_valid_changed_layout() {
        let key = [0x42; 32];
        let mut ciphertext = encrypt_bytes(&key, &vec![0x55; CHUNK_SIZE + 13]);
        ciphertext[17..25].copy_from_slice(&1u64.to_be_bytes());
        ciphertext.truncate(NEW_HEADER_LEN + MAX_CHUNK_CIPHERTEXT);
        retained(
            &key,
            &ciphertext,
            ChunkedOwnershipRetention::Unauthenticated,
        );
    }

    #[test]
    fn rf903_swapped_chunks_retain_due_to_per_chunk_nonce() {
        let key = [0x42; 32];
        let ciphertext = encrypt_bytes(&key, &vec![0x55; CHUNK_SIZE * 2]);
        let mut reordered = ciphertext[..NEW_HEADER_LEN].to_vec();
        reordered.extend_from_slice(&ciphertext[NEW_HEADER_LEN + MAX_CHUNK_CIPHERTEXT..]);
        reordered
            .extend_from_slice(&ciphertext[NEW_HEADER_LEN..NEW_HEADER_LEN + MAX_CHUNK_CIPHERTEXT]);
        retained(&key, &reordered, ChunkedOwnershipRetention::Unauthenticated);
    }

    #[test]
    fn rf903_forged_solc_header_and_body_do_not_authenticate() {
        let mut fake = vec![0u8; NEW_HEADER_LEN + TAG_LEN + 1];
        fake[..4].copy_from_slice(&CHUNKED_MAGIC);
        fake[4] = CHUNKED_VERSION;
        fake[17..25].copy_from_slice(&1u64.to_be_bytes());
        retained(
            &[0x42; 32],
            &fake,
            ChunkedOwnershipRetention::Unauthenticated,
        );
    }

    #[test]
    fn rf903_truncation_retained_with_original_and_changed_metadata_lengths() {
        let key = [0x42; 32];
        let mut ciphertext = encrypt_bytes(&key, &vec![0x55; CHUNK_SIZE + 13]);
        let original_length = ciphertext.len() as u64;
        ciphertext.pop();
        let result = authenticate_nonempty_chunked_v2_stream(
            &key,
            &mut Cursor::new(&ciphertext),
            original_length,
        )
        .unwrap();
        assert_eq!(
            result,
            ChunkedOwnership::Retained(ChunkedOwnershipRetention::Truncated)
        );
        retained(
            &key,
            &ciphertext,
            ChunkedOwnershipRetention::Unauthenticated,
        );
    }

    #[test]
    fn rf903_appended_tail_is_rejected_with_original_and_changed_metadata_lengths() {
        let key = [0x42; 32];
        let mut ciphertext = encrypt_bytes(&key, b"nonempty");
        let original_length = ciphertext.len() as u64;
        ciphertext.push(0x71);
        let result = authenticate_nonempty_chunked_v2_stream(
            &key,
            &mut Cursor::new(&ciphertext),
            original_length,
        )
        .unwrap();
        assert_eq!(
            result,
            ChunkedOwnership::Retained(ChunkedOwnershipRetention::TrailingData)
        );
        retained(
            &key,
            &ciphertext,
            ChunkedOwnershipRetention::Unauthenticated,
        );
    }

    #[test]
    fn rf903_zero_chunk_writer_output_and_extra_tail_never_prove_ownership() {
        let key = [0x42; 32];
        let mut ciphertext = encrypt_bytes(&key, b"");
        retained(&key, &ciphertext, ChunkedOwnershipRetention::ZeroChunks);
        ciphertext.extend_from_slice(b"arbitrary trailing bytes");
        retained(&key, &ciphertext, ChunkedOwnershipRetention::ZeroChunks);
    }

    #[test]
    fn rf903_legacy_ciphertext_and_plaintext_are_retained() {
        let key = [0x42; 32];
        let nonce = [0x17; 12];
        let mut legacy = nonce.to_vec();
        legacy.extend_from_slice(&1u64.to_be_bytes());
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        legacy.extend(
            cipher
                .encrypt(Nonce::from_slice(&nonce), b"legacy".as_slice())
                .unwrap(),
        );
        retained(&key, &legacy, ChunkedOwnershipRetention::LegacyOrPlaintext);
        for plaintext in [
            b"".as_slice(),
            b"SO".as_slice(),
            b"plain attachment".as_slice(),
        ] {
            retained(
                &key,
                plaintext,
                ChunkedOwnershipRetention::LegacyOrPlaintext,
            );
        }
    }

    #[test]
    fn rf903_unsupported_version_and_incomplete_solc_headers_are_distinct() {
        retained(
            &[0x42; 32],
            b"SOLC\x03",
            ChunkedOwnershipRetention::UnsupportedVersion { version: 3 },
        );
        for bytes in [
            b"SOLC".as_slice(),
            b"SOLC\x02".as_slice(),
            b"SOLC\x02short".as_slice(),
        ] {
            retained(
                &[0x42; 32],
                bytes,
                ChunkedOwnershipRetention::MalformedHeader,
            );
        }
    }

    #[test]
    fn rf903_huge_declared_count_overflow_is_rejected_before_body_read() {
        let mut header = [0u8; NEW_HEADER_LEN];
        header[..4].copy_from_slice(&CHUNKED_MAGIC);
        header[4] = CHUNKED_VERSION;
        header[17..25].copy_from_slice(&u64::MAX.to_be_bytes());
        let mut reader = FragmentedReader {
            cursor: Cursor::new(header.to_vec()),
            max_request: 0,
            fragment: usize::MAX,
            interrupt_once: false,
            fail_at: None,
        };
        let result =
            authenticate_nonempty_chunked_v2_stream(&[0x42; 32], &mut reader, u64::MAX).unwrap();
        assert_eq!(
            result,
            ChunkedOwnership::Retained(ChunkedOwnershipRetention::InvalidLayout)
        );
        assert_eq!(reader.cursor.position(), NEW_HEADER_LEN as u64);
        assert!(reader.max_request <= NEW_HEADER_LEN);
    }

    #[test]
    fn rf903_noncanonical_empty_last_chunk_and_oversized_last_chunk_are_retained() {
        let key = [0x42; 32];
        let nonce = [0x17; 12];
        let mut header = CHUNKED_MAGIC.to_vec();
        header.push(CHUNKED_VERSION);
        header.extend_from_slice(&nonce);
        header.extend_from_slice(&1u64.to_be_bytes());
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let mut empty_final = header;
        empty_final.extend(
            cipher
                .encrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: b"",
                        aad: &chunked_aad(&nonce, 1),
                    },
                )
                .unwrap(),
        );
        retained(&key, &empty_final, ChunkedOwnershipRetention::InvalidLayout);
        let mut oversized = encrypt_bytes(&key, &vec![0x55; CHUNK_SIZE]);
        oversized.push(0);
        retained(&key, &oversized, ChunkedOwnershipRetention::InvalidLayout);
    }

    #[test]
    fn rf903_inconsistent_short_frozen_length_cannot_authenticate() {
        let key = [0x42; 32];
        let ciphertext = encrypt_bytes(&key, b"nonempty");
        let result =
            authenticate_nonempty_chunked_v2_stream(&key, &mut Cursor::new(&ciphertext), 1)
                .unwrap();
        assert_eq!(
            result,
            ChunkedOwnership::Retained(ChunkedOwnershipRetention::InvalidLayout)
        );
    }

    #[test]
    fn rf903_non_eof_io_errors_before_header_in_body_and_at_eof_propagate() {
        let key = [0x42; 32];
        let ciphertext = encrypt_bytes(&key, &vec![0x55; CHUNK_SIZE + 13]);
        for fail_at in [0, NEW_HEADER_LEN as u64 + 17, ciphertext.len() as u64] {
            let mut reader = FragmentedReader {
                cursor: Cursor::new(ciphertext.clone()),
                max_request: 0,
                fragment: 4093,
                interrupt_once: false,
                fail_at: Some(fail_at),
            };
            let error =
                authenticate_nonempty_chunked_v2_stream(&key, &mut reader, ciphertext.len() as u64)
                    .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
            assert!(reader.max_request <= MAX_CHUNK_CIPHERTEXT);
        }
    }
}
