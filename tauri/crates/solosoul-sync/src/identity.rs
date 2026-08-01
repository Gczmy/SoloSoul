//! 同步身份相关的共享工具（桌面与移动端共用）。
//!
//! `sha256_hex_short` 用于在 mDNS/NSD TXT 记录中广播 account_id 的哈希值，
//! 避免局域网内明文泄露原始账户标识；桌面端广播 account_hash，移动端广播明文
//! account_id（由 NSD TXT 限制），发现层统一用它做账户过滤。

use sha2::{Digest, Sha256};

/// 计算 SHA-256 哈希并返回前 16 字节的 hex 编码（32 字符）。
/// 截断为 16 字节（128 位）在局域网发现场景下碰撞风险极低且不影响安全性
///（真正的身份验证在 Noise 握手阶段完成）。
///
/// 公开供同步引擎广播/过滤与 GUI 发现层（discovery.rs）复用。
pub fn sha256_hex_short(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(&hasher.finalize()[..16])
}
