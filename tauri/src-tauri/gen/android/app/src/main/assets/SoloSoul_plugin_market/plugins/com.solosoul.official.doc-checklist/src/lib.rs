//! Doc Checklist — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 根据目标场景检查 Vault 中已有/缺失的材料。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_error, log_info, send_result_json, show_dialog, PluginError, truncate};
use solosoul_plugin_sdk::{escape_json};

/// 材料项
struct DocItem {
    name: &'static str,
    field_path: &'static str,
}

/// 场景定义
struct Scenario {
    id: &'static str,
    name: &'static str,
    docs: &'static [DocItem],
}

/// 内置场景清单
const SCENARIOS: &[Scenario] = &[
    Scenario {
        id: "japan-visa",
        name: "日本签证",
        docs: &[
            DocItem { name: "有效护照", field_path: "passport.number" },
            DocItem { name: "证件照片", field_path: "identity.idPhoto" },
            DocItem { name: "在职/在学证明", field_path: "employment.company" },
            DocItem { name: "银行流水/存款证明", field_path: "financial.bankStatement" },
            DocItem { name: "行程单", field_path: "travel.itinerary" },
            DocItem { name: "酒店预订", field_path: "travel.hotelBooking" },
        ],
    },
    Scenario {
        id: "us-visa",
        name: "美国签证 (B1/B2)",
        docs: &[
            DocItem { name: "有效护照", field_path: "passport.number" },
            DocItem { name: "证件照片", field_path: "identity.idPhoto" },
            DocItem { name: "DS-160 确认页", field_path: "visa.ds160Confirmation" },
            DocItem { name: "面试预约确认", field_path: "visa.interviewAppointment" },
            DocItem { name: "财力证明", field_path: "financial.bankStatement" },
            DocItem { name: "在职/在学证明", field_path: "employment.company" },
        ],
    },
    Scenario {
        id: "schengen-visa",
        name: "申根签证",
        docs: &[
            DocItem { name: "有效护照", field_path: "passport.number" },
            DocItem { name: "证件照片", field_path: "identity.idPhoto" },
            DocItem { name: "旅行保险", field_path: "insurance.travel" },
            DocItem { name: "行程单", field_path: "travel.itinerary" },
            DocItem { name: "酒店预订", field_path: "travel.hotelBooking" },
            DocItem { name: "财力证明", field_path: "financial.bankStatement" },
            DocItem { name: "在职/在学证明", field_path: "employment.company" },
        ],
    },
    Scenario {
        id: "uk-visa",
        name: "英国签证",
        docs: &[
            DocItem { name: "有效护照", field_path: "passport.number" },
            DocItem { name: "证件照片", field_path: "identity.idPhoto" },
            DocItem { name: "肺结核检测证明", field_path: "medical.tbTest" },
            DocItem { name: "CAS/邀请函", field_path: "visa.casLetter" },
            DocItem { name: "财力证明", field_path: "financial.bankStatement" },
            DocItem { name: "住宿证明", field_path: "travel.hotelBooking" },
        ],
    },
    Scenario {
        id: "bank-account",
        name: "银行开户",
        docs: &[
            DocItem { name: "有效身份证件", field_path: "passport.number" },
            DocItem { name: "地址证明", field_path: "address.street" },
            DocItem { name: "收入/工作证明", field_path: "employment.company" },
        ],
    },
    Scenario {
        id: "hotel-checkin",
        name: "酒店入住",
        docs: &[
            DocItem { name: "有效护照", field_path: "passport.number" },
            DocItem { name: "酒店预订确认", field_path: "travel.hotelBooking" },
            DocItem { name: "信用卡", field_path: "card.number" },
        ],
    },
];

/// 查找场景（支持 ID、名称、关键词模糊匹配）
fn find_scenario(query: &str) -> Option<&'static Scenario> {
    let q = query.to_lowercase();

    // 1. 精确匹配 ID
    for s in SCENARIOS {
        if s.id == q {
            return Some(s);
        }
    }

    // 2. 包含匹配名称
    for s in SCENARIOS {
        if s.name.to_lowercase().contains(&q) || q.contains(&s.id.to_lowercase()) {
            return Some(s);
        }
    }

    // 3. 关键词匹配
    let keywords: Vec<(&str, &str)> = vec![
        ("日本", "japan-visa"),
        ("美国", "us-visa"),
        ("申根", "schengen-visa"),
        ("英国", "uk-visa"),
        ("银行", "bank-account"),
        ("酒店", "hotel-checkin"),
    ];
    for (kw, id) in &keywords {
        if q.contains(kw) {
            return SCENARIOS.iter().find(|s| s.id == *id);
        }
    }

    None
}

/// 检查单个材料是否存在
#[cfg(not(test))]
fn check_doc(item: &DocItem) -> bool {
    match get_field(item.field_path) {
        Ok(v) => !v.trim().is_empty(),
        Err(_) => false,
    }
}

#[cfg(test)]
fn check_doc_mock(item: &DocItem, present_fields: &[&str]) -> bool {
    present_fields.contains(&item.field_path)
}

/// 生成检查报告
fn generate_report(scenario: &Scenario, results: &[(String, bool)]) -> String {
    let total = results.len();
    let present = results.iter().filter(|(_, ok)| *ok).count();
    let missing = total - present;

    let mut lines = Vec::new();
    lines.push(format!("╔══════════════════════════════════════╗"));
    lines.push(format!("║      📋 DOC CHECKLIST                ║"));
    lines.push(format!("╠══════════════════════════════════════╣"));
    lines.push(format!("║ 场景: {:<31} ║", truncate(scenario.name, 31)));
    lines.push(format!("║ 进度: {}/{} 已准备 | {} 缺失        ║", present, total, missing));
    lines.push(format!("╠══════════════════════════════════════╣"));

    for (name, ok) in results {
        let icon = if *ok { "✅" } else { "❌" };
        lines.push(format!("║ {} {:<33} ║", icon, truncate(name, 33)));
    }

    lines.push(format!("╚══════════════════════════════════════╝"));

    if missing == 0 {
        lines.push("".to_string());
        lines.push("🎉 所有材料已准备就绪！".to_string());
    } else {
        lines.push("".to_string());
        lines.push(format!("⚠️  还有 {} 项材料缺失，请补充。", missing));
    }

    lines.join("\n")
}



/// 简单的 JSON 字符串转义


/// 解析对话框返回的 JSON 结果
fn parse_dialog_result(json_str: &str) -> String {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(selected) = val.get("selected").and_then(|v| v.as_str()) {
            return selected.to_string();
        }
    }
    String::new()
}

/// 从 Dart 端传入的初始参数中读取指定 key
/// 对应 Host Function: solosoul_get_param
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

/// 插件入口
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Doc Checklist 启动 — 检查材料清单");

    // 1. 优先尝试读取 Dart 端传入的场景参数
    let scenario_query = match get_param("scenario_id") {
        Ok(id) if !id.is_empty() => {
            log_info(&format!("使用传入场景参数: {}", id));
            id
        }
        _ => {
            // 2. 无参数时，通过通用对话框请求用户选择场景（向后兼容）
            let dialog_config = r#"{
                "title": {"zh": "选择签证/业务类型", "en": "Select Visa/Business Type"},
                "description": {"zh": "选择场景后，插件将请求访问相关字段，请继续授权。", "en": "After selecting a scenario, the plugin will request access to relevant fields."},
                "type": "radio_list",
                "items": [
                    {"id": "japan-visa", "label": {"zh": "日本签证", "en": "Japan Visa"}},
                    {"id": "us-visa", "label": {"zh": "美国签证 (B1/B2)", "en": "US Visa (B1/B2)"}},
                    {"id": "schengen-visa", "label": {"zh": "申根签证", "en": "Schengen Visa"}},
                    {"id": "uk-visa", "label": {"zh": "英国签证", "en": "UK Visa"}},
                    {"id": "bank-account", "label": {"zh": "银行开户", "en": "Bank Account"}},
                    {"id": "hotel-checkin", "label": {"zh": "酒店入住", "en": "Hotel Check-in"}}
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
    for item in scenario.docs {
        let present = check_doc(item);
        results.push((item.name.to_string(), present));
    }

    let report = generate_report(scenario, &results);
    for line in report.lines() {
        log_info(line);
    }

    // Phase 2: 结构化结果
    let pairs_json: Vec<String> = results.iter().map(|(name, present)| {
        format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(name), escape_json(if *present { "✅ 已准备" } else { "❌ 缺失" }))
    }).collect();
    let result_json = format!(r#"{{"type":"key_value","title":"{} 材料清单","pairs":[{}],"text":"{}"}}"#, escape_json(scenario.name), pairs_json.join(","), escape_json(&report));
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
    fn test_find_scenario_exact_id() {
        assert_eq!(find_scenario("japan-visa").unwrap().id, "japan-visa");
        assert_eq!(find_scenario("us-visa").unwrap().id, "us-visa");
    }

    #[test]
    fn test_find_scenario_keyword() {
        assert_eq!(find_scenario("日本").unwrap().id, "japan-visa");
        assert_eq!(find_scenario("美国签证").unwrap().id, "us-visa");
        assert_eq!(find_scenario("申根").unwrap().id, "schengen-visa");
        assert_eq!(find_scenario("银行开户").unwrap().id, "bank-account");
    }

    #[test]
    fn test_find_scenario_unknown() {
        assert!(find_scenario("火星签证").is_none());
    }

    #[test]
    fn test_generate_report() {
        let scenario = &SCENARIOS[0]; // japan-visa
        let results = vec![
            ("有效护照".to_string(), true),
            ("证件照片".to_string(), false),
            ("在职证明".to_string(), true),
        ];
        let report = generate_report(scenario, &results);
        assert!(report.contains("日本签证"));
        assert!(report.contains("✅"));
        assert!(report.contains("❌"));
        assert!(report.contains("2/3"));
    }

    #[test]
    fn test_check_doc_mock() {
        let item = DocItem { name: "护照", field_path: "passport.number" };
        assert!(check_doc_mock(&item, &["passport.number"]));
        assert!(!check_doc_mock(&item, &["address.street"]));
    }
}
