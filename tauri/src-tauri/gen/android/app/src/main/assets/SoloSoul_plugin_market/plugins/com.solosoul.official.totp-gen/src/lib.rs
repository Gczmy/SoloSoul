//! TOTP Generator — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 基于 Vault 中存储的 Base32 编码 TOTP Secret，按 RFC 6238 生成 6 位动态验证码。

use hmac::{Hmac, Mac};
use sha1::Sha1;
use solosoul_plugin_sdk::{get_field, get_timestamp, log_error, log_info, send_result_json, escape_json};

/// HMAC-SHA1 类型别名
type HmacSha1 = Hmac<Sha1>;

/// Base32 字母表（RFC 4648）
const BASE32_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// 解码 Base32 字符串（RFC 4648，不含填充）
fn base32_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    let cleaned: String = input.to_uppercase().chars().filter(|c| *c != '=').collect();
    if cleaned.is_empty() {
        return Ok(Vec::new());
    }

    let mut bits = 0u32;
    let mut bit_count = 0u8;
    let mut result = Vec::with_capacity(cleaned.len() * 5 / 8);

    for c in cleaned.chars() {
        let val = BASE32_ALPHABET
            .iter()
            .position(|&b| b as char == c)
            .ok_or("Invalid Base32 character")? as u32;

        bits = (bits << 5) | val;
        bit_count += 5;

        if bit_count >= 8 {
            bit_count -= 8;
            result.push((bits >> bit_count) as u8);
            bits &= (1u32 << bit_count) - 1;
        }
    }

    Ok(result)
}

/// 计算 HMAC-SHA1
fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    let mut mac = HmacSha1::new_from_slice(key).expect("HMAC key length valid");
    mac.update(message);
    let result = mac.finalize();
    let bytes = result.into_bytes();
    let mut output = [0u8; 20];
    output.copy_from_slice(&bytes);
    output
}

/// RFC 6238 TOTP 核心算法
///
/// # 参数
/// - `secret`: Base32 编码的共享密钥
/// - `timestamp_ms`: Unix 时间戳（毫秒）
/// - `time_step`: 时间步长（秒），默认 30
/// - `digits`: 验证码位数，默认 6
///
/// # 返回
/// (otp_code, seconds_remaining)
fn generate_totp(
    secret: &str,
    timestamp_ms: i64,
    time_step: u64,
    digits: u32,
) -> Result<(u32, u64), &'static str> {
    let secret_bytes = base32_decode(secret)?;

    let timestamp_sec = (timestamp_ms / 1000) as u64;
    let counter = timestamp_sec / time_step;
    let seconds_remaining = time_step - (timestamp_sec % time_step);

    // Counter 编码为 8 字节大端序
    let counter_bytes = counter.to_be_bytes();

    // HMAC-SHA1
    let hash = hmac_sha1(&secret_bytes, &counter_bytes);

    // Dynamic truncation (RFC 4226)
    let offset = (hash[19] & 0x0f) as usize;
    let binary_code = ((hash[offset] as u32 & 0x7f) << 24)
        | ((hash[offset + 1] as u32) << 16)
        | ((hash[offset + 2] as u32) << 8)
        | (hash[offset + 3] as u32);

    // 取模得到指定位数的 OTP
    let otp = binary_code % 10u32.pow(digits);

    Ok((otp, seconds_remaining))
}

/// 格式化 OTP 为固定长度字符串（前导零）
fn format_otp(otp: u32, digits: u32) -> String {
    format!("{:0width$}", otp, width = digits as usize)
}

/// 简单的 JSON 字符串转义


/// 插件入口
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("TOTP Generator 启动 — 生成动态验证码");

    let secret = match get_field("security.totpSecret") {
        Ok(s) if !s.trim().is_empty() => s.trim().to_uppercase(),
        Ok(_) => {
            log_error("TOTP Secret 为空");
            let result_json = r#"{"type":"text","status":"empty_totp_secret","content":"TOTP Secret 为空。请在 Vault 的「安全信息」中填写 TOTP 密钥。"}"#;
            let _ = send_result_json(result_json);
            return 0;
        }
        Err(e) => {
            log_error(&format!("获取 TOTP Secret 失败: {:?}", e));
            let result_json = r#"{"type":"text","status":"no_totp_secret","content":"未找到 TOTP 密钥。请在 Vault 中创建「安全信息」分区，并添加 totpSecret 字段。"}"#;
            let _ = send_result_json(result_json);
            return 0;
        }
    };

    let issuer = get_field("security.totpIssuer").unwrap_or_default();
    let account = get_field("security.totpAccount").unwrap_or_default();

    let now = get_timestamp();

    match generate_totp(&secret, now, 30, 6) {
        Ok((otp, remaining)) => {
            let otp_str = format_otp(otp, 6);
            let label = if !issuer.is_empty() && !account.is_empty() {
                format!("{} ({})", issuer, account)
            } else if !issuer.is_empty() {
                issuer
            } else if !account.is_empty() {
                account
            } else {
                "TOTP".to_string()
            };

            log_info(&format!("验证码: {}", otp_str));
            log_info(&format!("标签: {}", label));
            log_info(&format!("剩余有效时间: {} 秒", remaining));

            if remaining <= 5 {
                log_info("⚠️ 验证码即将过期，请等待下一轮");
            }

            // Phase 2: 结构化结果
            let pairs_json = vec![
                format!(r#"{{"key":"标签","value":"{}"}}"#, escape_json(&label)),
                format!(r#"{{"key":"验证码","value":"{}"}}"#, escape_json(&otp_str)),
                format!(r#"{{"key":"剩余时间","value":"{} 秒"}}"#, remaining),
            ];
            let result_json = format!(
                r#"{{"type":"key_value","title":"TOTP 动态验证码","pairs":[{}]}}"#,
                pairs_json.join(",")
            );
            let _ = send_result_json(&result_json);

            0
        }
        Err(e) => {
            log_error(&format!("TOTP 生成失败: {}", e));
            -3
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base32_decode_basic() {
        // RFC 4648 测试向量
        assert_eq!(base32_decode("").unwrap(), b"");
        assert_eq!(base32_decode("MY======").unwrap(), b"f");
        assert_eq!(base32_decode("MZXQ====").unwrap(), b"fo");
        assert_eq!(base32_decode("MZXW6===").unwrap(), b"foo");
        assert_eq!(base32_decode("MZXW6YQ=").unwrap(), b"foob");
        assert_eq!(base32_decode("MZXW6YTB").unwrap(), b"fooba");
        assert_eq!(base32_decode("MZXW6YTBOI======").unwrap(), b"foobar");
    }

    #[test]
    fn test_base32_decode_without_padding() {
        assert_eq!(base32_decode("MZXW6YTB").unwrap(), b"fooba");
        assert_eq!(base32_decode("MZXW6YTBOI").unwrap(), b"foobar");
    }

    #[test]
    fn test_base32_decode_invalid_char() {
        assert!(base32_decode("MZXW1").is_err()); // '1' not in Base32
    }

    #[test]
    fn test_hmac_sha1() {
        // RFC 2202 Test Case 1
        let key = b"\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b";
        let data = b"Hi There";
        let result = hmac_sha1(key, data);
        let expected: [u8; 20] = [
            0xb6, 0x17, 0x31, 0x86, 0x55, 0x05, 0x72, 0x64,
            0xe2, 0x8b, 0xc0, 0xb6, 0xfb, 0x37, 0x8c, 0x8e,
            0xf1, 0x46, 0xbe, 0x00,
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_totp_rfc6238_test_vector() {
        // RFC 6238 Appendix B Test Vectors
        // Secret = "12345678901234567890" (20 bytes ASCII)
        // Base32 encoded: "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

        // Time = 59, counter = 1 → expected = 94287082 (SHA1, 8 digits)
        let (otp, _) = generate_totp(secret, 59_000, 30, 8).unwrap();
        assert_eq!(format_otp(otp, 8), "94287082");

        // Time = 1111111109, counter = 37037036 → expected = 07081804
        let (otp, _) = generate_totp(secret, 1_111_111_109_000, 30, 8).unwrap();
        assert_eq!(format_otp(otp, 8), "07081804");
    }

    #[test]
    fn test_generate_totp_6_digits() {
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        // Same secret, 6 digits: 94287082 % 1000000 = 287082
        let (otp, _) = generate_totp(secret, 59_000, 30, 6).unwrap();
        assert_eq!(format_otp(otp, 6), "287082");
    }

    #[test]
    fn test_format_otp() {
        assert_eq!(format_otp(123456, 6), "123456");
        assert_eq!(format_otp(42, 6), "000042");
        assert_eq!(format_otp(0, 6), "000000");
    }

    #[test]
    fn test_seconds_remaining() {
        // timestamp = 60_000 (60s), step = 30 → remaining = 30 - (60 % 30) = 30? No...
        // 60 / 30 = 2, 60 % 30 = 0, remaining = 30 - 0 = 30
        let (_, remaining) = generate_totp("GEZDGNBVGY3TQOJQ", 60_000, 30, 6).unwrap();
        assert_eq!(remaining, 30);

        // timestamp = 75_000 (75s), 75 % 30 = 15, remaining = 30 - 15 = 15
        let (_, remaining) = generate_totp("GEZDGNBVGY3TQOJQ", 75_000, 30, 6).unwrap();
        assert_eq!(remaining, 15);
    }
}
