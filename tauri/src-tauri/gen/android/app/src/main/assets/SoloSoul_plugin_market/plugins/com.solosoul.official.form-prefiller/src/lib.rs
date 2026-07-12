//! Form Prefiller — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 根据常见表单场景生成 Vault 字段映射表，显示哪些字段已就绪/缺失。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_error, log_info, send_result_json, show_dialog, PluginError, truncate};
use solosoul_plugin_sdk::{escape_json};

/// 表单字段映射
struct FieldMapping {
    form_field: &'static str,
    vault_path: &'static str,
    #[allow(dead_code)]
    description: &'static str,
}

/// 场景定义
struct FormScenario {
    id: &'static str,
    name: &'static str,
    fields: &'static [FieldMapping],
}

const SCENARIOS: &[FormScenario] = &[
    FormScenario {
        id: "visa-application",
        name: "签证申请表",
        fields: &[
            FieldMapping { form_field: "Full Name", vault_path: "identity.full_name", description: "全名" },
            FieldMapping { form_field: "Date of Birth", vault_path: "identity.date_of_birth", description: "出生日期" },
            FieldMapping { form_field: "Nationality", vault_path: "identity.nationality", description: "国籍" },
            FieldMapping { form_field: "Sex", vault_path: "identity.gender", description: "性别" },
            FieldMapping { form_field: "Passport Number", vault_path: "passport.number", description: "护照号" },
            FieldMapping { form_field: "Passport Expiry", vault_path: "passport.expiryDate", description: "护照有效期" },
            FieldMapping { form_field: "Place of Birth", vault_path: "passport.place_of_birth", description: "出生地" },
            FieldMapping { form_field: "Issuing Authority", vault_path: "passport.authority", description: "签发机关" },
            FieldMapping { form_field: "Email", vault_path: "contact.email", description: "电子邮箱" },
            FieldMapping { form_field: "Phone", vault_path: "contact.phone", description: "电话" },
            FieldMapping { form_field: "Home Address", vault_path: "address.street", description: "家庭住址" },
            FieldMapping { form_field: "Employer", vault_path: "employment.company", description: "雇主" },
            FieldMapping { form_field: "Occupation", vault_path: "employment.position", description: "职业" },
        ],
    },
    FormScenario {
        id: "hotel-checkin",
        name: "酒店入住",
        fields: &[
            FieldMapping { form_field: "Guest Name", vault_path: "identity.full_name", description: "客人姓名" },
            FieldMapping { form_field: "Phone", vault_path: "contact.phone", description: "联系电话" },
            FieldMapping { form_field: "Email", vault_path: "contact.email", description: "电子邮箱" },
            FieldMapping { form_field: "Passport/ID", vault_path: "passport.number", description: "护照号" },
            FieldMapping { form_field: "Credit Card", vault_path: "card.cardNumber", description: "信用卡" },
        ],
    },
    FormScenario {
        id: "bank-account",
        name: "银行开户",
        fields: &[
            FieldMapping { form_field: "Full Name", vault_path: "identity.full_name", description: "全名" },
            FieldMapping { form_field: "Date of Birth", vault_path: "identity.date_of_birth", description: "出生日期" },
            FieldMapping { form_field: "Nationality", vault_path: "identity.nationality", description: "国籍" },
            FieldMapping { form_field: "ID Number", vault_path: "passport.number", description: "证件号码" },
            FieldMapping { form_field: "Phone", vault_path: "contact.phone", description: "电话" },
            FieldMapping { form_field: "Email", vault_path: "contact.email", description: "电子邮箱" },
            FieldMapping { form_field: "Home Address", vault_path: "address.street", description: "住址" },
            FieldMapping { form_field: "City", vault_path: "address.city", description: "城市" },
            FieldMapping { form_field: "Postal Code", vault_path: "address.postalCode", description: "邮编" },
            FieldMapping { form_field: "Country", vault_path: "address.country", description: "国家" },
            FieldMapping { form_field: "Employer", vault_path: "employment.company", description: "雇主" },
            FieldMapping { form_field: "Occupation", vault_path: "employment.position", description: "职业" },
        ],
    },
    FormScenario {
        id: "airline-checkin",
        name: "航空值机",
        fields: &[
            FieldMapping { form_field: "Passenger Name", vault_path: "identity.full_name", description: "乘客姓名" },
            FieldMapping { form_field: "Passport Number", vault_path: "passport.number", description: "护照号" },
            FieldMapping { form_field: "Nationality", vault_path: "identity.nationality", description: "国籍" },
            FieldMapping { form_field: "Date of Birth", vault_path: "passport.dateOfBirth", description: "出生日期" },
            FieldMapping { form_field: "Passport Expiry", vault_path: "passport.expiryDate", description: "护照有效期" },
            FieldMapping { form_field: "Emergency Contact", vault_path: "contact.emergencyName", description: "紧急联系人" },
            FieldMapping { form_field: "Emergency Phone", vault_path: "contact.emergencyPhone", description: "紧急联系电话" },
        ],
    },
];

/// 查找场景
fn find_scenario(query: &str) -> Option<&'static FormScenario> {
    let q = query.to_lowercase();
    for s in SCENARIOS {
        if s.id == q || s.name.to_lowercase().contains(&q) || q.contains(s.id) {
            return Some(s);
        }
    }
    let keywords: Vec<(&str, &str)> = vec![
        ("签证", "visa-application"),
        ("酒店", "hotel-checkin"),
        ("银行", "bank-account"),
        ("航空", "airline-checkin"),
        ("值机", "airline-checkin"),
    ];
    for (kw, id) in &keywords {
        if q.contains(kw) {
            return SCENARIOS.iter().find(|s| s.id == *id);
        }
    }
    None
}

#[cfg(not(test))]
fn check_field(path: &str) -> bool {
    match get_field(path) {
        Ok(v) => !v.trim().is_empty(),
        Err(_) => false,
    }
}

/// 解析对话框返回的 JSON 结果（无 serde_json 依赖，手动提取）
fn parse_dialog_result(json_str: &str) -> String {
    // 查找 "selected" 键，提取其后第一个字符串值
    let mut key_buf = String::new();
    let mut in_string = false;
    let mut chars = json_str.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' if !in_string => {
                in_string = true;
                key_buf.clear();
            }
            '"' if in_string => {
                in_string = false;
                if key_buf == "selected" {
                    // 跳过冒号和空白，找下一个字符串
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '"' {
                            chars.next(); // consume "
                            let mut val = String::new();
                            while let Some(c) = chars.next() {
                                if c == '"' { break; }
                                if c == '\\' {
                                    if let Some(escaped) = chars.next() {
                                        match escaped {
                                            'n' => val.push('\n'),
                                            'r' => val.push('\r'),
                                            't' => val.push('\t'),
                                            '\\' => val.push('\\'),
                                            '"' => val.push('"'),
                                            c => val.push(c),
                                        }
                                    }
                                } else {
                                    val.push(c);
                                }
                            }
                            return val;
                        }
                        if next_ch.is_alphanumeric() || next_ch == '_' || next_ch == '-' { break; }
                        chars.next();
                    }
                }
            }
            c if in_string => key_buf.push(c),
            ':' => { key_buf.clear(); }
            _ => {}
        }
    }
    String::new()
}

/// 从 Dart 端传入的初始参数中读取指定 key
#[cfg(not(test))]
fn get_param(key: &str) -> Result<String, i32> {
    extern "C" {
        fn solosoul_get_param(key_ptr: *const u8, key_len: usize, out_ptr: *mut u8, out_cap: usize) -> i32;
    }
    let mut buf = vec![0u8; 256];
    let ret = unsafe {
        solosoul_get_param(key.as_ptr(), key.len(), buf.as_mut_ptr(), buf.len())
    };
    if ret == 0 {
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        Ok(String::from_utf8_lossy(&buf[..len]).to_string())
    } else {
        Err(ret)
    }
}

/// 生成分字段映射报告
fn generate_report(scenario: &FormScenario, results: &[(String, String, bool)]) -> String {
    let total = results.len();
    let ready = results.iter().filter(|(_, _, ok)| *ok).count();

    let mut lines = Vec::new();
    lines.push("╔══════════════════════════════════════╗".to_string());
    lines.push("║       📝 FORM PREFILLER              ║".to_string());
    lines.push("╠══════════════════════════════════════╣".to_string());
    lines.push(format!("║ 场景: {:<31} ║", truncate(scenario.name, 31)));
    lines.push(format!("║ 就绪: {}/{} 字段                     ║", ready, total));
    lines.push("╠══════════════════════════════════════╣".to_string());

    for (form_field, vault_path, ok) in results {
        let icon = if *ok { "✅" } else { "❌" };
        let status = if *ok { "就绪" } else { "缺失" };
        lines.push(format!("║ {} {:<20} → {}", icon, truncate(form_field, 20), vault_path));
        lines.push(format!("║   [{}]", status));
    }

    lines.push("╚══════════════════════════════════════╝".to_string());

    if ready == total {
        lines.push("".to_string());
        lines.push("🎉 所有字段已就绪，可以开始填表！".to_string());
    } else {
        lines.push("".to_string());
        lines.push(format!("💡 还有 {} 个字段需要补充。", total - ready));
    }

    lines.join("\n")
}



/// 简单的 JSON 字符串转义


#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Form Prefiller 启动 — 生成表单字段映射");

    // 1. 优先尝试读取 Dart 端传入的场景参数
    let scenario_query = match get_param("scenario_id") {
        Ok(id) if !id.is_empty() => {
            log_info(&format!("使用传入场景参数: {}", id));
            id
        }
        _ => {
            // 2. 无参数时，通过通用对话框请求用户选择场景（向后兼容）
            let dialog_config = r#"{
                "title": {"zh": "选择表单场景", "en": "Select Form Scenario"},
                "description": {"zh": "选择场景后，插件将生成 Vault 字段到表单字段的映射表。", "en": "After selecting a scenario, the plugin will generate a mapping table from Vault fields to form fields."},
                "type": "radio_list",
                "items": [
                    {"id": "visa-application", "label": {"zh": "签证申请表", "en": "Visa Application"}},
                    {"id": "hotel-checkin", "label": {"zh": "酒店入住", "en": "Hotel Check-in"}},
                    {"id": "bank-account", "label": {"zh": "银行开户", "en": "Bank Account"}},
                    {"id": "airline-checkin", "label": {"zh": "航空值机", "en": "Airline Check-in"}}
                ]
            }"#;

            let result_json = match show_dialog(dialog_config) {
                Ok(json) => json,
                Err(PluginError::UserDenied) => {
                    log_info("用户取消场景选择");
                    return 0;
                }
                Err(PluginError::TtlExpired) => {
                    log_error("场景选择超时，请重试");
                    return -3;
                }
                Err(e) => {
                    log_error(&format!("对话框错误: {:?}", e));
                    return -1;
                }
            };

            let query = parse_dialog_result(&result_json);
            if query.is_empty() {
                log_error("未选择场景");
                return -2;
            }
            query
        }
    };

    let scenario = match find_scenario(&scenario_query) {
        Some(s) => s,
        None => {
            log_error(&format!("未知场景: '{}'", scenario_query));
            log_info("支持的场景:");
            for s in SCENARIOS {
                log_info(&format!("  - {} ({})", s.name, s.id));
            }
            return -3;
        }
    };

    let mut results = Vec::new();
    for mapping in scenario.fields {
        let present = check_field(mapping.vault_path);
        results.push((
            mapping.form_field.to_string(),
            mapping.vault_path.to_string(),
            present,
        ));
    }

    let report = generate_report(scenario, &results);
    for line in report.lines() {
        log_info(line);
    }

    // Phase 2: 结构化结果
    let pairs_json: Vec<String> = results.iter().map(|(field, _vault_path, present)| {
        format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(field), escape_json(if *present { "✅ 已就绪" } else { "❌ 缺失" }))
    }).collect();
    let result_json = format!(r#"{{"type":"key_value","title":"{} 表单预填","pairs":[{}],"text":"{}"}}"#, escape_json(scenario.name), pairs_json.join(","), escape_json(&report));
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
    fn test_find_scenario() {
        assert_eq!(find_scenario("visa-application").unwrap().id, "visa-application");
        assert_eq!(find_scenario("签证").unwrap().id, "visa-application");
        assert_eq!(find_scenario("酒店入住").unwrap().id, "hotel-checkin");
        assert_eq!(find_scenario("银行").unwrap().id, "bank-account");
        assert_eq!(find_scenario("航空值机").unwrap().id, "airline-checkin");
    }

    #[test]
    fn test_find_scenario_unknown() {
        assert!(find_scenario("火星表单").is_none());
    }

    #[test]
    fn test_generate_report() {
        let scenario = &SCENARIOS[0];
        let results = vec![
            ("Full Name".to_string(), "identity.full_name".to_string(), true),
            ("Passport Number".to_string(), "passport.number".to_string(), false),
        ];
        let report = generate_report(scenario, &results);
        assert!(report.contains("FORM PREFILLER"));
        assert!(report.contains("✅"));
        assert!(report.contains("❌"));
    }
}
