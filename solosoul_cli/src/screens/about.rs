//! /about 信息屏幕。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::commands::system::AboutInfo;
use crate::i18n::I18n;
use crate::t;

pub fn render(frame: &mut ratatui::Frame, area: Rect, info: &AboutInfo, i18n: &I18n) {
    let lock_status = if info.lock_acquired {
        t!(i18n, "about-lock-acquired")
    } else {
        t!(i18n, "about-lock-none")
    };
    let lines = vec![
        Line::from(""),
        Line::from(info.app_name.clone())
            .bold()
            .alignment(Alignment::Center),
        Line::from(t!(i18n, "about-version", ver = info.version)).alignment(Alignment::Center),
        Line::from(t!(i18n, "about-platform", os = info.os, arch = info.arch))
            .alignment(Alignment::Center),
        Line::from(t!(i18n, "about-data-dir", path = info.data_dir)).alignment(Alignment::Center),
        Line::from(t!(i18n, "about-lock", status = lock_status)).alignment(Alignment::Center),
        Line::from(""),
        Line::from(t!(i18n, "app-tagline-short"))
            .dark_gray()
            .alignment(Alignment::Center),
    ];

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(t!(i18n, "about-title"))
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}
