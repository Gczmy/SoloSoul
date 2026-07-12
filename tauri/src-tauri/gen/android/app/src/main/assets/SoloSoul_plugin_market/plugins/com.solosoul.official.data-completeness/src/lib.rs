//! Data Completeness — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 扫描 Vault 各分区，计算档案完整度百分比并给出补充建议。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_info, send_result_json, truncate};
use solosoul_plugin_sdk::{escape_json};

/// 分区定义
struct Section {
    name: &'static str,
    icon: &'static str,
    fields: &'static [(&'static str, &'static str)],
}

/// 内置分区清单
const SECTIONS: &[Section] = &[
    Section {
        name: "身份信息",
        icon: "👤",
        fields: &[
            ("identity.fullName", "姓名"),
            ("identity.dateOfBirth", "出生日期"),
            ("identity.nationality", "国籍"),
            ("identity.sex", "性别"),
        ],
    },
    Section {
        name: "联系方式",
        icon: "📇",
        fields: &[
            ("contact.email", "电子邮箱"),
            ("contact.phone", "电话"),
        ],
    },
    Section {
        name: "地址",
        icon: "📍",
        fields: &[
            ("address.street", "街道"),
            ("address.city", "城市"),
            ("address.country", "国家"),
        ],
    },
    Section {
        name: "护照",
        icon: "🛂",
        fields: &[
            ("passport.number", "护照号码"),
            ("passport.expiryDate", "有效期"),
        ],
    },
    Section {
        name: "身份证",
        icon: "🆔",
        fields: &[
            ("idCard.number", "身份证号"),
        ],
    },
    Section {
        name: "教育",
        icon: "🎓",
        fields: &[
            ("education.institution", "学校"),
            ("education.degree", "学位"),
        ],
    },
    Section {
        name: "工作",
        icon: "💼",
        fields: &[
            ("employment.company", "公司"),
            ("employment.position", "职位"),
        ],
    },
    Section {
        name: "财务",
        icon: "💳",
        fields: &[
            ("financial.bankStatement", "银行流水"),
        ],
    },
    Section {
        name: "医疗",
        icon: "🏥",
        fields: &[
            ("medical.bloodType", "血型"),
            ("medical.allergies", "过敏史"),
        ],
    },
    Section {
        name: "旅行",
        icon: "✈️",
        fields: &[
            ("travel.visitedCountries", "到访国家"),
        ],
    },
    Section {
        name: "安全",
        icon: "🔐",
        fields: &[
            ("security.totpSecret", "2FA 密钥"),
        ],
    },
];

/// 检查单个字段是否存在
#[cfg(not(test))]
fn check_field(path: &str) -> bool {
    match get_field(path) {
        Ok(v) => !v.trim().is_empty(),
        Err(_) => false,
    }
}

/// 计算进度条字符串
fn progress_bar(percentage: u32, width: usize) -> String {
    let filled = (percentage as usize * width / 100).min(width);
    let empty = width - filled;
    format!(
        "[{}{}] {}",
        "█".repeat(filled),
        "░".repeat(empty),
        percentage
    ) + "%"
}

/// 简单的 JSON 字符串转义


/// 生成分区报告
fn generate_report(results: &[(String, u32, Vec<String>)]) -> String {
    let total_fields: usize = SECTIONS.iter().map(|s| s.fields.len()).sum();
    let filled_fields: usize = results
        .iter()
        .map(|(_, pct, _)| {
            let section_total = SECTIONS
                .iter()
                .find(|s| s.name == results.iter().find(|(name, _, _)| name == &s.name).map(|(n, _, _)| n.as_str()).unwrap_or(""))
                .map(|s| s.fields.len())
                .unwrap_or(1);
            *pct as usize * section_total / 100
        })
        .sum();

    let overall = if total_fields > 0 {
        (filled_fields * 100 / total_fields) as u32
    } else {
        0
    };

    let mut lines = Vec::new();
    lines.push("╔══════════════════════════════════════╗".to_string());
    lines.push("║    📊 DATA COMPLETENESS REPORT       ║".to_string());
    lines.push("╠══════════════════════════════════════╣".to_string());
    lines.push(format!("║ 总体完成度: {}              ║", progress_bar(overall, 20)));
    lines.push("╠══════════════════════════════════════╣".to_string());

    for (name, pct, missing) in results {
        let section = SECTIONS.iter().find(|s| s.name == name.as_str());
        let icon = section.map(|s| s.icon).unwrap_or("📋");
        lines.push(format!("║ {} {}: {}", icon, name, progress_bar(*pct, 20)));
        if !missing.is_empty() && missing.len() <= 3 {
            let missing_str = missing.join(", ");
            lines.push(format!("║   💡 建议补充: {}", truncate(&missing_str, 30)));
        }
    }

    lines.push("╚══════════════════════════════════════╝".to_string());

    if overall >= 80 {
        lines.push("".to_string());
        lines.push("🌟 档案非常完整！".to_string());
    } else if overall >= 50 {
        lines.push("".to_string());
        lines.push("📈 档案基本完成，继续补充细节。".to_string());
    } else {
        lines.push("".to_string());
        lines.push("📝 档案尚有较大完善空间。".to_string());
    }

    lines.join("\n")
}



#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Data Completeness 启动 — 扫描档案完整度");

    let mut results = Vec::new();

    for section in SECTIONS {
        let mut filled = 0u32;
        let mut missing = Vec::new();

        for (path, label) in section.fields {
            if check_field(path) {
                filled += 1;
            } else {
                missing.push(label.to_string());
            }
        }

        let total = section.fields.len() as u32;
        let pct = if total > 0 { filled * 100 / total } else { 0 };
        results.push((section.name.to_string(), pct, missing));
    }

    let report = generate_report(&results);

    // Phase 2: 发送结构化结果
    let sections_json: Vec<String> = results
        .iter()
        .map(|(name, pct, missing)| {
            let section = SECTIONS.iter().find(|s| s.name == name.as_str()).unwrap_or(&SECTIONS[0]);
            let total = section.fields.len() as u32;
            let filled = (*pct as u32 * total / 100).min(total);
            let missing_json: Vec<String> = missing.iter().map(|m| format!(r#""{}""#, escape_json(m))).collect();
            format!(
                r#"{{"name":"{}","icon":"{}","percentage":{},"totalFields":{},"filledFields":{},"missing":[{}]}}"#,
                escape_json(name),
                escape_json(section.icon),
                pct,
                total,
                filled,
                missing_json.join(",")
            )
        })
        .collect();

    let total_fields: usize = SECTIONS.iter().map(|s| s.fields.len()).sum();
    let filled_fields: usize = results
        .iter()
        .map(|(name, pct, _)| {
            let section = SECTIONS.iter().find(|s| s.name == name.as_str()).unwrap_or(&SECTIONS[0]);
            (*pct as usize * section.fields.len() / 100).min(section.fields.len())
        })
        .sum();
    let overall = if total_fields > 0 { (filled_fields * 100 / total_fields) as u32 } else { 0 };

    let message = if overall >= 80 {
        "档案非常完整！"
    } else if overall >= 50 {
        "档案基本完成，继续补充细节。"
    } else {
        "档案尚有较大完善空间。"
    };

    let result_json = format!(
        r#"{{"type":"data_completeness","title":"档案完整度报告","overall":{},"sections":[{}],"message":"{}"}}"#,
        overall,
        sections_json.join(","),
        escape_json(message)
    );
    let _ = send_result_json(&result_json);

    // 同时保留日志输出
    for line in report.lines() {
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
    fn test_progress_bar() {
        assert!(progress_bar(50, 10).contains("█████"));
        assert!(progress_bar(50, 10).contains("50%"));
        assert!(progress_bar(0, 10).contains("░"));
        assert!(progress_bar(100, 10).contains("100%"));
    }

    #[test]
    fn test_generate_report() {
        let results = vec![
            ("身份信息".to_string(), 75, vec!["性别".to_string()]),
            ("联系方式".to_string(), 50, vec!["电话".to_string()]),
            ("地址".to_string(), 100, vec![]),
        ];
        let report = generate_report(&results);
        assert!(report.contains("DATA COMPLETENESS REPORT"));
        assert!(report.contains("75%"));
        assert!(report.contains("100%"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }
}
