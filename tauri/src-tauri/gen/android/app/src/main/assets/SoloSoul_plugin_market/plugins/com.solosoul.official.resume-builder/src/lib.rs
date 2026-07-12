//! Resume Builder — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 从 Vault 档案生成标准 Markdown 格式简历。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_info, send_result_json};
use solosoul_plugin_sdk::{escape_json};

/// 简历数据
struct ResumeData {
    name: String,
    email: String,
    phone: String,
    address: String,
    website: String,
    linkedin: String,
    company: String,
    position: String,
    emp_start: String,
    emp_end: String,
    emp_desc: String,
    institution: String,
    degree: String,
    field: String,
    year: String,
    skill_primary: String,
    skill_secondary: String,
    lang_native: String,
    lang_others: String,
}

#[cfg(not(test))]
fn read_field(path: &str) -> String {
    get_field(path).unwrap_or_default().trim().to_string()
}

#[cfg(not(test))]
fn read_resume_data() -> ResumeData {
    ResumeData {
        name: read_field("identity.fullName"),
        email: read_field("identity.email"),
        phone: read_field("identity.phone"),
        address: read_field("identity.address"),
        website: read_field("identity.website"),
        linkedin: read_field("identity.linkedin"),
        company: read_field("employment.company"),
        position: read_field("employment.position"),
        emp_start: read_field("employment.startDate"),
        emp_end: read_field("employment.endDate"),
        emp_desc: read_field("employment.description"),
        institution: read_field("education.institution"),
        degree: read_field("education.degree"),
        field: read_field("education.field"),
        year: read_field("education.year"),
        skill_primary: read_field("skill.primary"),
        skill_secondary: read_field("skill.secondary"),
        lang_native: read_field("language.native"),
        lang_others: read_field("language.others"),
    }
}

/// 生成 Markdown 简历（纯函数，便于测试）
fn build_resume(data: &ResumeData) -> String {
    let mut lines = Vec::new();

    // 标题
    lines.push(format!("# {}", data.name));
    lines.push(String::new());

    // 联系信息
    let mut contacts = Vec::new();
    if !data.email.is_empty() { contacts.push(format!("📧 {}", data.email)); }
    if !data.phone.is_empty() { contacts.push(format!("📱 {}", data.phone)); }
    if !data.address.is_empty() { contacts.push(format!("📍 {}", data.address)); }
    if !data.website.is_empty() { contacts.push(format!("🌐 {}", data.website)); }
    if !data.linkedin.is_empty() { contacts.push(format!("💼 {}", data.linkedin)); }

    if !contacts.is_empty() {
        lines.push(contacts.join(" | "));
        lines.push(String::new());
    }

    // 工作经历
    if !data.company.is_empty() || !data.position.is_empty() {
        lines.push("## 工作经历".to_string());
        lines.push(String::new());

        let duration = if !data.emp_start.is_empty() {
            if data.emp_end.is_empty() {
                format!("{} - 至今", data.emp_start)
            } else {
                format!("{} - {}", data.emp_start, data.emp_end)
            }
        } else {
            String::new()
        };

        let header = if !data.position.is_empty() && !data.company.is_empty() {
            format!("**{}** @ {}", data.position, data.company)
        } else if !data.position.is_empty() {
            format!("**{}**", data.position)
        } else {
            format!("@{}", data.company)
        };

        lines.push(format!("- {}", header));
        if !duration.is_empty() {
            lines.push(format!("  - 时间: {}", duration));
        }
        if !data.emp_desc.is_empty() {
            for desc_line in data.emp_desc.lines() {
                let trimmed = desc_line.trim();
                if !trimmed.is_empty() {
                    lines.push(format!("  - {}", trimmed));
                }
            }
        }
        lines.push(String::new());
    }

    // 教育背景
    if !data.institution.is_empty() || !data.degree.is_empty() {
        lines.push("## 教育背景".to_string());
        lines.push(String::new());

        let edu_header = if !data.degree.is_empty() && !data.field.is_empty() {
            format!("**{}**，{}", data.degree, data.field)
        } else if !data.degree.is_empty() {
            format!("**{}**", data.degree)
        } else if !data.field.is_empty() {
            data.field.clone()
        } else {
            String::new()
        };

        if !edu_header.is_empty() && !data.institution.is_empty() {
            lines.push(format!("- {} — {}", edu_header, data.institution));
        } else if !data.institution.is_empty() {
            lines.push(format!("- {}", data.institution));
        } else if !edu_header.is_empty() {
            lines.push(format!("- {}", edu_header));
        }

        if !data.year.is_empty() {
            lines.push(format!("  - 毕业年份: {}", data.year));
        }
        lines.push(String::new());
    }

    // 技能
    if !data.skill_primary.is_empty() || !data.skill_secondary.is_empty() {
        lines.push("## 技能".to_string());
        lines.push(String::new());
        if !data.skill_primary.is_empty() {
            lines.push(format!("- **核心技能**: {}", data.skill_primary));
        }
        if !data.skill_secondary.is_empty() {
            lines.push(format!("- **其他技能**: {}", data.skill_secondary));
        }
        lines.push(String::new());
    }

    // 语言能力
    if !data.lang_native.is_empty() || !data.lang_others.is_empty() {
        lines.push("## 语言能力".to_string());
        lines.push(String::new());
        if !data.lang_native.is_empty() {
            lines.push(format!("- **母语**: {}", data.lang_native));
        }
        if !data.lang_others.is_empty() {
            lines.push(format!("- **其他**: {}", data.lang_others));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

/// 简单的 JSON 字符串转义


/// 插件入口
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Resume Builder 启动 — 生成简历");

    let data = read_resume_data();
    let resume = build_resume(&data);
    for line in resume.lines() {
        log_info(line);
    }

    // Phase 2: 结构化结果
    let mut pairs: Vec<(&str, String)> = Vec::new();
    if !data.name.is_empty() { pairs.push(("姓名", data.name.clone())); }
    if !data.email.is_empty() { pairs.push(("邮箱", data.email.clone())); }
    if !data.phone.is_empty() { pairs.push(("电话", data.phone.clone())); }
    if !data.address.is_empty() { pairs.push(("地址", data.address.clone())); }
    if !data.website.is_empty() { pairs.push(("网站", data.website.clone())); }
    if !data.linkedin.is_empty() { pairs.push(("LinkedIn", data.linkedin.clone())); }
    if !data.company.is_empty() || !data.position.is_empty() {
        let work = if !data.position.is_empty() && !data.company.is_empty() {
            format!("{} @ {}", data.position, data.company)
        } else if !data.position.is_empty() {
            data.position.clone()
        } else {
            data.company.clone()
        };
        pairs.push(("工作", work));
    }
    if !data.emp_start.is_empty() || !data.emp_end.is_empty() {
        let duration = if data.emp_end.is_empty() {
            format!("{} - 至今", data.emp_start)
        } else {
            format!("{} - {}", data.emp_start, data.emp_end)
        };
        pairs.push(("在职时间", duration));
    }
    if !data.institution.is_empty() || !data.degree.is_empty() {
        let edu = if !data.degree.is_empty() && !data.institution.is_empty() {
            format!("{} — {}", data.degree, data.institution)
        } else if !data.degree.is_empty() {
            data.degree.clone()
        } else {
            data.institution.clone()
        };
        pairs.push(("教育", edu));
    }
    if !data.skill_primary.is_empty() { pairs.push(("核心技能", data.skill_primary.clone())); }
    if !data.skill_secondary.is_empty() { pairs.push(("其他技能", data.skill_secondary.clone())); }
    if !data.lang_native.is_empty() { pairs.push(("母语", data.lang_native.clone())); }
    if !data.lang_others.is_empty() { pairs.push(("其他语言", data.lang_others.clone())); }

    let pairs_json: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(k), escape_json(v)))
        .collect();

    let result_json = format!(
        r#"{{"type":"key_value","title":"个人简历","pairs":[{}],"text":"{}"}}"#,
        pairs_json.join(","),
        escape_json(&resume)
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

    fn sample_data() -> ResumeData {
        ResumeData {
            name: "张三".to_string(),
            email: "zhangsan@example.com".to_string(),
            phone: "13800138000".to_string(),
            address: "北京市海淀区".to_string(),
            website: "https://example.com".to_string(),
            linkedin: "linkedin.com/in/zhangsan".to_string(),
            company: "Example Tech".to_string(),
            position: "软件工程师".to_string(),
            emp_start: "2020-01".to_string(),
            emp_end: "".to_string(),
            emp_desc: "负责后端开发\n使用 Rust 和 Go".to_string(),
            institution: "北京大学".to_string(),
            degree: "本科".to_string(),
            field: "计算机科学".to_string(),
            year: "2019".to_string(),
            skill_primary: "Rust, Go, Python".to_string(),
            skill_secondary: "React, Docker".to_string(),
            lang_native: "中文".to_string(),
            lang_others: "英语 (流利), 日语 (入门)".to_string(),
        }
    }

    #[test]
    fn test_build_resume_full() {
        let resume = build_resume(&sample_data());
        assert!(resume.contains("# 张三"));
        assert!(resume.contains("zhangsan@example.com"));
        assert!(resume.contains("软件工程师"));
        assert!(resume.contains("Example Tech"));
        assert!(resume.contains("北京大学"));
        assert!(resume.contains("Rust, Go, Python"));
        assert!(resume.contains("中文"));
    }

    #[test]
    fn test_build_resume_minimal() {
        let data = ResumeData {
            name: "Test".to_string(),
            email: String::new(),
            phone: String::new(),
            address: String::new(),
            website: String::new(),
            linkedin: String::new(),
            company: String::new(),
            position: String::new(),
            emp_start: String::new(),
            emp_end: String::new(),
            emp_desc: String::new(),
            institution: String::new(),
            degree: String::new(),
            field: String::new(),
            year: String::new(),
            skill_primary: String::new(),
            skill_secondary: String::new(),
            lang_native: String::new(),
            lang_others: String::new(),
        };
        let resume = build_resume(&data);
        assert!(resume.contains("# Test"));
    }
}
