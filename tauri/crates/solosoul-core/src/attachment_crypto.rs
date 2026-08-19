//! P001：附件静态加密（at-rest）工具。
//!
//! 附件文件不再明文落盘，统一以 `encrypt_chunked_stream`（SOLC magic 头）加密存储：
//! - **写入**：任何把附件复制进 Vault `attachments/` 目录的路径都必须先加密；
//! - **读取**：先检测文件头 magic——`SOLC` 开头则流式解密，否则视为旧版本明文
//!   直接返回（零迁移兼容，旧附件不重加密也能继续使用）。
//!
//! 密钥由调用方提供（`VaultService::attachment_encryption_key()`，HKDF 派生），
//! 本模块不接触会话密钥本身，便于 GUI / CLI / 测试共用。

use solosoul_crypto::cipher::{decrypt_chunked_stream, encrypt_chunked_stream, CipherError};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

/// HKDF info 标签：附件静态加密（与导出包附件密钥 `solosoul:attachments:v1` 区分）。
pub const ATTACHMENT_AT_REST_INFO: &[u8] = b"solosoul:attachments:at-rest:v1";
/// HKDF salt：附件静态加密域。
pub const ATTACHMENT_AT_REST_SALT: &[u8] = b"solosoul:attachments:at-rest";

/// 由会话密钥派生附件静态加密密钥（32 字节）。
pub fn derive_attachment_key(session_key: &[u8; 32]) -> Result<[u8; 32], String> {
    solosoul_crypto::hkdf_ext::derive_hkdf_key(
        session_key,
        ATTACHMENT_AT_REST_SALT,
        ATTACHMENT_AT_REST_INFO,
    )
    .map_err(|e| format!("附件密钥派生失败: {e}"))
}

/// 检测文件是否以 SOLC magic 开头（已加密）。
pub fn is_encrypted_file(path: &Path) -> bool {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut magic = [0u8; 4];
    if f.read_exact(&mut magic).is_err() {
        return false;
    }
    magic == *b"SOLC"
}

/// 流式加密复制：把 `src` 明文加密写入 `dst`（SOLC 头格式）。
pub fn encrypt_file_stream(key: &[u8; 32], src: &Path, dst: &Path) -> Result<(), String> {
    let mut reader = BufReader::new(File::open(src).map_err(|e| format!("打开源文件失败: {e}"))?);
    let file_size = std::fs::metadata(src)
        .map(|m| m.len())
        .map_err(|e| format!("读取源文件元数据失败: {e}"))?;
    let mut writer =
        BufWriter::new(File::create(dst).map_err(|e| format!("创建目标文件失败: {e}"))?);
    encrypt_chunked_stream(key, file_size, &mut reader, &mut writer)
        .map_err(|e| format!("加密附件失败: {e}"))?;
    writer
        .flush()
        .map_err(|e| format!("写入目标文件失败: {e}"))?;
    Ok(())
}

/// 把 `src` 复制到 `dst`：若 `src` 是 SOLC 密文则解密复制，否则原样复制（旧明文兼容）。
pub fn copy_decrypt_file(key: &[u8; 32], src: &Path, dst: &Path) -> Result<(), String> {
    if is_encrypted_file(src) {
        let mut reader =
            BufReader::new(File::open(src).map_err(|e| format!("打开源文件失败: {e}"))?);
        let mut writer =
            BufWriter::new(File::create(dst).map_err(|e| format!("创建目标文件失败: {e}"))?);
        decrypt_chunked_stream(key, &mut reader, &mut writer)
            .map_err(|e| format!("解密附件失败: {e}"))?;
        writer
            .flush()
            .map_err(|e| format!("写入目标文件失败: {e}"))?;
        Ok(())
    } else {
        std::fs::copy(src, dst).map_err(|e| format!("复制文件失败: {e}"))?;
        Ok(())
    }
}

/// 读取文件全部内容：SOLC 密文则解密，否则原样（旧明文兼容）。
/// `max_size` 为读取上限（防御超大文件 OOM）；密文长度上限按其 1.03 倍放大估算。
pub fn read_file_decrypted(key: &[u8; 32], path: &Path, max_size: u64) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("读取文件元数据失败: {e}"))?;
    if meta.len() > max_size * 103 / 100 {
        return Err(format!("文件过大（{} 字节）", meta.len()));
    }
    let mut file = File::open(path).map_err(|e| format!("打开文件失败: {e}"))?;
    if !is_encrypted_file(path) {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| format!("读取文件失败: {e}"))?;
        return Ok(buf);
    }
    let mut out: Vec<u8> = Vec::new();
    decrypt_chunked_stream(key, &mut file, &mut out).map_err(|e: CipherError| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn roundtrip_key() -> [u8; 32] {
        [0x42u8; 32]
    }

    #[test]
    fn test_derive_attachment_key_deterministic_and_distinct() {
        let session = [0x11u8; 32];
        let k1 = derive_attachment_key(&session).unwrap();
        let k2 = derive_attachment_key(&session).unwrap();
        assert_eq!(k1, k2);
        assert_ne!(k1, session, "附件密钥必须与会话密钥域分离");
        // 不同会话密钥 → 不同附件密钥
        let other = [0x22u8; 32];
        assert_ne!(k1, derive_attachment_key(&other).unwrap());
    }

    #[test]
    fn test_encrypt_roundtrip() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("a.txt");
        let enc = dir.path().join("a.txt.enc");
        let dec = dir.path().join("a.out");
        let plaintext = b"hello attachment at-rest".repeat(10);
        std::fs::write(&src, &plaintext).unwrap();

        encrypt_file_stream(&roundtrip_key(), &src, &enc).unwrap();
        assert!(is_encrypted_file(&enc));
        assert!(!is_encrypted_file(&src));

        copy_decrypt_file(&roundtrip_key(), &enc, &dec).unwrap();
        assert_eq!(std::fs::read(&dec).unwrap(), plaintext);
    }

    #[test]
    fn test_copy_decrypt_passthrough_plaintext() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("legacy.bin");
        let dst = dir.path().join("legacy.out");
        let data = b"old plaintext attachment, no SOLC magic".repeat(3);
        std::fs::write(&src, &data).unwrap();

        copy_decrypt_file(&roundtrip_key(), &src, &dst).unwrap();
        assert_eq!(std::fs::read(&dst).unwrap(), data);
        assert!(!is_encrypted_file(&dst));
    }

    #[test]
    fn test_read_file_decrypted_both_formats() {
        let dir = TempDir::new().unwrap();
        let plain = dir.path().join("p.bin");
        let cipher = dir.path().join("c.bin");
        let data = b"content".repeat(100);
        std::fs::write(&plain, &data).unwrap();
        encrypt_file_stream(&roundtrip_key(), &plain, &cipher).unwrap();

        assert_eq!(
            read_file_decrypted(&roundtrip_key(), &plain, 10_000).unwrap(),
            data
        );
        assert_eq!(
            read_file_decrypted(&roundtrip_key(), &cipher, 10_000).unwrap(),
            data
        );
        // 超限拒绝
        assert!(read_file_decrypted(&roundtrip_key(), &cipher, 10).is_err());
    }

    #[test]
    fn test_wrong_key_fails() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("a.bin");
        let enc = dir.path().join("a.bin.enc");
        let dst = dir.path().join("a.out");
        std::fs::write(&src, b"secret").unwrap();
        encrypt_file_stream(&roundtrip_key(), &src, &enc).unwrap();

        let wrong = [0x99u8; 32];
        assert!(copy_decrypt_file(&wrong, &enc, &dst).is_err());
    }
}
