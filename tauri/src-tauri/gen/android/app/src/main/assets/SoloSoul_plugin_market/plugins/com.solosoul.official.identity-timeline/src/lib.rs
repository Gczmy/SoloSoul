//! Identity Timeline — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 按时间顺序展示教育、工作、证件等人生里程碑。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_info, send_result_json, truncate};
use solosoul_plugin_sdk::{escape_json};

/// 时间线事件
#[derive(Debug, Clone)]
struct TimelineEvent {
    year: i32,
    month: u32,
    title: String,
    detail: String,
}

/// 从日期字符串提取年份（支持 YYYY, YYYY-MM, YYYY-MM-DD）
fn extract_year(date_str: &str) -> Option<i32> {
    let s = date_str.trim();
    if s.len() >= 4 {
        s[..4].parse().ok()
    } else {
        None
    }
}

/// 从日期字符串提取月份
fn extract_month(date_str: &str) -> u32 {
    let s = date_str.trim();
    if s.len() >= 7 && s.as_bytes()[4] == b'-' {
        s[5..7].parse().unwrap_or(1)
    } else {
        1
    }
}

/// 添加事件（如年份有效）
fn add_event(events: &mut Vec<TimelineEvent>, year: i32, month: u32, title: &str, detail: &str) {
    if year > 1900 && year < 2100 {
        events.push(TimelineEvent {
            year,
            month,
            title: title.to_string(),
            detail: detail.to_string(),
        });
    }
}

/// 生成时间线报告
fn generate_timeline(events: &[TimelineEvent]) -> String {
    if events.is_empty() {
        return "暂无时间线数据。请在 Vault 中补充教育、工作、证件等日期信息。".to_string();
    }

    let mut sorted = events.to_vec();
    sorted.sort_by(|a, b| {
        a.year.cmp(&b.year)
            .then_with(|| a.month.cmp(&b.month))
    });

    let mut lines = Vec::new();
    lines.push("╔══════════════════════════════════════╗".to_string());
    lines.push("║      🕐 IDENTITY TIMELINE            ║".to_string());
    lines.push("╠══════════════════════════════════════╣".to_string());

    let mut current_year = 0;
    for event in &sorted {
        if event.year != current_year {
            current_year = event.year;
            lines.push(format!("║ {}                                ║", current_year));
            lines.push("║ ──────────────────────────────────── ║".to_string());
        }
        let month_str = format!("{:02}", event.month);
        lines.push(format!(
            "║ {} {}: {}",
            month_str,
            truncate(&event.title, 6),
            truncate(&event.detail, 24)
        ));
    }

    lines.push("╚══════════════════════════════════════╝".to_string());
    lines.join("\n")
}



/// 简单的 JSON 字符串转义


#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Identity Timeline 启动 — 生成身份时间线");

    let mut events = Vec::new();

    // 出生
    if let Ok(dob) = get_field("identity.dateOfBirth") {
        if let Some(year) = extract_year(&dob) {
            add_event(&mut events, year, 1, "出生", "");
        }
    }

    // 教育
    if let (Ok(inst), Ok(deg), Ok(year_str)) = (
        get_field("education.institution"),
        get_field("education.degree"),
        get_field("education.year"),
    ) {
        if let Some(year) = extract_year(&year_str) {
            let detail = if !deg.is_empty() {
                format!("{} 毕业于 {}", deg, inst)
            } else {
                format!("毕业于 {}", inst)
            };
            add_event(&mut events, year, 6, "教育", &detail);
        }
    }

    // 工作
    if let (Ok(company), Ok(pos), Ok(start)) = (
        get_field("employment.company"),
        get_field("employment.position"),
        get_field("employment.startDate"),
    ) {
        if let Some(year) = extract_year(&start) {
            let month = extract_month(&start);
            let detail = if !pos.is_empty() {
                format!("{} 入职 {} 任 {}", pos, company, pos)
            } else {
                format!("入职 {}", company)
            };
            add_event(&mut events, year, month, "工作", &detail);
        }
    }

    // 护照
    if let (Ok(issue), Ok(expiry)) = (
        get_field("passport.issueDate"),
        get_field("passport.expiryDate"),
    ) {
        if let Some(year) = extract_year(&issue) {
            let month = extract_month(&issue);
            add_event(&mut events, year, month, "护照", "护照签发");
        }
        if let Some(year) = extract_year(&expiry) {
            let month = extract_month(&expiry);
            add_event(&mut events, year, month, "护照", "护照到期");
        }
    }

    // 签证
    if let (Ok(visa_type), Ok(issue)) = (
        get_field("visa.type"),
        get_field("visa.issueDate"),
    ) {
        if let Some(year) = extract_year(&issue) {
            let month = extract_month(&issue);
            let detail = format!("{} 签证签发", visa_type);
            add_event(&mut events, year, month, "签证", &detail);
        }
    }

    let report = generate_timeline(&events);
    for line in report.lines() {
        log_info(line);
    }

    // Phase 2: 结构化结果
    let pairs_json: Vec<String> = events.iter().map(|e| {
        format!(r#"{{"key":"{} {}","value":"{}"}}"#, escape_json(&e.year.to_string()), escape_json(&e.title), escape_json(&e.detail))
    }).collect();
    let result_json = format!(r#"{{"type":"key_value","title":"身份时间线","pairs":[{}],"text":"{}"}}"#, pairs_json.join(","), escape_json(&report));
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
    fn test_extract_year() {
        assert_eq!(extract_year("1990-05-23"), Some(1990));
        assert_eq!(extract_year("2000"), Some(2000));
        assert_eq!(extract_year("invalid"), None);
        assert_eq!(extract_year(""), None);
    }

    #[test]
    fn test_extract_month() {
        assert_eq!(extract_month("1990-05-23"), 5);
        assert_eq!(extract_month("2000"), 1);
        assert_eq!(extract_month("invalid"), 1);
    }

    #[test]
    fn test_generate_timeline() {
        let events = vec![
            TimelineEvent { year: 2019, month: 6, title: "教育".to_string(), detail: "本科毕业".to_string() },
            TimelineEvent { year: 2020, month: 3, title: "工作".to_string(), detail: "入职 Example".to_string() },
            TimelineEvent { year: 2019, month: 1, title: "出生".to_string(), detail: "".to_string() },
        ];
        let report = generate_timeline(&events);
        assert!(report.contains("IDENTITY TIMELINE"));
        assert!(report.contains("2019"));
        assert!(report.contains("2020"));
        assert!(report.contains("教育"));
        assert!(report.contains("工作"));
    }

    #[test]
    fn test_generate_timeline_empty() {
        let report = generate_timeline(&[]);
        assert!(report.contains("暂无时间线数据"));
    }

    #[test]
    fn test_add_event_invalid_year() {
        let mut events = Vec::new();
        add_event(&mut events, 1800, 1, "测试", "详情");
        add_event(&mut events, 2500, 1, "测试", "详情");
        assert!(events.is_empty());
    }
}
