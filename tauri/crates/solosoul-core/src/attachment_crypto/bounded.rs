//! 同一文件句柄上的有界附件预览读取。
use solosoul_crypto::cipher::{decrypt_chunked_stream, CipherError};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// 同一打开句柄上的有界读取；只有 SOLC 密文才向宿主索取密钥。
pub fn read_file_with_attachment_key(
    path: &Path,
    max_size: u64,
    key_provider: impl FnOnce() -> Result<[u8; 32], String>,
) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| format!("打开文件失败: {error}"))?;
    let size = file
        .metadata()
        .map_err(|error| format!("读取文件元数据失败: {error}"))?
        .len();
    read_attachment_stream(file, size, max_size, key_provider)
}

fn preview_too_large(limit: u64) -> String {
    format!("File too large (max {limit} bytes)")
}

fn read_attachment_stream(
    mut reader: impl Read,
    advertised_size: u64,
    max_size: u64,
    key_provider: impl FnOnce() -> Result<[u8; 32], String>,
) -> Result<Vec<u8>, String> {
    // SOLC v2: 25 字节头，每个 64 KiB 明文块增加 16 字节认证标签。
    let cipher_limit = max_size
        .div_ceil(64 * 1024)
        .checked_mul(16)
        .and_then(|overhead| max_size.checked_add(overhead))
        .and_then(|size| size.checked_add(25))
        .ok_or("Invalid attachment read limit")?;
    let cipher_read_limit = cipher_limit
        .checked_add(1)
        .ok_or("Invalid attachment read limit")?;
    let plain_read_limit = max_size
        .checked_add(1)
        .ok_or("Invalid attachment read limit")?;
    if advertised_size > cipher_limit {
        return Err(preview_too_large(max_size));
    }
    let mut prefix = [0u8; 4];
    let mut length = 0;
    while length < prefix.len() {
        match reader.read(&mut prefix[length..]) {
            Ok(0) => break,
            Ok(count) => length += count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("读取文件失败: {error}")),
        }
    }
    let encrypted = length == prefix.len() && prefix == *b"SOLC";
    let source = (&prefix[..length]).chain(reader);
    if !encrypted {
        if advertised_size > max_size {
            return Err(preview_too_large(max_size));
        }
        let mut bytes = Vec::new();
        source
            .take(plain_read_limit)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("读取文件失败: {error}"))?;
        if bytes.len() as u64 > max_size {
            return Err(preview_too_large(max_size));
        }
        return Ok(bytes);
    }
    let key = key_provider()?;
    let mut input = source.take(cipher_read_limit);
    let mut output = BoundedPreviewOutput {
        bytes: Vec::new(),
        limit: max_size,
    };
    decrypt_chunked_stream(&key, &mut input, &mut output).map_err(|error| error.to_string())?;
    // 拒绝超限输入与未消费尾随字节，包含空分块流，不能把截断的输入当成完整文件。
    let mut trailing = Vec::new();
    input
        .by_ref()
        .take(1)
        .read_to_end(&mut trailing)
        .map_err(|error| format!("读取文件失败: {error}"))?;
    if input.limit() == 0 {
        return Err(preview_too_large(max_size));
    }
    if !trailing.is_empty() {
        return Err(CipherError::InvalidCiphertext.to_string());
    }
    Ok(output.bytes)
}

struct BoundedPreviewOutput {
    bytes: Vec<u8>,
    limit: u64,
}

impl Write for BoundedPreviewOutput {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let next = self.bytes.len().checked_add(buffer.len()).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                preview_too_large(self.limit),
            )
        })?;
        if next as u64 > self.limit {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                preview_too_large(self.limit),
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
