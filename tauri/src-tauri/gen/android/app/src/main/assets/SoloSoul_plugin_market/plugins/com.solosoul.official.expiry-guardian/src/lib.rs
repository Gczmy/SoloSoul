//! Expiry Guardian — SoloSoul Official Plugin (Typed Contract Edition)
//!
//! 基于 Stage 4-B typed contract 扫描 Vault 中所有带 expiryDate 角色的对象，
//! 计算剩余天数并按 urgency 分级输出结构化结果（支持 custom_ui "expiry_guardian"）。

use serde::{Deserialize, Serialize};
use solosoul_plugin_sdk::{
    days_until_ymd, get_data_structure_tree, get_locale, list_objects, log_error, log_info,
    parse_date_yyyymmdd_or_iso, send_result_json,
};

// ============================================================================
// 类型定义
// ============================================================================

/// Urgency 分级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Urgency {
    Expired,
    Critical,
    Warning,
    Notice,
    Safe,
}

impl Urgency {
    fn from_days(days: i64) -> Self {
        match days {
            d if d < 0 => Urgency::Expired,
            d if d <= 30 => Urgency::Critical,
            d if d <= 60 => Urgency::Warning,
            d if d <= 90 => Urgency::Notice,
            _ => Urgency::Safe,
        }
    }

    fn i18n_key(self) -> &'static str {
        match self {
            Urgency::Expired => "expired",
            Urgency::Critical => "critical",
            Urgency::Warning => "warning",
            Urgency::Notice => "notice",
            Urgency::Safe => "safe",
        }
    }
}

/// 单个证件条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpiryItem {
    object_id: String,
    object_name: String,
    kind: String,
    expiry_date: String,
    days_remaining: i64,
    urgency: Urgency,
}

/// 统计摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpirySummary {
    total: usize,
    expired: usize,
    critical: usize,
    warning: usize,
    notice: usize,
    safe: usize,
}

/// 最终结果（custom_ui: "expiry_guardian"）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpiryResult {
    #[serde(rename = "type")]
    result_type: String,
    title: String,
    locale: String,
    items: Vec<ExpiryItem>,
    summary: ExpirySummary,
}

// ============================================================================
// 扫描逻辑
// ============================================================================

/// 从数据结构树找出所有声明了 expiryDate 角色的类型别名。
fn discover_expiry_types() -> Vec<(String, String)> {
    let mut result = Vec::new();
    let tree_json = match get_data_structure_tree() {
        Ok(json) => json,
        Err(e) => {
            log_error(&format!("无法读取数据结构树: {:?}", e));
            return result;
        }
    };

    let tree: serde_json::Value = match serde_json::from_str(&tree_json) {
        Ok(v) => v,
        Err(e) => {
            log_error(&format!("数据结构树 JSON 解析失败: {}", e));
            return result;
        }
    };

    let empty_vec = vec![];
    let types = tree
        .get("types")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_vec);
    for t in types {
        let alias = t
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = t
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let props = t
            .get("properties")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let has_expiry = props.iter().any(|p| {
            p.get("id")
                .and_then(|v| v.as_str())
                .map(|id| id == "expiryDate")
                .unwrap_or(false)
        });
        if has_expiry {
            result.push((alias, name));
        }
    }
    result
}

/// 扫描单个类型的所有对象，读取 document name + expiryDate。
fn scan_type(alias: &str, type_name: &str) -> Vec<ExpiryItem> {
    let mut items = Vec::new();
    let json = match list_objects(alias) {
        Ok(j) => j,
        Err(e) => {
            log_error(&format!("list_objects({}) 失败: {:?}", alias, e));
            return items;
        }
    };

    let objects: Vec<serde_json::Value> = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(e) => {
            log_error(&format!("解析 {} 对象列表失败: {}", alias, e));
            return items;
        }
    };

    for obj in &objects {
        let id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let props = obj
            .get("properties")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // 直接从当前对象的 properties 读取 expiryDate，避免通过 get_field
        // 查询（get_field 返回的是该类型第一个对象的属性值，而非当前迭代的对象）。
        // 见 field.rs resolve_typed: objects[0].properties
        let raw_date = props
            .get("expiryDate")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if raw_date.is_empty() {
            log_info(&format!("{}: 未填写到期日", name));
            continue;
        }

        match parse_date_yyyymmdd_or_iso(&raw_date) {
            Some((year, month, day)) => match days_until_ymd(year, month, day) {
                Some(days) => {
                    let urgency = Urgency::from_days(days);
                    log_info(&format!(
                        "{} ({}): {} — {}天后到期 ({})",
                        name,
                        type_name,
                        raw_date,
                        days,
                        urgency.i18n_key()
                    ));
                    items.push(ExpiryItem {
                        object_id: id,
                        object_name: name,
                        kind: type_name.to_string(),
                        expiry_date: raw_date,
                        days_remaining: days,
                        urgency,
                    });
                }
                None => {
                    log_error(&format!("{}: 日期计算失败 '{}'", name, raw_date));
                }
            },
            None => {
                log_error(&format!(
                    "{}: 无法解析日期 '{}' (期望 YYYY-MM-DD 或 YYMMDD)",
                    name, raw_date
                ));
            }
        }
    }

    items
}

// ============================================================================
// 入口函数
// ============================================================================

#[no_mangle]
pub extern "C" fn run() -> i32 {
    let locale = get_locale().unwrap_or_else(|_| "en".to_string());
    log_info("Expiry Guardian 启动 — 基于契约扫描证件有效期");

    let mut items = Vec::new();
    for (alias, type_name) in discover_expiry_types() {
        log_info(&format!("扫描类型: {} ({})", type_name, alias));
        items.extend(scan_type(&alias, &type_name));
    }

    // 按 urgency 升序、days_remaining 升序排序
    items.sort_by_key(|i| (i.urgency, i.days_remaining));

    let summary = ExpirySummary {
        total: items.len(),
        expired: items.iter().filter(|i| i.urgency == Urgency::Expired).count(),
        critical: items
            .iter()
            .filter(|i| i.urgency == Urgency::Critical)
            .count(),
        warning: items
            .iter()
            .filter(|i| i.urgency == Urgency::Warning)
            .count(),
        notice: items
            .iter()
            .filter(|i| i.urgency == Urgency::Notice)
            .count(),
        safe: items.iter().filter(|i| i.urgency == Urgency::Safe).count(),
    };

    let title = if locale.starts_with("zh") {
        "证件到期预警"
    } else {
        "Document Expiry Alerts"
    };

    let result = ExpiryResult {
        result_type: "expiry_guardian".to_string(),
        title: title.to_string(),
        locale,
        items,
        summary,
    };

    match serde_json::to_string(&result) {
        Ok(json) => {
            log_info(&format!("结果序列化成功 ({} 字节)", json.len()));
            if let Err(e) = send_result_json(&json) {
                log_error(&format!("发送结果失败: code={}", e));
                return -1;
            }
        }
        Err(e) => {
            log_error(&format!("结果序列化失败: {}", e));
            return -1;
        }
    }

    log_info("Expiry Guardian 扫描完毕");
    0
}
