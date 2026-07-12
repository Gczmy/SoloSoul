//! Calendar Events — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 为护照/签证/身份证到期日生成 iCalendar (.ics) 格式的提醒事件。

#[cfg(not(test))]
use solosoul_plugin_sdk::{get_field, log_info, send_result_json};
use solosoul_plugin_sdk::{escape_json};

/// 事件定义
struct CalendarEvent {
    uid: String,
    summary: String,
    description: String,
    date: String, // YYYYMMDD
    alarm_days_before: i32,
    kind: String, // passport, visa, idcard, card
    original_date: String, // 原始日期（用于展示）
}

/// 解析日期为 YYYYMMDD 格式（支持 ISO 和 YYMMDD）
fn to_ics_date(date_str: &str) -> Option<String> {
    let s = date_str.trim();
    if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        // YYYY-MM-DD → YYYYMMDD
        Some(format!("{}{}{}", &s[0..4], &s[5..7], &s[8..10]))
    } else if s.len() == 6 && s.chars().all(|c| c.is_ascii_digit()) {
        // YYMMDD → 20YYMMDD 或 19YYMMDD
        let yy: i32 = s[..2].parse().ok()?;
        let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
        Some(format!("{}{}{}", year, &s[2..4], &s[4..6]))
    } else {
        None
    }
}

/// 简单的 JSON 字符串转义


/// 生成 iCalendar VEVENT 块
fn generate_vevent(event: &CalendarEvent) -> String {
    let alarm_trigger = format!("-P{}D", event.alarm_days_before);
    format!(
        "BEGIN:VEVENT\n\
         UID:{}\n\
         SUMMARY:{}\n\
         DESCRIPTION:{}\n\
         DTSTART;VALUE=DATE:{}\n\
         DTEND;VALUE=DATE:{}\n\
         BEGIN:VALARM\n\
         ACTION:DISPLAY\n\
         DESCRIPTION:Reminder\n\
         TRIGGER:{}\n\
         END:VALARM\n\
         END:VEVENT",
        event.uid,
        ics_escape(&event.summary),
        ics_escape(&event.description),
        event.date,
        event.date,
        alarm_trigger,
    )
}

/// iCalendar 文本转义
fn ics_escape(s: &str) -> String {
    s.replace("\\", "\\\\")
        .replace(";", "\\;")
        .replace(",", "\\,")
        .replace("\n", "\\n")
}

/// 生成完整 iCalendar 文件内容
fn generate_ics(events: &[CalendarEvent]) -> String {
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//SoloSoul//Calendar Events//EN".to_string(),
        "CALSCALE:GREGORIAN".to_string(),
        "METHOD:PUBLISH".to_string(),
    ];

    for event in events {
        for line in generate_vevent(event).lines() {
            lines.push(line.to_string());
        }
    }

    lines.push("END:VCALENDAR".to_string());
    lines.join("\n")
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Calendar Events 启动 — 生成本地日历提醒");

    let mut events = Vec::new();
    let mut event_count = 0;

    // 护照到期
    if let Ok(date) = get_field("passport.expiryDate") {
        if let Some(ics_date) = to_ics_date(&date) {
            events.push(CalendarEvent {
                uid: format!("solosoul-passport-expiry-{}", ics_date),
                summary: "护照到期提醒".to_string(),
                description: format!("您的护照将于 {} 到期，请提前办理续签。", date),
                date: ics_date,
                alarm_days_before: 90,
                kind: "passport".to_string(),
                original_date: date,
            });
            event_count += 1;
        }
    }

    // 签证到期
    if let Ok(date) = get_field("visa.expiryDate") {
        if let Some(ics_date) = to_ics_date(&date) {
            events.push(CalendarEvent {
                uid: format!("solosoul-visa-expiry-{}", ics_date),
                summary: "签证到期提醒".to_string(),
                description: format!("您的签证将于 {} 到期，请提前办理续签。", date),
                date: ics_date,
                alarm_days_before: 30,
                kind: "visa".to_string(),
                original_date: date,
            });
            event_count += 1;
        }
    }

    // 身份证到期
    if let Ok(date) = get_field("idCard.expiryDate") {
        if let Some(ics_date) = to_ics_date(&date) {
            events.push(CalendarEvent {
                uid: format!("solosoul-idcard-expiry-{}", ics_date),
                summary: "身份证到期提醒".to_string(),
                description: format!("您的身份证将于 {} 到期，请提前办理换领。", date),
                date: ics_date,
                alarm_days_before: 60,
                kind: "idcard".to_string(),
                original_date: date,
            });
            event_count += 1;
        }
    }

    // 信用卡到期
    if let Ok(date) = get_field("card.expiryDate") {
        if let Some(ics_date) = to_ics_date(&date) {
            events.push(CalendarEvent {
                uid: format!("solosoul-card-expiry-{}", ics_date),
                summary: "信用卡到期提醒".to_string(),
                description: format!("您的信用卡将于 {} 到期，请联系银行换卡。", date),
                date: ics_date,
                alarm_days_before: 30,
                kind: "card".to_string(),
                original_date: date,
            });
            event_count += 1;
        }
    }

    if event_count == 0 {
        log_info("未找到任何到期日期字段，无需生成日历事件。");
        return 0;
    }

    let ics = generate_ics(&events);

    // Phase 2: 发送结构化结果（日历事件卡片 + 可导出的 ICS）
    let events_json: Vec<String> = events
        .iter()
        .map(|e| {
            format!(
                r#"{{"kind":"{}","summary":"{}","date":"{}","dateIcs":"{}","alarmDays":{},"description":"{}"}}"#,
                escape_json(&e.kind),
                escape_json(&e.summary),
                escape_json(&e.original_date),
                escape_json(&e.date),
                e.alarm_days_before,
                escape_json(&e.description)
            )
        })
        .collect();

    let result_json = format!(
        r#"{{"type":"calendar_events","title":"日历提醒事件","eventCount":{},"events":[{}],"ics":"{}"}}"#,
        event_count,
        events_json.join(","),
        escape_json(&ics)
    );
    let _ = send_result_json(&result_json);

    // 同时保留日志输出供调试
    log_info("【iCalendar (.ics) 输出】");
    log_info("（可复制到系统日历软件导入）");
    log_info("");
    for line in ics.lines() {
        log_info(line);
    }
    log_info("");
    log_info(&format!("已生成 {} 个提醒事件。", event_count));

    0
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_ics_date_iso() {
        assert_eq!(to_ics_date("2025-12-31"), Some("20251231".to_string()));
        assert_eq!(to_ics_date("2020-01-01"), Some("20200101".to_string()));
    }

    #[test]
    fn test_to_ics_date_mrz() {
        assert_eq!(to_ics_date("251231"), Some("20251231".to_string()));
        assert_eq!(to_ics_date("991231"), Some("19991231".to_string()));
    }

    #[test]
    fn test_ics_escape() {
        assert_eq!(ics_escape("a;b,c\\d"), "a\\;b\\,c\\\\d");
        assert_eq!(ics_escape("hello\nworld"), "hello\\nworld");
    }

    #[test]
    fn test_generate_vevent() {
        let event = CalendarEvent {
            uid: "test-uid".to_string(),
            summary: "护照到期".to_string(),
            description: "请续签".to_string(),
            date: "20251231".to_string(),
            alarm_days_before: 90,
        };
        let vevent = generate_vevent(&event);
        assert!(vevent.contains("BEGIN:VEVENT"));
        assert!(vevent.contains("END:VEVENT"));
        assert!(vevent.contains("UID:test-uid"));
        assert!(vevent.contains("SUMMARY:护照到期"));
        assert!(vevent.contains("DTSTART;VALUE=DATE:20251231"));
        assert!(vevent.contains("TRIGGER:-P90D"));
        assert!(vevent.contains("BEGIN:VALARM"));
    }

    #[test]
    fn test_generate_ics() {
        let events = vec![CalendarEvent {
            uid: "uid1".to_string(),
            summary: "Test".to_string(),
            description: "Desc".to_string(),
            date: "20250101".to_string(),
            alarm_days_before: 30,
        }];
        let ics = generate_ics(&events);
        assert!(ics.contains("BEGIN:VCALENDAR"));
        assert!(ics.contains("END:VCALENDAR"));
        assert!(ics.contains("BEGIN:VEVENT"));
        assert!(ics.contains("PRODID:-//SoloSoul//Calendar Events//EN"));
    }
}
