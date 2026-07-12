//! Digital Will — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 基于 Vault 数据生成紧急情况下的资产与账户处理建议。
//! ⚠️ 本插件输出仅供个人参考，不具有法律效力。请咨询专业律师制定正式遗嘱。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_info, send_result_json};
use solosoul_plugin_sdk::{escape_json};

#[cfg(not(test))]
fn read_field(path: &str) -> String {
    get_field(path).unwrap_or_default().trim().to_string()
}

/// 生成数字遗产指示
fn generate_will(
    name: &str,
    emer_name: &str,
    emer_phone: &str,
    emer_rel: &str,
    address: &str,
    bank: &str,
    investment: &str,
    insurance: &str,
    digital_email: &str,
    digital_social: &str,
    digital_crypto: &str,
) -> String {
    let mut lines = Vec::new();

    lines.push("═══════════════════════════════════════".to_string());
    lines.push("         📜 DIGITAL WILL".to_string());
    lines.push("═══════════════════════════════════════".to_string());
    lines.push(String::new());
    lines.push(format!("立嘱人: {}", name));
    if !address.is_empty() {
        lines.push(format!("住址: {}", address));
    }
    lines.push(String::new());
    lines.push("───────────────────────────────────────".to_string());
    lines.push("一、紧急联系人".to_string());
    lines.push("───────────────────────────────────────".to_string());
    if !emer_name.is_empty() {
        lines.push(format!("姓名: {}", emer_name));
        if !emer_rel.is_empty() {
            lines.push(format!("关系: {}", emer_rel));
        }
        if !emer_phone.is_empty() {
            lines.push(format!("电话: {}", emer_phone));
        }
    } else {
        lines.push("（未设置紧急联系人，请补充）".to_string());
    }
    lines.push(String::new());

    lines.push("───────────────────────────────────────".to_string());
    lines.push("二、资产概览".to_string());
    lines.push("───────────────────────────────────────".to_string());
    let has_assets = !bank.is_empty() || !investment.is_empty() || !insurance.is_empty();
    if has_assets {
        if !bank.is_empty() {
            lines.push(format!("• 银行账户/流水: {}", bank));
        }
        if !investment.is_empty() {
            lines.push(format!("• 投资/理财: {}", investment));
        }
        if !insurance.is_empty() {
            lines.push(format!("• 人寿保险: {}", insurance));
        }
        lines.push(String::new());
        lines.push("建议: 请向继承人提供上述资产的详细清单、".to_string());
        lines.push("      账户号码及对应的金融机构联系方式。".to_string());
    } else {
        lines.push("（Vault 中未记录资产信息）".to_string());
    }
    lines.push(String::new());

    lines.push("───────────────────────────────────────".to_string());
    lines.push("三、数字账户处理".to_string());
    lines.push("───────────────────────────────────────".to_string());
    let has_digital = !digital_email.is_empty() || !digital_social.is_empty() || !digital_crypto.is_empty();
    if has_digital {
        if !digital_email.is_empty() {
            lines.push(format!("• 主要邮箱: {}", digital_email));
        }
        if !digital_social.is_empty() {
            lines.push(format!("• 社交媒体: {}", digital_social));
        }
        if !digital_crypto.is_empty() {
            lines.push(format!("• 加密资产/钱包: {}", digital_crypto));
        }
        lines.push(String::new());
        lines.push("建议: 数字账户的访问凭证应单独保存于".to_string());
        lines.push("      安全的密码管理器或物理保险箱中，".to_string());
        lines.push("      并告知可信赖的继承人其位置。".to_string());
    } else {
        lines.push("（Vault 中未记录数字账户信息）".to_string());
    }
    lines.push(String::new());

    lines.push("───────────────────────────────────────".to_string());
    lines.push("四、重要提醒".to_string());
    lines.push("───────────────────────────────────────".to_string());
    lines.push("1. 本文件由 SoloSoul 自动生成，仅供个人参考。".to_string());
    lines.push("2. 不具有法律效力，不能替代正式遗嘱。".to_string());
    lines.push("3. 建议咨询专业律师，根据当地法律制定正式遗嘱。".to_string());
    lines.push("4. 定期更新 Vault 中的资产和联系人信息。".to_string());
    lines.push(String::new());
    lines.push("═══════════════════════════════════════".to_string());
    lines.push(format!("生成时间: {}", "（由插件运行时确定）"));
    lines.push("═══════════════════════════════════════".to_string());

    lines.join("\n")
}

/// 简单的 JSON 字符串转义


#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Digital Will 启动 — 生成数字遗产指示");

    let name = read_field("identity.fullName");
    let emer_name = read_field("contact.emergencyName");
    let emer_phone = read_field("contact.emergencyPhone");
    let emer_rel = read_field("contact.emergencyRelationship");

    let street = read_field("address.street");
    let city = read_field("address.city");
    let address = if !city.is_empty() {
        if !street.is_empty() {
            format!("{}, {}", street, city)
        } else {
            city
        }
    } else {
        street
    };

    let bank = read_field("financial.bankStatement");
    let investment = read_field("financial.investment");
    let insurance = read_field("insurance.life");
    let digital_email = read_field("digitalAccounts.email");
    let digital_social = read_field("digitalAccounts.socialMedia");
    let digital_crypto = read_field("digitalAccounts.crypto");

    let will = generate_will(
        &name, &emer_name, &emer_phone, &emer_rel, &address,
        &bank, &investment, &insurance,
        &digital_email, &digital_social, &digital_crypto,
    );

    for line in will.lines() {
        log_info(line);
    }

    // Phase 2: 结构化结果
    let mut pairs: Vec<(&str, String)> = Vec::new();
    if !name.is_empty() { pairs.push(("立嘱人", name.clone())); }
    if !emer_name.is_empty() { pairs.push(("紧急联系人", format!("{} ({}) {}", emer_name, emer_rel, emer_phone))); }
    if !address.is_empty() { pairs.push(("住址", address.clone())); }
    if !bank.is_empty() { pairs.push(("资产", format!("银行: {}", bank))); }
    if !investment.is_empty() { pairs.push(("投资", investment.clone())); }
    if !insurance.is_empty() { pairs.push(("保险", insurance.clone())); }
    if !digital_email.is_empty() || !digital_social.is_empty() || !digital_crypto.is_empty() {
        let mut accounts = Vec::new();
        if !digital_email.is_empty() { accounts.push(format!("邮箱: {}", digital_email)); }
        if !digital_social.is_empty() { accounts.push(format!("社交: {}", digital_social)); }
        if !digital_crypto.is_empty() { accounts.push(format!("加密: {}", digital_crypto)); }
        pairs.push(("数字账户", accounts.join(", ")));
    }
    let pairs_json: Vec<String> = pairs.iter().map(|(k, v)| format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(k), escape_json(v))).collect();
    let result_json = format!(r#"{{"type":"key_value","title":"数字遗嘱","pairs":[{}],"text":"{}"}}"#, pairs_json.join(","), escape_json(&will));
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
    fn test_generate_will_full() {
        let will = generate_will(
            "张三", "李四", "13800138000", "配偶",
            "北京市海淀区", "招商银行", "股票基金", "平安人寿",
            "zhangsan@example.com", "微信/微博", "BTC/ETH",
        );
        assert!(will.contains("DIGITAL WILL"));
        assert!(will.contains("张三"));
        assert!(will.contains("李四"));
        assert!(will.contains("招商银行"));
        assert!(will.contains("平安人寿"));
        assert!(will.contains("zhangsan@example.com"));
        assert!(will.contains("BTC/ETH"));
        assert!(will.contains("不具有法律效力"));
    }

    #[test]
    fn test_generate_will_minimal() {
        let will = generate_will(
            "Test", "", "", "", "", "", "", "", "", "", "",
        );
        assert!(will.contains("Test"));
        assert!(will.contains("未设置紧急联系人"));
    }
}
