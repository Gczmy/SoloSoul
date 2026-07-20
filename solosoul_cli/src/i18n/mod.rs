//! 国际化（i18n）模块。
//!
//! 基于 fluent-rs，支持运行时语言切换。
//! .ftl 翻译文件在编译时嵌入（`include_str!`），无外部运行时依赖。

use std::collections::HashMap;

use fluent_bundle::{FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

// ---------------------------------------------------------------------------
// I18n 管理器
// ---------------------------------------------------------------------------

pub struct I18n {
    /// 当前 locale（如 "zh-CN"、"en-US"）。
    pub locale: String,
    /// 预先编译好的 locale → FluentBundle 映射。
    bundles: HashMap<String, FluentBundle<FluentResource>>,
    /// 支持的所有 locale 代码列表。
    available: Vec<String>,
}

impl I18n {
    /// 创建一个新的 I18n 实例。
    ///
    /// `locale` 为期望的语言代码（如 "zh-CN"、"en-US"），若不受支持则回退到
    /// `available` 中的第一个（注册顺序 = 优先级顺序）。
    pub fn new(locale: &str) -> Self {
        let bundles = build_bundles();
        let available: Vec<String> = bundles.keys().cloned().collect();

        let resolved = resolve_locale(locale, &available);

        I18n {
            locale: resolved,
            bundles,
            available,
        }
    }

    /// 运行时切换语言。
    pub fn set_locale(&mut self, locale: &str) {
        let resolved = resolve_locale(locale, &self.available);
        if self.bundles.contains_key(&resolved) {
            self.locale = resolved;
        }
    }

    /// 根据 key 获取本地化字符串，无参数。
    pub fn t(&self, key: &str) -> String {
        self.t_args(key, &[])
    }

    /// 根据 key 获取本地化字符串，支持插值参数。
    ///
    /// `args` 格式：`&[("name", "value"), ("count", "3")]`
    /// 在 .ftl 文件中对应 `{$name}`、`{$count}`。
    pub fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let bundle = match self.bundles.get(&self.locale) {
            Some(b) => b,
            None => return key.to_string(),
        };

        let msg = match bundle.get_message(key) {
            Some(m) => m,
            None => return key.to_string(),
        };

        let pattern = match msg.value() {
            Some(p) => p,
            None => return key.to_string(),
        };

        let mut fluent_args = fluent_bundle::FluentArgs::new();
        for (k, v) in args {
            fluent_args.set(k.to_string(), FluentValue::from(*v));
        }

        let mut errors = Vec::new();
        let value = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
        value.to_string()
    }
}

// ---------------------------------------------------------------------------
// 内置翻译资源
// ---------------------------------------------------------------------------

/// 编译期嵌入所有 .ftl 文件并构建 FluentBundle。
fn build_bundles() -> HashMap<String, FluentBundle<FluentResource>> {
    let mut map = HashMap::new();

    let locales: &[(&str, &str)] = &[
        ("zh-CN", include_str!("zh-CN.ftl")),
        ("en-US", include_str!("en-US.ftl")),
    ];

    for (locale, ftl_content) in locales {
        let langid = parse_langid(locale);
        let resource =
            FluentResource::try_new(ftl_content.to_string()).expect("Invalid FTL content");
        let mut bundle = FluentBundle::new(vec![langid]);
        // FluentBundle::set_use_isolating(false) 可避免某些语言中插值两侧被加上隔离符
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .expect("Failed to add FTL resource");
        map.insert(locale.to_string(), bundle);
    }

    map
}

/// 解析语言标识符，解析失败时使用 en-US 作为安全回退。
fn parse_langid(s: &str) -> LanguageIdentifier {
    s.parse().unwrap_or_else(|_| {
        let fallback: LanguageIdentifier = "en-US".parse().expect("en-US is a valid langid");
        fallback
    })
}

/// 在可用 locale 列表中解析最匹配的 locale。
///
/// 先尝试不区分大小写的精确匹配（"zh-cn" → "zh-CN"），
/// 再尝试语言前缀匹配（"zh" → "zh-CN", "en" → "en-US"），
/// 最后回退到 "en-US"。
fn resolve_locale(requested: &str, available: &[String]) -> String {
    let lower_req = requested.to_lowercase();
    let lower_avail: Vec<String> = available.iter().map(|s| s.to_lowercase()).collect();

    // 1. 不区分大小写的精确匹配（"zh-cn" → "zh-CN"）
    if let Some(idx) = lower_avail.iter().position(|a| a == &lower_req) {
        return available[idx].clone();
    }

    // 2. 语言前缀匹配（例如 "zh" → "zh-CN", "en" → "en-US"）
    //    同样不区分大小写
    let lang = lower_req.split(['-', '_']).next().unwrap_or("");
    if !lang.is_empty() {
        for (avail, lower) in available.iter().zip(lower_avail.iter()) {
            if lower.starts_with(lang) {
                return avail.clone();
            }
        }
    }

    // 3. 回退到 en-US
    "en-US".to_string()
}

// ---------------------------------------------------------------------------
// t!() 宏 — 简化 i18n 调用
// ---------------------------------------------------------------------------

/// 从 `I18n` 实例获取本地化字符串。
///
/// # 用法
///
/// ```ignore
/// t!(i18n, "hello")
/// t!(i18n, "file-count", count = "3")
/// ```
#[macro_export]
macro_rules! t {
    ($i18n:expr, $key:literal) => {
        $i18n.t($key)
    };
    ($i18n:expr, $key:literal, $($k:ident = $v:expr),+ $(,)?) => {
        $i18n.t_args($key, &[$( (stringify!($k), &$v.to_string()) ),+])
    };
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_new_zh() {
        let i18n = I18n::new("zh-CN");
        assert_eq!(i18n.locale, "zh-CN");
    }

    #[test]
    fn test_i18n_new_en() {
        let i18n = I18n::new("en-US");
        assert_eq!(i18n.locale, "en-US");
    }

    #[test]
    fn test_i18n_fallback() {
        // 不支持的 locale 应回退到 en-US
        let i18n = I18n::new("ja-JP");
        assert_eq!(i18n.locale, "en-US");
    }

    #[test]
    fn test_i18n_t_key_exists() {
        let i18n = I18n::new("en-US");
        let val = i18n.t("app-title");
        assert_eq!(val, "SoloSoul CLI");
    }

    #[test]
    fn test_i18n_t_key_missing() {
        let i18n = I18n::new("en-US");
        // 不存在的 key 应返回 key 本身
        let val = i18n.t("nonexistent-key");
        assert_eq!(val, "nonexistent-key");
    }

    #[test]
    fn test_set_locale_switch() {
        let mut i18n = I18n::new("en-US");
        assert_eq!(i18n.locale, "en-US");
        i18n.set_locale("zh-CN");
        assert_eq!(i18n.locale, "zh-CN");
        // 确认翻译切换
        let val = i18n.t("app-title");
        assert_eq!(val, "SoloSoul CLI"); // 标题英文不变
    }

    #[test]
    fn test_t_macro() {
        let i18n = I18n::new("en-US");
        let val = t!(i18n, "app-title");
        assert_eq!(val, "SoloSoul CLI");
    }

    #[test]
    fn test_resolve_locale_case_insensitive() {
        // "zh-cn"（全小写）应匹配 "zh-CN"
        let i18n = I18n::new("zh-cn");
        assert_eq!(i18n.locale, "zh-CN");
    }

    #[test]
    fn test_resolve_locale_prefix_case_insensitive() {
        // "ZH"（全大写）应通过前缀匹配 "zh-CN"
        let i18n = I18n::new("ZH");
        assert_eq!(i18n.locale, "zh-CN");
    }

    #[test]
    fn test_t_macro_with_args() {
        let i18n = I18n::new("zh-CN");
        // 假设有一个带参数的 key
        let val = t!(i18n, "backup-created", name = "test", size = "1.2 KB");
        assert!(val.contains("test"));
        assert!(val.contains("1.2 KB"));
    }
}
