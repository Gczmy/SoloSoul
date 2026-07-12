//! Contact Exporter — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 将 Vault 中的个人联系信息导出为 CSV 格式。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_info, send_result_json};
use solosoul_plugin_sdk::{escape_json};

/// 简单的 JSON 字符串转义


/// CSV 字段转义
fn csv_escape(s: &str) -> String {
    if s.contains('"') || s.contains(',') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// 生成 CSV 内容
fn generate_csv(data: &[(&str, &str)]) -> String {
    let mut lines = vec!["Field,Value".to_string()];
    for (field, value) in data {
        if !value.is_empty() {
            lines.push(format!("{}, {}", csv_escape(field), csv_escape(value)));
        }
    }
    lines.join("\n")
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Contact Exporter 启动 — 导出联系人 CSV");

    let name = get_field("identity.fullName").unwrap_or_default();
    let email = get_field("contact.email").unwrap_or_default();
    let phone = get_field("contact.phone").unwrap_or_default();
    let website = get_field("contact.website").unwrap_or_default();
    let title = get_field("identity.title").unwrap_or_default();
    let org = get_field("identity.organization").unwrap_or_default();
    let street = get_field("address.street").unwrap_or_default();
    let city = get_field("address.city").unwrap_or_default();
    let state = get_field("address.state").unwrap_or_default();
    let postal = get_field("address.postalCode").unwrap_or_default();
    let country = get_field("address.country").unwrap_or_default();

    let address = if !street.is_empty() || !city.is_empty() || !country.is_empty() {
        let mut parts = Vec::new();
        if !street.is_empty() { parts.push(street.clone()); }
        if !city.is_empty() { parts.push(city.clone()); }
        if !state.is_empty() { parts.push(state.clone()); }
        if !postal.is_empty() { parts.push(postal.clone()); }
        if !country.is_empty() { parts.push(country.clone()); }
        parts.join(", ")
    } else {
        String::new()
    };

    let data: Vec<(&str, &str)> = vec![
        ("Name", &name),
        ("Email", &email),
        ("Phone", &phone),
        ("Website", &website),
        ("Title", &title),
        ("Organization", &org),
        ("Address", &address),
    ];

    let csv = generate_csv(&data);

    // Phase 2: 发送结构化结果（key_value 卡片 + CSV 导出）
    let pairs_json: Vec<String> = data
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| {
            format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(k), escape_json(v))
        })
        .collect();

    let result_json = format!(
        r#"{{"type":"key_value","title":"联系人信息","pairs":[{}],"csv":"{}"}}"#,
        pairs_json.join(","),
        escape_json(&csv)
    );
    let _ = send_result_json(&result_json);

    // 同时保留日志输出供调试
    log_info("【CSV 输出】（可复制到 Excel/Numbers）");
    log_info("");
    for line in csv.lines() {
        log_info(line);
    }

    0
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_escape() {
        assert_eq!(csv_escape("hello"), "hello");
        assert_eq!(csv_escape("hello, world"), "\"hello, world\"");
        assert_eq!(csv_escape("he said \"hi\""), "\"he said \"\"hi\"\"\"");
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn test_generate_csv() {
        let data = vec![
            ("Name", "张三"),
            ("Email", "zhangsan@example.com"),
            ("Phone", ""),
        ];
        let csv = generate_csv(&data);
        assert!(csv.contains("Field,Value"));
        assert!(csv.contains("Name, 张三"));
        assert!(csv.contains("Email, zhangsan@example.com"));
        assert!(!csv.contains("Phone")); // empty value skipped
    }
}
