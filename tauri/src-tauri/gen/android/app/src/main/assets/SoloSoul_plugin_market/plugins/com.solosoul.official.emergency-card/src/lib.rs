//! Emergency Card — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 生成紧急情况下使用的医疗/联系信息卡片。

use solosoul_plugin_sdk::{get_field, log_error, log_info, send_result_json, escape_json, truncate};
/// 安全读取字段
fn read_field(path: &str) -> String {
    get_field(path).unwrap_or_default().trim().to_string()
}

/// 紧急联系卡数据
struct EmergencyData {
    name: String,
    dob: String,
    blood: String,
    allergies: String,
    medications: String,
    conditions: String,
    emer_name: String,
    emer_phone: String,
    emer_rel: String,
}

/// 从 Vault 读取紧急联系卡数据
fn read_emergency_data() -> EmergencyData {
    EmergencyData {
        name: read_field("identity.fullName"),
        dob: read_field("identity.dateOfBirth"),
        blood: read_field("medical.bloodType"),
        allergies: read_field("medical.allergies"),
        medications: read_field("medical.medications"),
        conditions: read_field("medical.conditions"),
        emer_name: read_field("contact.emergencyName"),
        emer_phone: read_field("contact.emergencyPhone"),
        emer_rel: read_field("contact.emergencyRelationship"),
    }
}

/// 生成紧急联系卡（纯函数，便于测试）
fn generate_card(data: &EmergencyData) -> String {
    let EmergencyData {
        name, dob, blood, allergies, medications, conditions,
        emer_name, emer_phone, emer_rel,
    } = data;

    let mut lines = Vec::new();

    // 标题
    lines.push("╔══════════════════════════════════════╗".to_string());
    lines.push("║         🚨 EMERGENCY CARD            ║".to_string());
    lines.push("╠══════════════════════════════════════╣".to_string());

    // 基本信息
    lines.push(format!("║ NAME:  {:<31} ║", truncate(&name, 31)));
    if !dob.is_empty() {
        lines.push(format!("║ DOB:   {:<31} ║", truncate(&dob, 31)));
    }
    if !blood.is_empty() {
        lines.push(format!("║ BLOOD: {:<31} ║", truncate(&blood, 31)));
    }

    lines.push("╠══════════════════════════════════════╣".to_string());

    // 医疗信息
    if !allergies.is_empty() {
        lines.push(format!("║ ALLERGIES: {:<27} ║", truncate(&allergies, 27)));
    }
    if !medications.is_empty() {
        lines.push(format!("║ MEDICATIONS: {:<25} ║", truncate(&medications, 25)));
    }
    if !conditions.is_empty() {
        lines.push(format!("║ CONDITIONS: {:<26} ║", truncate(&conditions, 26)));
    }

    if allergies.is_empty() && medications.is_empty() && conditions.is_empty() {
        lines.push("║ No medical info provided.            ║".to_string());
    }

    lines.push("╠══════════════════════════════════════╣".to_string());

    // 紧急联系人
    if !emer_name.is_empty() {
        lines.push(format!("║ EMERGENCY CONTACT                    ║"));
        lines.push(format!("║ Name:  {:<31} ║", truncate(&emer_name, 31)));
        if !emer_phone.is_empty() {
            lines.push(format!("║ Phone: {:<31} ║", truncate(&emer_phone, 31)));
        }
        if !emer_rel.is_empty() {
            lines.push(format!("║ Rel:   {:<31} ║", truncate(&emer_rel, 31)));
        }
    } else {
        lines.push("║ No emergency contact provided.       ║".to_string());
    }

    lines.push("╚══════════════════════════════════════╝".to_string());

    lines.join("\n")
}



/// 简单的 JSON 字符串转义


/// 插件入口
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Emergency Card 启动 — 生成紧急联系卡");

    let name = read_field("identity.fullName");
    if name.is_empty() {
        log_error("缺少必需字段: identity.fullName");
        return -1;
    }

    let data = read_emergency_data();
    let card = generate_card(&data);
    for line in card.lines() {
        log_info(line);
    }

    // Phase 2: 结构化结果
    let mut pairs: Vec<(&str, String)> = Vec::new();
    if !data.name.is_empty() { pairs.push(("姓名", data.name.clone())); }
    if !data.blood.is_empty() { pairs.push(("血型", data.blood.clone())); }
    if !data.allergies.is_empty() { pairs.push(("过敏", data.allergies.clone())); }
    if !data.medications.is_empty() { pairs.push(("用药", data.medications.clone())); }
    if !data.emer_name.is_empty() { pairs.push(("紧急联系人", format!("{} ({}) {}", data.emer_name, data.emer_rel, data.emer_phone))); }
    let pairs_json: Vec<String> = pairs.iter().map(|(k, v)| format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(k), escape_json(v))).collect();
    let result_json = format!(r#"{{"type":"key_value","title":"紧急联系卡","pairs":[{}],"text":"{}"}}"#, pairs_json.join(","), escape_json(&card));
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
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_generate_card_structure() {
        let data = EmergencyData {
            name: "张三".to_string(),
            dob: "1990-01-01".to_string(),
            blood: "O+".to_string(),
            allergies: "青霉素过敏".to_string(),
            medications: "阿司匹林".to_string(),
            conditions: "高血压".to_string(),
            emer_name: "李四".to_string(),
            emer_phone: "13800138000".to_string(),
            emer_rel: "配偶".to_string(),
        };
        let card = generate_card(&data);
        assert!(card.contains("EMERGENCY CARD"));
        assert!(card.contains("张三"));
        assert!(card.contains("O+"));
        assert!(card.contains("青霉素过敏"));
        assert!(card.contains("13800138000"));
        assert!(card.contains("╔"));
        assert!(card.contains("╝"));
    }

    #[test]
    fn test_generate_card_minimal() {
        let data = EmergencyData {
            name: "Test".to_string(),
            dob: String::new(),
            blood: String::new(),
            allergies: String::new(),
            medications: String::new(),
            conditions: String::new(),
            emer_name: String::new(),
            emer_phone: String::new(),
            emer_rel: String::new(),
        };
        let card = generate_card(&data);
        assert!(card.contains("Test"));
        assert!(card.contains("No medical info"));
        assert!(card.contains("No emergency contact"));
    }
}
