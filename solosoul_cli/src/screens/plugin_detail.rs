//! 插件详情 TUI 屏幕（展示完整 PluginManifest）。

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::i18n::I18n;
use crate::t;
use solosoul_plugin::PluginManifest;

/// 渲染插件详情页。
pub fn render(frame: &mut Frame, area: Rect, manifest: &PluginManifest, i18n: &I18n) {
    let mut lines: Vec<Line> = Vec::new();

    let header = Style::default().fg(Color::Cyan);
    let label = Style::default().fg(Color::Yellow);
    let value = Style::default().fg(Color::White);
    let dim = Style::default().fg(Color::DarkGray);

    // 标题
    lines.push(Line::from(vec![Span::styled(
        format!("{} v{}", manifest.name, manifest.version),
        header,
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!("id: {}", manifest.id),
        dim,
    )]));
    lines.push(Line::from(vec![Span::raw("")]));

    // 描述
    if !manifest.description.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            manifest.description.as_str(),
            value,
        )]));
        lines.push(Line::from(vec![Span::raw("")]));
    }

    // 基本信息
    let none_str = t!(i18n, "generic-none");
    lines.push(Line::from(vec![
        Span::styled(t!(i18n, "plugin-detail-author"), label),
        Span::styled(manifest.author.as_deref().unwrap_or(&none_str), value),
    ]));
    lines.push(Line::from(vec![
        Span::styled(t!(i18n, "plugin-detail-category"), label),
        Span::styled(manifest.category.as_str(), value),
    ]));
    lines.push(Line::from(vec![
        Span::styled(t!(i18n, "plugin-detail-tier"), label),
        Span::styled(format!("{:?}", manifest.tier), value),
    ]));
    lines.push(Line::from(vec![
        Span::styled(t!(i18n, "plugin-detail-confirm"), label),
        Span::styled(
            if manifest.require_user_confirmation {
                t!(i18n, "generic-yes")
            } else {
                t!(i18n, "generic-no")
            },
            value,
        ),
    ]));

    if let Some(ref homepage) = manifest.homepage {
        lines.push(Line::from(vec![
            Span::styled(t!(i18n, "plugin-detail-homepage"), label),
            Span::styled(homepage.as_str(), value),
        ]));
    }

    if let Some(ref core_ver) = manifest.required_core_version {
        lines.push(Line::from(vec![
            Span::styled(t!(i18n, "plugin-detail-core"), label),
            Span::styled(format!(">= {}", core_ver), value),
        ]));
    }

    lines.push(Line::from(vec![Span::raw("")]));

    // 权限
    lines.push(Line::from(vec![Span::styled(
        t!(i18n, "plugin-detail-permissions"),
        label,
    )]));
    if manifest.permissions.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            t!(i18n, "plugin-detail-no-permissions"),
            dim,
        )]));
    } else {
        for perm in &manifest.permissions {
            lines.push(Line::from(vec![Span::styled(
                format!("  - {}", perm),
                value,
            )]));
        }
    }

    lines.push(Line::from(vec![Span::raw("")]));

    // 网络策略
    let net = &manifest.network_policy;
    lines.push(Line::from(vec![Span::styled(
        t!(i18n, "plugin-detail-network"),
        label,
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!("  allow_outbound: {}", net.block_all_outbound),
        value,
    )]));

    if !net.allowed_domains.is_empty() {
        for d in &net.allowed_domains {
            lines.push(Line::from(vec![Span::styled(
                format!("  domain: {}", d),
                dim,
            )]));
        }
    }

    lines.push(Line::from(vec![Span::raw("")]));

    // 参数
    if !manifest.params.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            t!(i18n, "plugin-detail-params"),
            label,
        )]));
        for p in &manifest.params {
            let required = if p.required {
                t!(i18n, "plugin-detail-required")
            } else {
                t!(i18n, "plugin-detail-optional")
            };
            lines.push(Line::from(vec![Span::styled(
                format!("  {}: {} {}", p.label, p.description, required),
                value,
            )]));
        }
        lines.push(Line::from(vec![Span::raw("")]));
    }

    // 数据 TTL
    if manifest.data_ttl_seconds > 0 {
        lines.push(Line::from(vec![
            Span::styled(t!(i18n, "plugin-detail-ttl"), label),
            Span::styled(format!("{}s", manifest.data_ttl_seconds), value),
        ]));
    }

    // WASM hash
    if let Some(ref hash) = manifest.wasm_hash_sha256 {
        lines.push(Line::from(vec![
            Span::styled(t!(i18n, "plugin-detail-wasm-hash"), label),
            Span::styled(hash.as_str(), dim),
        ]));
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(format!(
                    "{} {}",
                    t!(i18n, "plugin-detail-prefix"),
                    manifest.name
                ))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
