//! 插件详情 TUI 屏幕（展示完整 PluginManifest）。

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use solosoul_plugin::PluginManifest;

/// 渲染插件详情页。
pub fn render(frame: &mut Frame, area: Rect, manifest: &PluginManifest) {
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
    lines.push(Line::from(vec![
        Span::styled("作者: ", label),
        Span::styled(manifest.author.as_deref().unwrap_or("未指定"), value),
    ]));
    lines.push(Line::from(vec![
        Span::styled("类别: ", label),
        Span::styled(manifest.category.as_str(), value),
    ]));
    lines.push(Line::from(vec![
        Span::styled("等级: ", label),
        Span::styled(format!("{:?}", manifest.tier), value),
    ]));
    lines.push(Line::from(vec![
        Span::styled("需要确认: ", label),
        Span::styled(
            if manifest.require_user_confirmation {
                "是"
            } else {
                "否"
            },
            value,
        ),
    ]));

    if let Some(ref homepage) = manifest.homepage {
        lines.push(Line::from(vec![
            Span::styled("主页: ", label),
            Span::styled(homepage.as_str(), value),
        ]));
    }

    if let Some(ref core_ver) = manifest.required_core_version {
        lines.push(Line::from(vec![
            Span::styled("要求 Core: ", label),
            Span::styled(format!(">= {}", core_ver), value),
        ]));
    }

    lines.push(Line::from(vec![Span::raw("")]));

    // 权限
    lines.push(Line::from(vec![Span::styled("权限:", label)]));
    if manifest.permissions.is_empty() {
        lines.push(Line::from(vec![Span::styled("  无特殊权限", dim)]));
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
    lines.push(Line::from(vec![Span::styled("网络策略:", label)]));
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
        lines.push(Line::from(vec![Span::styled("参数:", label)]));
        for p in &manifest.params {
            let required = if p.required { "[必填]" } else { "[可选]" };
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
            Span::styled("数据 TTL: ", label),
            Span::styled(format!("{}s", manifest.data_ttl_seconds), value),
        ]));
    }

    // WASM hash
    if let Some(ref hash) = manifest.wasm_hash_sha256 {
        lines.push(Line::from(vec![
            Span::styled("WASM SHA256: ", label),
            Span::styled(hash.as_str(), dim),
        ]));
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(format!("插件详情: {}", manifest.name))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
