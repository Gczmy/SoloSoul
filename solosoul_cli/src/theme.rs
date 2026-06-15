//! SoloSoul CLI 主题系统（GUI 默认品牌蓝）。
//!
//! 统一提供品牌色、状态色、图标回退与终端颜色自动降级。
//! 品牌蓝使用新主色 `--accent-primary: #4fa8ff`。
//! 降级规则：
//! - `COLORTERM=truecolor` 或 `SOLOSOUL_CLI_TRUECOLOR=1` → RGB 真色
//! - `TERM` 包含 `256color` → 256 色（Indexed）
//! - 否则 → ANSI 基础色

use ratatui::style::{Color, Modifier, Style};

/// GUI 品牌蓝主题调色板与图标配置。
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub brand: Color,
    pub cream: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    /// Logo 填充色，GUI 暖石白 `--stone-50: #fafaf6`。
    pub logo_fill: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub bg: Color,
    /// 是否使用 Unicode 图标（emoji / 符号）。
    pub use_icons: bool,
}

impl Theme {
    /// 根据环境自动选择颜色级别并加载主题。
    pub fn load() -> Self {
        let color_level = detect_color_level();
        let use_icons = should_use_icons();
        Self::with_level(color_level, use_icons)
    }

    /// 按指定颜色级别构造 GUI 品牌蓝主题。
    pub fn with_level(level: ColorLevel, use_icons: bool) -> Self {
        match level {
            ColorLevel::TrueColor => Self {
                brand: Color::Rgb(79, 168, 255),      // #4fa8ff
                cream: Color::Rgb(142, 175, 200),     // #8eafc8
                border: Color::Rgb(74, 106, 133),     // #4a6a85
                text: Color::Rgb(224, 234, 242),      // #e0eaf2
                muted: Color::Rgb(143, 163, 176),     // #8fa3b0
                logo_fill: Color::Rgb(250, 250, 246), // #fafaf6 暖石白
                success: Color::Rgb(130, 200, 120),
                warning: Color::Rgb(255, 200, 100),
                error: Color::Rgb(255, 120, 100),
                bg: Color::Rgb(26, 32, 38), // #1a2026
                use_icons,
            },
            ColorLevel::Indexed => Self {
                brand: Color::Indexed(75), // 最接近 #4fa8ff 的 256 色
                cream: Color::Indexed(110),
                border: Color::Indexed(60),
                text: Color::Indexed(255),
                muted: Color::Indexed(109),
                logo_fill: Color::Indexed(15),
                success: Color::Indexed(114),
                warning: Color::Indexed(221),
                error: Color::Indexed(203),
                bg: Color::Indexed(234),
                use_icons,
            },
            ColorLevel::Ansi => Self {
                brand: Color::LightBlue, // ANSI 下最接近 #4fa8ff 的亮色蓝
                cream: Color::LightBlue,
                border: Color::DarkGray,
                text: Color::White,
                muted: Color::Gray,
                logo_fill: Color::White,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                bg: Color::Black,
                use_icons,
            },
        }
    }

    /// 大号品牌标题样式。
    pub fn style_brand(&self) -> Style {
        Style::default().fg(self.brand).add_modifier(Modifier::BOLD)
    }

    /// 普通品牌高亮样式。
    pub fn style_brand_dim(&self) -> Style {
        Style::default().fg(self.brand)
    }

    /// 副标语/奶油色强调样式。
    pub fn style_cream(&self) -> Style {
        Style::default().fg(self.cream)
    }

    /// 正文样式。
    pub fn style_text(&self) -> Style {
        Style::default().fg(self.text)
    }

    /// 次要/提示文字样式。
    pub fn style_muted(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Logo 填充色（GUI 暖石白）。
    pub fn style_logo_fill(&self) -> Style {
        Style::default()
            .fg(self.logo_fill)
            .add_modifier(Modifier::BOLD)
    }

    /// 卡片标题样式。
    pub fn style_card_title(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(self.brand).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.cream)
        }
    }

    /// 卡片边框样式。
    pub fn style_border(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(self.brand)
        } else {
            Style::default().fg(self.border)
        }
    }

    /// 卡片背景/选中样式。
    pub fn style_card_focused(&self) -> Style {
        Style::default()
            .fg(self.brand)
            .bg(self.bg)
            .add_modifier(Modifier::BOLD)
    }

    /// 提示信息样式。
    pub fn style_hint(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// 成功信息样式。
    pub fn style_success(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// 警告信息样式。
    pub fn style_warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// 错误信息样式。
    pub fn style_error(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// 状态栏应用名样式。
    pub fn style_status_brand(&self) -> Style {
        Style::default().fg(self.brand).add_modifier(Modifier::BOLD)
    }

    /// 命令输入前缀样式。
    pub fn style_command_prefix(&self) -> Style {
        Style::default().fg(self.brand).add_modifier(Modifier::BOLD)
    }

    /// 命令输入边框样式。
    pub fn style_command_border(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// 根据 `use_icons` 返回图标或 ASCII 回退文本。
    pub fn icon_or_text(&self, icon: &str, fallback: &str) -> String {
        if self.use_icons {
            icon.to_string()
        } else {
            fallback.to_string()
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::load()
    }
}

/// 颜色支持级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorLevel {
    /// 24-bit 真色。
    TrueColor,
    /// 256 色。
    Indexed,
    /// 8/16 色 ANSI。
    Ansi,
}

/// 检测当前终端颜色支持级别。
fn detect_color_level() -> ColorLevel {
    if let Ok(force) = std::env::var("SOLOSOUL_CLI_TRUECOLOR") {
        if force == "1" {
            return ColorLevel::TrueColor;
        }
        if force == "0" {
            return ColorLevel::Ansi;
        }
    }

    if let Ok(ct) = std::env::var("COLORTERM") {
        if ct.eq_ignore_ascii_case("truecolor") || ct.eq_ignore_ascii_case("24bit") {
            return ColorLevel::TrueColor;
        }
    }

    if let Ok(term) = std::env::var("TERM") {
        if term.contains("256color") {
            return ColorLevel::Indexed;
        }
        if term == "dumb" || term == "linux" {
            return ColorLevel::Ansi;
        }
    }

    // 保守默认：使用 256 色，兼容性较好。
    ColorLevel::Indexed
}

/// 判断是否应使用 Unicode 图标。
///
/// 默认关闭，避免 emoji 或宽字符符号在不同终端出现显示问题。
fn should_use_icons() -> bool {
    if let Ok(v) = std::env::var("SOLOSOUL_CLI_ICONS") {
        return v == "1";
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_with_level() {
        let t = Theme::with_level(ColorLevel::Ansi, true);
        assert_eq!(t.brand, Color::LightBlue);
        assert!(t.use_icons);

        let t = Theme::with_level(ColorLevel::TrueColor, false);
        assert_eq!(t.brand, Color::Rgb(79, 168, 255));
        assert!(!t.use_icons);
    }

    #[test]
    fn test_icon_or_text() {
        let with_icon = Theme::with_level(ColorLevel::Indexed, true);
        assert_eq!(with_icon.icon_or_text(">", "[浏览]"), ">");

        let no_icon = Theme::with_level(ColorLevel::Indexed, false);
        assert_eq!(no_icon.icon_or_text(">", "[浏览]"), "[浏览]");
    }
}
