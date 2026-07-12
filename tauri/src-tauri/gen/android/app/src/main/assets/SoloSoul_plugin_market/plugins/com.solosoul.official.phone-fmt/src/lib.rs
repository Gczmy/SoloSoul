//! Phone Formatter — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 按国家规范格式化电话号码。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_error, log_info, send_result_json};
use solosoul_plugin_sdk::{escape_json};

/// 提取纯数字
fn digits_only(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// 检测国家并格式化
fn format_phone(phone: &str, country_hint: &str) -> Option<String> {
    let digits = digits_only(phone);
    if digits.is_empty() {
        return None;
    }

    // 根据国家提示或号码前缀判断
    let country = if !country_hint.is_empty() {
        normalize_country(country_hint)
    } else {
        detect_country_by_prefix(&digits)
    };

    match country {
        "CN" => format_china(&digits),
        "US" | "CA" => format_us(&digits),
        "GB" => format_gb(&digits),
        "JP" => format_jp(&digits),
        "DE" => format_de(&digits),
        "FR" => format_fr(&digits),
        "KR" => format_kr(&digits),
        _ => format_generic(&digits),
    }
}

fn normalize_country(code: &str) -> &'static str {
    let c = code.to_uppercase();
    match c.as_str() {
        "CN" | "CHN" | "中国" | "CHINA" => "CN",
        "US" | "USA" | "美国" | "UNITED STATES" => "US",
        "GB" | "GBR" | "UK" | "英国" | "UNITED KINGDOM" => "GB",
        "JP" | "JPN" | "日本" | "JAPAN" => "JP",
        "DE" | "DEU" | "德国" | "GERMANY" => "DE",
        "FR" | "FRA" | "法国" | "FRANCE" => "FR",
        "KR" | "KOR" | "韩国" | "SOUTH KOREA" => "KR",
        _ => "UNKNOWN",
    }
}

fn detect_country_by_prefix(digits: &str) -> &'static str {
    if digits.starts_with("86") && digits.len() >= 11 {
        "CN"
    } else if digits.starts_with("1") && digits.len() == 11 {
        "US"
    } else if digits.starts_with("44") {
        "GB"
    } else if digits.starts_with("81") {
        "JP"
    } else if digits.starts_with("49") {
        "DE"
    } else if digits.starts_with("33") {
        "FR"
    } else if digits.starts_with("82") {
        "KR"
    } else {
        "UNKNOWN"
    }
}

/// 中国：+86 138 0013 8000 或 138-0013-8000
fn format_china(digits: &str) -> Option<String> {
    let d = if digits.starts_with('1') && digits.len() == 11 {
        digits.to_string()
    } else if digits.starts_with("86") && digits.len() == 13 {
        digits[2..].to_string()
    } else {
        return None;
    };
    Some(format!("+86 {} {} {}", &d[..3], &d[3..7], &d[7..11]))
}

/// 美国/加拿大：(555) 123-4567
fn format_us(digits: &str) -> Option<String> {
    let d = if digits.len() == 10 {
        digits.to_string()
    } else if digits.len() == 11 && digits.starts_with('1') {
        digits[1..].to_string()
    } else {
        return None;
    };
    Some(format!("({}) {}-{}", &d[..3], &d[3..6], &d[6..10]))
}

/// 英国：+44 20 7946 0958
fn format_gb(digits: &str) -> Option<String> {
    let d = if digits.starts_with("44") {
        digits[2..].to_string()
    } else {
        digits.to_string()
    };
    if d.len() < 10 { return None; }
    Some(format!("+44 {}", &d))
}

/// 日本：+81 90-1234-5678
fn format_jp(digits: &str) -> Option<String> {
    let d = if digits.starts_with("81") {
        digits[2..].to_string()
    } else {
        digits.to_string()
    };
    if d.len() < 10 { return None; }
    Some(format!("+81 {}-{}-{}", &d[..2], &d[2..6], &d[6..10]))
}

/// 德国：+49 170 1234567
fn format_de(digits: &str) -> Option<String> {
    let d = if digits.starts_with("49") {
        digits[2..].to_string()
    } else {
        digits.to_string()
    };
    if d.len() < 10 { return None; }
    Some(format!("+49 {} {}", &d[..3], &d[3..]))
}

/// 法国：+33 1 23 45 67 89
fn format_fr(digits: &str) -> Option<String> {
    let d = if digits.starts_with("33") {
        digits[2..].to_string()
    } else {
        digits.to_string()
    };
    if d.len() < 9 { return None; }
    Some(format!("+33 {}", d.chars().collect::<Vec<_>>().chunks(2).map(|c| c.iter().collect::<String>()).collect::<Vec<_>>().join(" ")))
}

/// 韩国：+82 10-1234-5678
fn format_kr(digits: &str) -> Option<String> {
    let d = if digits.starts_with("82") {
        digits[2..].to_string()
    } else {
        digits.to_string()
    };
    if d.len() < 10 { return None; }
    Some(format!("+82 {}-{}-{}", &d[..2], &d[2..6], &d[6..10]))
}

/// 简单的 JSON 字符串转义


/// 通用：按 3-3-4 分组
fn format_generic(digits: &str) -> Option<String> {
    let chunks: Vec<String> = digits.chars().collect::<Vec<_>>().chunks(3).map(|c| c.iter().collect()).collect();
    Some(chunks.join(" "))
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Phone Formatter 启动 — 格式化电话号码");

    let phone = match get_field("contact.phone") {
        Ok(v) if !v.trim().is_empty() => v,
        Ok(_) => {
            log_error("电话号码为空");
            let result_json = r#"{"type":"text","status":"empty_phone","content":"电话号码为空。请检查 Vault 中的联系方式数据。"}"#;
            let _ = send_result_json(result_json);
            return 0;
        }
        Err(e) => {
            log_error(&format!("获取电话失败: {:?}", e));
            let result_json = r#"{"type":"text","status":"no_phone","content":"未找到电话号码。请在 Vault 的「联系方式」分区中添加电话（支持标签：手机、电话、Phone 等）。"}"#;
            let _ = send_result_json(result_json);
            return 0;
        }
    };

    let country = get_field("address.country").unwrap_or_default();

    match format_phone(&phone, &country) {
        Some(formatted) => {
            log_info(&format!("原始号码: {}", phone));
            log_info(&format!("格式化后: {}", formatted));

            // Phase 2: 结构化结果
            let pairs_json = vec![
                format!(r#"{{"key":"原始号码","value":"{}"}}"#, escape_json(&phone)),
                format!(r#"{{"key":"格式化后","value":"{}"}}"#, escape_json(&formatted)),
            ];
            let result_json = format!(
                r#"{{"type":"key_value","title":"电话号码格式化","pairs":[{}]}}"#,
                pairs_json.join(",")
            );
            let _ = send_result_json(&result_json);

            0
        }
        None => {
            log_error("无法识别电话号码格式");
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
    fn test_digits_only() {
        assert_eq!(digits_only("138-0013-8000"), "13800138000");
        assert_eq!(digits_only("+86 138 0013 8000"), "8613800138000");
    }

    #[test]
    fn test_format_china() {
        assert_eq!(format_china("13800138000"), Some("+86 138 0013 8000".to_string()));
        assert_eq!(format_china("8613800138000"), Some("+86 138 0013 8000".to_string()));
    }

    #[test]
    fn test_format_us() {
        assert_eq!(format_us("5551234567"), Some("(555) 123-4567".to_string()));
        assert_eq!(format_us("15551234567"), Some("(555) 123-4567".to_string()));
    }

    #[test]
    fn test_format_jp() {
        assert_eq!(format_jp("819012345678"), Some("+81 90-1234-5678".to_string()));
    }

    #[test]
    fn test_detect_country() {
        assert_eq!(detect_country_by_prefix("8613800138000"), "CN");
        assert_eq!(detect_country_by_prefix("15551234567"), "US");
        assert_eq!(detect_country_by_prefix("442079460958"), "GB");
    }

    #[test]
    fn test_format_phone_with_hint() {
        assert_eq!(format_phone("13800138000", "中国"), Some("+86 138 0013 8000".to_string()));
        assert_eq!(format_phone("5551234567", "美国"), Some("(555) 123-4567".to_string()));
    }
}
