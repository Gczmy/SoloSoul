//! 对象详情屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use solosoul_core::{is_protected_sensitivity, ObjectRecord, UserTemplate};

use crate::i18n::I18n;
use crate::t;

/// 渲染对象详情。
///
/// `templates` 用于字段级敏感度兜底：旧对象缺少 `property_labels`（或模板
/// 创建后新增了敏感字段）时，从模板定义读取字段敏感度，与 `/search` 路径
/// 的 `collect_protected_field_keys` 判定保持一致（P006 复核）。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    object: &ObjectRecord,
    templates: &std::collections::HashMap<String, UserTemplate>,
    i18n: &I18n,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area);

    // 元信息
    let meta_text = Text::from(vec![
        Line::from(t!(i18n, "object-detail-name", name = &object.name)).bold(),
        Line::from(t!(i18n, "object-detail-id", id = &object.id)).dark_gray(),
        Line::from(t!(i18n, "object-detail-type", r#type = &object.type_id)),
        Line::from(t!(
            i18n,
            "object-detail-section",
            section = object.template_type.as_deref().unwrap_or("none")
        )),
        Line::from(t!(
            i18n,
            "object-detail-sensitivity",
            level = &object.sensitivity_level
        )),
        Line::from(t!(
            i18n,
            "object-detail-version",
            ver = &object.version.to_string()
        )),
    ]);
    let meta =
        Paragraph::new(meta_text).block(Block::default().title(" 对象信息 ").borders(Borders::ALL));
    frame.render_widget(meta, layout[0]);

    // P006: 掩码判定升级为字段级——对象级 sensitive/critical 整体掩码（旧行为）；
    // 否则逐字段按 property_labels 中的敏感度判断（sensitive/critical/restricted 字段
    // 掩码，public/internal 字段照常展示），与 GUI 的 SensitivityLevel 约定一致。
    let object_masked = is_protected_sensitivity(&object.sensitivity_level);
    let field_levels = collect_field_levels_for(object, templates);
    let header = Row::new(vec!["字段", "值"])
        .style(ratatui::style::Style::default().bold())
        .bottom_margin(1);

    let rows: Vec<Row> = if let serde_json::Value::Object(map) = &object.properties {
        map.iter()
            .map(|(k, v)| {
                let value_str = format_value(v);
                let display = field_display_value(k, &value_str, &field_levels, object_masked);
                Row::new(vec![k.clone(), display])
            })
            .collect()
    } else {
        vec![Row::new(vec![
            "properties".to_string(),
            format_value(&object.properties),
        ])]
    };

    let table = Table::new(
        rows,
        [Constraint::Percentage(35), Constraint::Percentage(65)],
    )
    .header(header)
    .block(Block::default().title(" 属性 ").borders(Borders::ALL));
    frame.render_widget(table, layout[1]);

    // 底部提示（任一字段被掩码即显示）
    if object_masked
        || field_levels
            .values()
            .any(|lvl| is_protected_sensitivity(lvl))
    {
        let hint =
            Paragraph::new(Line::from(t!(i18n, "object-detail-sensitive-masked")).dark_gray())
                .alignment(Alignment::Center);
        let hint_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        frame.render_widget(hint, hint_area);
    }
}

/// P006: 构建字段级敏感度表——property_labels 优先（对象自带的字段级覆盖），
/// 缺失时从模板定义兜底（旧对象或模板新增的敏感字段），与 `/search` 的
/// `collect_protected_field_keys` 判定保持一致。
fn collect_field_levels_for(
    object: &ObjectRecord,
    templates: &std::collections::HashMap<String, UserTemplate>,
) -> std::collections::HashMap<String, String> {
    let mut field_levels: std::collections::HashMap<String, String> = object
        .property_labels
        .as_ref()
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    // 模板兜底：property_labels 缺失（旧对象）或模板新增的敏感字段从模板补齐。
    if let Some(tid) = object.template_id.as_deref() {
        if let Some(tpl) = templates.get(tid) {
            for prop in &tpl.properties {
                field_levels
                    .entry(prop.id.clone())
                    .or_insert_with(|| prop.sensitivity_level.clone().unwrap_or_default());
            }
        }
    }
    field_levels
}

/// P006: 字段展示值——字段级敏感度优先（sensitive/critical/restricted 掩码），
/// 缺失时回退对象级；public/internal 字段照常展示。
fn field_display_value(
    key: &str,
    value: &str,
    field_levels: &std::collections::HashMap<String, String>,
    object_masked: bool,
) -> String {
    let field_masked = field_levels
        .get(key)
        .map(|lvl| is_protected_sensitivity(lvl))
        .unwrap_or(object_masked);
    if field_masked {
        mask(value)
    } else {
        value.to_string()
    }
}

fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => (if *b { "是" } else { "否" }).to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => "-".to_string(),
        serde_json::Value::Array(arr) => {
            arr.iter().map(format_value).collect::<Vec<_>>().join(", ")
        }
        serde_json::Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn mask(value: &str) -> String {
    if value.is_empty() {
        return value.to_string();
    }
    "••••••".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn levels(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_field_display_masks_sensitive_and_critical() {
        let lv = levels(&[("id", "critical"), ("phone", "internal")]);
        // critical 字段掩码
        assert_eq!(field_display_value("id", "123", &lv, false), "••••••");
        // internal 字段照常
        assert_eq!(
            field_display_value("phone", "13800138000", &lv, false),
            "13800138000"
        );
        // 未在 labels 中的字段：对象非敏感 → 照常
        assert_eq!(field_display_value("name", "张三", &lv, false), "张三");
    }

    #[test]
    fn test_field_display_falls_back_to_object_level() {
        let lv = levels(&[]);
        // 对象级 sensitive → 未标注字段掩码
        assert_eq!(field_display_value("name", "张三", &lv, true), "••••••");
        // 对象级 internal → 未标注字段照常
        assert_eq!(field_display_value("name", "张三", &lv, false), "张三");
    }

    #[test]
    fn test_field_display_public_overrides_object_level() {
        let lv = levels(&[("name", "public")]);
        // 对象级 sensitive，但字段显式 public → 照常
        assert_eq!(field_display_value("name", "张三", &lv, true), "张三");
    }

    #[test]
    fn test_field_display_empty_value_not_masked() {
        let lv = levels(&[("id", "critical")]);
        assert_eq!(field_display_value("id", "", &lv, false), "");
    }

    #[test]
    fn test_field_display_keeps_explicit_public_even_when_object_masked() {
        // 模板定义 public 字段 → 即使对象级 sensitive 也照常展示（与 GUI 一致）
        let lv = levels(&[("name", "public")]);
        assert_eq!(field_display_value("name", "张三", &lv, true), "张三");
    }

    #[test]
    fn test_render_template_fallback_masks_old_object_fields() {
        // P006 复核：旧对象无 property_labels 但模板含 sensitive 字段时，/open 也应掩码
        // （与 /search 的 collect_protected_field_keys 兜底一致）。
        let mut tpl = solosoul_core::UserTemplate {
            id: "tpl-1".to_string(),
            account_id: "acc-1".to_string(),
            name: "测试".to_string(),
            icon_id: None,
            category: None,
            created_at: "2026-01-01".to_string(),
            updated_at: None,
            contract_type_id: None,
            properties: vec![solosoul_core::TemplateProperty {
                id: "passport_no".to_string(),
                name: "护照号".to_string(),
                prop_type: solosoul_core::PropertyType::Text,
                sensitivity_level: Some("critical".to_string()),
                sensitive: None,
                options: None,
                deprecated_at: None,
                contract_field: None,
                contract_bindings: None,
                allowed_types: None,
                max_items: None,
            }],
        };
        tpl.id = "tpl-1".to_string();
        let mut templates = std::collections::HashMap::new();
        templates.insert("tpl-1".to_string(), tpl);

        let mut record = solosoul_core::ObjectRecord {
            id: "obj-1".to_string(),
            account_id: "acc-1".to_string(),
            type_id: "page".to_string(),
            section_type: "custom".to_string(),
            name: "对象".to_string(),
            icon_name: String::new(),
            parent_id: None,
            children_ids: Vec::new(),
            properties: serde_json::json!({"passport_no": "E12345678"}),
            property_labels: None, // 旧对象：无 property_labels
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: Vec::new(),
            template_id: Some("tpl-1".to_string()),
            template_type: None,
            contract_type_id: None,
            template_hash: None,
            ignored_template_hash: None,
            version: 1,
            updated_at: "2026-01-01".to_string(),
            created_at: "2026-01-01".to_string(),
        };
        record.id = "obj-1".to_string();

        // 模板兜底后的 field_levels：passport_no → critical → 掩码
        let field_levels = super::collect_field_levels_for(&record, &templates);
        assert_eq!(
            field_levels.get("passport_no").map(String::as_str),
            Some("critical"),
            "模板敏感字段应被补充为 critical"
        );
        assert_eq!(
            field_display_value(
                "passport_no",
                "E12345678",
                &field_levels,
                is_protected_sensitivity(&record.sensitivity_level)
            ),
            "••••••",
            "模板敏感字段在 /open 中应被掩码"
        );
    }
}
