//! Tax Profile — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 汇总税务居民身份、税号、收入来源等申报基础数据。
//! ⚠️ 本插件仅做数据整理，不提供税务建议。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_error, log_info, send_result_json, truncate};
use solosoul_plugin_sdk::{escape_json};

/// 税务数据
struct TaxData {
    name: String,
    dob: String,
    residence_country: String,
    tax_id: String,
    tax_country: String,
    employer: String,
    position: String,
    income_source: String,
    address: String,
}

#[cfg(not(test))]
fn read_field(path: &str) -> String {
    get_field(path).unwrap_or_default().trim().to_string()
}

#[cfg(not(test))]
fn read_tax_data() -> TaxData {
    TaxData {
        name: read_field("identity.fullName"),
        dob: read_field("identity.dateOfBirth"),
        residence_country: read_field("address.country"),
        tax_id: read_field("taxId.number"),
        tax_country: read_field("taxId.country"),
        employer: read_field("employment.company"),
        position: read_field("employment.position"),
        income_source: read_field("employment.incomeSource"),
        address: format_address(),
    }
}

#[cfg(not(test))]
fn format_address() -> String {
    let street = read_field("address.street");
    let city = read_field("address.city");
    let state = read_field("address.state");
    let postal = read_field("address.postalCode");
    let country = read_field("address.country");

    let mut parts = Vec::new();
    if !street.is_empty() { parts.push(street); }
    if !city.is_empty() { parts.push(city); }
    if !state.is_empty() { parts.push(state); }
    if !postal.is_empty() { parts.push(postal); }
    if !country.is_empty() { parts.push(country); }
    parts.join(", ")
}

/// 推断税务居民国（如未单独设置）
fn infer_tax_country(tax_country: &str, residence: &str) -> String {
    if !tax_country.is_empty() {
        tax_country.to_string()
    } else {
        residence.to_string()
    }
}

/// 生成税务摘要报告
fn generate_report(data: &TaxData) -> String {
    let tax_country = infer_tax_country(&data.tax_country, &data.residence_country);

    let mut lines = Vec::new();
    lines.push("╔══════════════════════════════════════╗".to_string());
    lines.push("║         🧾 TAX PROFILE               ║".to_string());
    lines.push("╠══════════════════════════════════════╣".to_string());

    if !data.name.is_empty() {
        lines.push(format!("║ 纳税人: {:<29} ║", truncate(&data.name, 29)));
    }
    if !data.dob.is_empty() {
        lines.push(format!("║ 出生日期: {:<27} ║", truncate(&data.dob, 27)));
    }

    lines.push("╠══════════════════════════════════════╣".to_string());

    lines.push(format!("║ 税务居民国: {:<25} ║", truncate(&tax_country, 25)));
    if !data.tax_id.is_empty() {
        lines.push(format!("║ 税号: {:<31} ║", mask_sensitive(&data.tax_id)));
    }

    if !data.address.is_empty() {
        lines.push(format!("║ 住址: {:<31} ║", truncate(&data.address, 31)));
    }

    lines.push("╠══════════════════════════════════════╣".to_string());

    if !data.employer.is_empty() {
        lines.push(format!("║ 雇主: {:<31} ║", truncate(&data.employer, 31)));
    }
    if !data.position.is_empty() {
        lines.push(format!("║ 职位: {:<31} ║", truncate(&data.position, 31)));
    }
    if !data.income_source.is_empty() {
        lines.push(format!("║ 收入来源: {:<27} ║", truncate(&data.income_source, 27)));
    }

    lines.push("╚══════════════════════════════════════╝".to_string());
    lines.push(String::new());
    lines.push("⚠️ 本插件仅整理数据，不构成税务建议。".to_string());
    lines.push("   请咨询专业税务顾问。".to_string());

    lines.join("\n")
}

/// 脱敏显示（保留前3后2）
fn mask_sensitive(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 6 {
        s.to_string()
    } else {
        let prefix: String = chars[..3].iter().collect();
        let suffix: String = chars[chars.len() - 2..].iter().collect();
        format!("{}****{}", prefix, suffix)
    }
}



/// 简单的 JSON 字符串转义


#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Tax Profile 启动 — 汇总税务档案");

    let residence = read_field("address.country");
    if residence.is_empty() {
        log_error("缺少必需字段: address.country");
        return -1;
    }

    let data = read_tax_data();
    let report = generate_report(&data);
    for line in report.lines() {
        log_info(line);
    }

    // Phase 2: 结构化结果
    let tax_country = infer_tax_country(&data.tax_country, &data.residence_country);
    let mut pairs: Vec<(&str, String)> = Vec::new();
    if !data.name.is_empty() { pairs.push(("纳税人", data.name.clone())); }
    if !data.dob.is_empty() { pairs.push(("出生日期", data.dob.clone())); }
    pairs.push(("税务居民国", tax_country.clone()));
    if !data.tax_id.is_empty() { pairs.push(("税号", mask_sensitive(&data.tax_id))); }
    if !data.address.is_empty() { pairs.push(("住址", data.address.clone())); }
    if !data.employer.is_empty() { pairs.push(("雇主", data.employer.clone())); }
    if !data.position.is_empty() { pairs.push(("职位", data.position.clone())); }
    if !data.income_source.is_empty() { pairs.push(("收入来源", data.income_source.clone())); }

    let pairs_json: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(k), escape_json(v)))
        .collect();

    let result_json = format!(
        r#"{{"type":"key_value","title":"税务档案","pairs":[{}],"text":"{}"}}"#,
        pairs_json.join(","),
        escape_json(&report)
    );
    let _ = send_result_json(&result_json);

    0
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_tax_country() {
        assert_eq!(infer_tax_country("美国", "中国"), "美国");
        assert_eq!(infer_tax_country("", "中国"), "中国");
    }

    #[test]
    fn test_mask_sensitive() {
        assert_eq!(mask_sensitive("1234567890"), "123****90");
        assert_eq!(mask_sensitive("abc"), "abc");
    }

    #[test]
    fn test_generate_report() {
        let data = TaxData {
            name: "张三".to_string(),
            dob: "1990-01-01".to_string(),
            residence_country: "中国".to_string(),
            tax_id: "110101199001011234".to_string(),
            tax_country: "".to_string(),
            employer: "Example Tech".to_string(),
            position: "工程师".to_string(),
            income_source: "工资薪金".to_string(),
            address: "北京市海淀区".to_string(),
        };
        let report = generate_report(&data);
        assert!(report.contains("TAX PROFILE"));
        assert!(report.contains("张三"));
        assert!(report.contains("中国"));
        assert!(report.contains("Example Tech"));
        assert!(report.contains("工资薪金"));
        assert!(report.contains("110****34")); // 脱敏税号
    }

    #[test]
    fn test_generate_report_minimal() {
        let data = TaxData {
            name: String::new(),
            dob: String::new(),
            residence_country: "新加坡".to_string(),
            tax_id: String::new(),
            tax_country: String::new(),
            employer: String::new(),
            position: String::new(),
            income_source: String::new(),
            address: String::new(),
        };
        let report = generate_report(&data);
        assert!(report.contains("新加坡"));
    }
}
