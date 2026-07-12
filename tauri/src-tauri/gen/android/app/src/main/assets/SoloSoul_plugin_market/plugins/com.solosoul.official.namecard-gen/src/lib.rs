//! Namecard Generator — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 从 Vault 联系信息生成标准 vCard 3.0 数字名片。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_info, send_result_json, truncate};
use solosoul_plugin_sdk::{escape_json};

/// vCard 3.0 字段转义（逗号、分号、反斜杠）
fn vcard_escape(s: &str) -> String {
    s.replace("\\", "\\\\")
        .replace(";", "\\;")
        .replace(",", "\\,")
        .replace("\n", "\\n")
}

/// 将全名拆分为姓和名（简化处理）
fn split_name(full_name: &str) -> (String, String) {
    let parts: Vec<&str> = full_name.trim().split_whitespace().collect();
    if parts.len() >= 2 {
        // 假设最后一部分是姓，前面的是名
        let given = parts[..parts.len() - 1].join(" ");
        let family = parts[parts.len() - 1].to_string();
        (family, given)
    } else {
        (String::new(), full_name.to_string())
    }
}

/// 生成 vCard 3.0
fn generate_vcard(
    full_name: &str,
    email: &str,
    phone: &str,
    website: &str,
    title: &str,
    org: &str,
    street: &str,
    city: &str,
    state: &str,
    postal: &str,
    country: &str,
) -> String {
    let (family, given) = split_name(full_name);
    let mut lines = Vec::new();

    lines.push("BEGIN:VCARD".to_string());
    lines.push("VERSION:3.0".to_string());
    lines.push(format!(
        "N:{};{};;;",
        vcard_escape(&family),
        vcard_escape(&given)
    ));
    lines.push(format!("FN:{}", vcard_escape(full_name)));

    if !org.is_empty() {
        lines.push(format!("ORG:{}", vcard_escape(org)));
    }
    if !title.is_empty() {
        lines.push(format!("TITLE:{}", vcard_escape(title)));
    }
    if !email.is_empty() {
        lines.push(format!("EMAIL;TYPE=INTERNET:{}", vcard_escape(email)));
    }
    if !phone.is_empty() {
        lines.push(format!("TEL;TYPE=CELL:{}", vcard_escape(phone)));
    }
    if !website.is_empty() {
        lines.push(format!("URL:{}", vcard_escape(website)));
    }

    // 地址
    let has_address = !street.is_empty() || !city.is_empty() || !state.is_empty()
        || !postal.is_empty() || !country.is_empty();
    if has_address {
        lines.push(format!(
            "ADR;TYPE=WORK:;;{};{};{};{};{}",
            vcard_escape(street),
            vcard_escape(city),
            vcard_escape(state),
            vcard_escape(postal),
            vcard_escape(country)
        ));
    }

    lines.push("END:VCARD".to_string());

    lines.join("\n")
}

/// 生成文本名片（人类可读）
fn generate_text_card(
    full_name: &str,
    email: &str,
    phone: &str,
    website: &str,
    title: &str,
    org: &str,
) -> String {
    let mut lines = Vec::new();
    lines.push("╔══════════════════════════════════════╗".to_string());
    lines.push("║         🪪 DIGITAL NAMECARD          ║".to_string());
    lines.push("╠══════════════════════════════════════╣".to_string());
    lines.push(format!("║ {:<36} ║", truncate(full_name, 36)));

    if !title.is_empty() || !org.is_empty() {
        let title_org = if !title.is_empty() && !org.is_empty() {
            format!("{} @ {}", title, org)
        } else if !title.is_empty() {
            title.to_string()
        } else {
            org.to_string()
        };
        lines.push(format!("║ {:<36} ║", truncate(&title_org, 36)));
    }

    lines.push("╠══════════════════════════════════════╣".to_string());

    if !phone.is_empty() {
        lines.push(format!("║ 📱 {:<33} ║", truncate(phone, 33)));
    }
    if !email.is_empty() {
        lines.push(format!("║ 📧 {:<33} ║", truncate(email, 33)));
    }
    if !website.is_empty() {
        lines.push(format!("║ 🌐 {:<33} ║", truncate(website, 33)));
    }

    lines.push("╚══════════════════════════════════════╝".to_string());
    lines.join("\n")
}

/// 简单的 JSON 字符串转义




#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Namecard Generator 启动 — 生成数字名片");

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

    let text_card = generate_text_card(&name, &email, &phone, &website, &title, &org);
    let vcard = generate_vcard(&name, &email, &phone, &website, &title, &org, &street, &city, &state, &postal, &country);

    // Phase 2: 结构化结果
    let mut pairs: Vec<(String, String)> = Vec::new();
    if !name.is_empty() { pairs.push(("姓名".to_string(), name.clone())); }
    if !title.is_empty() || !org.is_empty() {
        let title_org = if !title.is_empty() && !org.is_empty() {
            format!("{} @ {}", title, org)
        } else if !title.is_empty() {
            title.clone()
        } else {
            org.clone()
        };
        pairs.push(("职位/组织".to_string(), title_org));
    }
    if !phone.is_empty() { pairs.push(("电话".to_string(), phone.clone())); }
    if !email.is_empty() { pairs.push(("邮箱".to_string(), email.clone())); }
    if !website.is_empty() { pairs.push(("网站".to_string(), website.clone())); }

    let pairs_json: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(k), escape_json(v)))
        .collect();

    let result_json = format!(
        r#"{{"type":"key_value","title":"数字名片","pairs":[{}],"text":"{}"}}"#,
        pairs_json.join(","),
        escape_json(&vcard)
    );
    let _ = send_result_json(&result_json);

    // 同时保留日志输出
    log_info("【文本名片】");
    for line in text_card.lines() {
        log_info(line);
    }

    log_info("【vCard 3.0】");
    for line in vcard.lines() {
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
    fn test_vcard_escape() {
        assert_eq!(vcard_escape("a;b,c\\d"), "a\\;b\\,c\\\\d");
        assert_eq!(vcard_escape("hello"), "hello");
    }

    #[test]
    fn test_split_name() {
        assert_eq!(split_name("San Zhang"), ("Zhang".to_string(), "San".to_string()));
        assert_eq!(split_name("Zhang"), (String::new(), "Zhang".to_string()));
        assert_eq!(split_name("John Paul Smith"), ("Smith".to_string(), "John Paul".to_string()));
    }

    #[test]
    fn test_generate_vcard() {
        let vcard = generate_vcard(
            "San Zhang", "san@example.com", "+8613800138000",
            "https://example.com", "Engineer", "Example Inc",
            "1 Main St", "Beijing", "", "100000", "China",
        );
        assert!(vcard.contains("BEGIN:VCARD"));
        assert!(vcard.contains("VERSION:3.0"));
        assert!(vcard.contains("FN:San Zhang"));
        assert!(vcard.contains("EMAIL;TYPE=INTERNET:san@example.com"));
        assert!(vcard.contains("TEL;TYPE=CELL:+8613800138000"));
        assert!(vcard.contains("ORG:Example Inc"));
        assert!(vcard.contains("TITLE:Engineer"));
        assert!(vcard.contains("END:VCARD"));
    }

    #[test]
    fn test_generate_text_card() {
        let card = generate_text_card(
            "张三", "zhangsan@example.com", "13800138000",
            "", "高级工程师", "Example Inc",
        );
        assert!(card.contains("DIGITAL NAMECARD"));
        assert!(card.contains("张三"));
        assert!(card.contains("13800138000"));
        assert!(card.contains("高级工程师 @ Example Inc"));
    }
}
