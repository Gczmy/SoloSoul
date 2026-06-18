//! LLM Context Service — 系统提示词构建（模式 B，后端构建）
//! §28 Phase 2.1
//! 负责在 Rust 端查询 Vault 数据、组装 7 Section 系统提示词。
//! 隐私过滤在 Rust 端强制完成，不可被绕过。

use once_cell::sync::Lazy;
use solosoul_vault::{ObjectSummary, VaultStore};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

// ── 缓存层（内存）────────────────────────────────────────────

struct CachedPrompt {
    static_prompt: String, // Section 1-5（不含实时统计）
    #[allow(dead_code)]
    created_at: Instant,
}

static PROMPT_CACHE: Lazy<Mutex<HashMap<String, CachedPrompt>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// ── 长度限制常量 ─────────────────────────────────────────────

const MAX_OBJECTS_PER_TYPE: usize = 3;
const MAX_PROPERTIES_PER_OBJECT: usize = 8;
const MAX_VALUE_LENGTH: usize = 100;
const MAX_SYSTEM_PROMPT_CHARS: usize = 1500;

// ── 主构建入口 ───────────────────────────────────────────────

pub fn build_context(
    account_id: &str,
    vault: &VaultStore,
    usage_count: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    language: &str,
) -> Result<String, String> {
    // 1. 读取 public_data_version
    let public_data_version = load_public_data_version(vault, account_id)?;

    // 2. 构建缓存键
    let cache_key = format!("{}_{}", account_id, public_data_version);

    // 3. 检查内存缓存
    {
        let cache = PROMPT_CACHE.lock().map_err(|e| e.to_string())?;
        if let Some(cached) = cache.get(&cache_key) {
            let stats_section =
                build_section6_stats(usage_count, prompt_tokens, completion_tokens, total_tokens);
            let full = format!(
                "{}\n\n{}\n\n{}",
                cached.static_prompt,
                stats_section,
                build_section7_guidelines()
            );
            return Ok(trim_to_limit(&full, MAX_SYSTEM_PROMPT_CHARS));
        }
    }

    // 4. 缓存未命中：重新构建静态部分（Section 1-5）
    let static_prompt = build_static_prompt(account_id, vault, language)?;

    // 5. 存入内存缓存
    {
        let mut cache = PROMPT_CACHE.lock().map_err(|e| e.to_string())?;
        cache.insert(
            cache_key.clone(),
            CachedPrompt {
                static_prompt: static_prompt.clone(),
                created_at: Instant::now(),
            },
        );
    }

    // 6. 追加实时统计（Section 6）+ 行为规范（Section 7），并截断
    let stats_section =
        build_section6_stats(usage_count, prompt_tokens, completion_tokens, total_tokens);
    let guidelines = build_section7_guidelines();
    let full = format!("{}\n\n{}\n\n{}", static_prompt, stats_section, guidelines);
    Ok(trim_to_limit(&full, MAX_SYSTEM_PROMPT_CHARS))
}

/// 清除缓存。在账户切换或 Vault 锁定时调用。
pub fn clear_cache() {
    if let Ok(mut cache) = PROMPT_CACHE.lock() {
        cache.clear();
    }
}

// ── 静态提示词构建（Section 1-5）────────────────────────────

fn build_static_prompt(
    account_id: &str,
    vault: &VaultStore,
    language: &str,
) -> Result<String, String> {
    let mut sections: Vec<String> = Vec::new();

    sections.push(format!(
        "【Section 1: AI 身份定义】\n{}",
        build_section1_identity()
    ));
    sections.push(format!(
        "【Section 2: 软件信息】\n{}",
        build_section2_software_info(language)
    ));

    let section3 = build_section3_public_objects(vault, account_id)?;
    if !section3.is_empty() && !section3.starts_with('（') {
        sections.push(format!("【Section 3: 用户公开对象数据】\n{}", section3));
    }

    let section4 = build_section4_preferences(vault, account_id)?;
    if !section4.is_empty() && !section4.starts_with('（') {
        sections.push(format!("【Section 4: 偏好设置】\n{}", section4));
    }

    sections.push(format!(
        "【Section 5: 已安装插件】\n{}",
        build_section5_plugins()
    ));

    Ok(sections.join("\n\n"))
}

// ── 各 Section 构建器 ───────────────────────────────────────

fn build_section1_identity() -> String {
    "你是 SoloSoul（独灵）的 AI 助手 Solon，由 SoloSoul 团队开发。\n\
     你是用户的个人智能助手，了解用户的个人信息（仅限用户主动分享的部分）。\n\
     你的回答应当简洁、准确、有帮助。"
        .to_string()
}

fn build_section2_software_info(language: &str) -> String {
    let app_version = env!("CARGO_PKG_VERSION");
    let platform = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    };
    format!(
        "当前 SoloSoul 版本：{}\n平台：{}\n界面语言：{}",
        app_version, platform, language
    )
}

fn build_section3_public_objects(vault: &VaultStore, account_id: &str) -> Result<String, String> {
    let objects = vault
        .list_objects(account_id, None, None, None, false, false)
        .map_err(|e| format!("List objects: {}", e))?;

    let public_objects: Vec<&ObjectSummary> = objects
        .iter()
        .filter(|o| o.sensitivity_level == "public" && !o.is_deleted)
        .collect();

    if public_objects.is_empty() {
        return Ok("（用户尚未公开任何对象数据）".to_string());
    }

    // 按 collection_type 分组
    let mut by_type: HashMap<String, Vec<&ObjectSummary>> = HashMap::new();
    for obj in public_objects {
        by_type
            .entry(obj.collection_type.clone())
            .or_default()
            .push(obj);
    }

    let mut lines: Vec<String> = Vec::new();
    for (type_name, objs) in by_type.iter().take(3) {
        // 最多 3 种类型
        let display_name = type_display_name(type_name);
        let mut type_lines: Vec<String> = Vec::new();

        for obj in objs.iter().take(MAX_OBJECTS_PER_TYPE) {
            // 每类型最多 3 个对象
            let prop_entries = extract_properties(&obj.properties);
            if prop_entries.is_empty() {
                type_lines.push(format!("  - {}（无属性）", obj.name));
            } else {
                type_lines.push(format!("  - {}：{}", obj.name, prop_entries.join("、")));
            }
        }

        if !type_lines.is_empty() {
            lines.push(format!("- {}：\n{}", display_name, type_lines.join("\n")));
        }
    }

    Ok(lines.join("\n"))
}

fn extract_properties(properties: &serde_json::Value) -> Vec<String> {
    let mut entries = Vec::new();
    if let Some(map) = properties.as_object() {
        for (k, v) in map.iter().take(MAX_PROPERTIES_PER_OBJECT) {
            let str_val = match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => (if *b { "是" } else { "否" }).to_string(),
                _ => continue,
            };
            let truncated = if str_val.len() > MAX_VALUE_LENGTH {
                format!("{}…", &str_val[..MAX_VALUE_LENGTH])
            } else {
                str_val
            };
            let label = property_key_to_label(k);
            entries.push(format!("{}: {}", label, truncated));
        }
    }
    entries
}

fn build_section4_preferences(vault: &VaultStore, account_id: &str) -> Result<String, String> {
    let profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p,
        _ => return Ok("（无特殊偏好设置）".to_string()),
    };

    let data: serde_json::Value = match serde_json::from_slice(&profile.data) {
        Ok(d) => d,
        Err(_) => return Ok("（无特殊偏好设置）".to_string()),
    };

    let mut items: Vec<String> = Vec::new();
    if let Some(prefs) = data.get("preferences") {
        if let Some(theme) = prefs.get("theme").and_then(|v| v.as_str()) {
            items.push(format!("主题：{}", theme));
        }
        if let Some(lang) = prefs.get("language").and_then(|v| v.as_str()) {
            items.push(format!("语言：{}", lang));
        }
        if let Some(accent) = prefs.get("accentColor").and_then(|v| v.as_str()) {
            items.push(format!("主题色：{}", accent));
        }
        if let Some(auto_lock) = prefs.get("autoLockTimeoutMinutes").and_then(|v| v.as_i64()) {
            items.push(format!("自动锁定：{} 分钟", auto_lock));
        }
    }

    if items.is_empty() {
        Ok("（无特殊偏好设置）".to_string())
    } else {
        Ok(items.join("\n"))
    }
}

fn build_section5_plugins() -> String {
    // Plugin context is intentionally omitted until the installed plugin list
    // is exposed to the LLM context service.
    "（暂无已安装插件）".to_string()
}

fn build_section6_stats(
    usage_count: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
) -> String {
    format!(
        "【Section 6: 使用统计】\n\
         - 累计使用次数：{} 次\n\
         - Prompt Token 估算：{}\n\
         - Completion Token 估算：{}\n\
         - 总 Token 估算：{}",
        usage_count, prompt_tokens, completion_tokens, total_tokens
    )
}

fn build_section7_guidelines() -> String {
    "【Section 7: 行为规范】\n\
     1. 使用与用户提问相同的语言回答\n\
     2. 区分\"插件\"（功能扩展）和\"对象\"（用户数据）\n\
     3. 敏感/受限/关键数据需要重新验证密码，无法直接访问\n\
     4. 无法访问用户本地数据时，建议用户手动查找而非编造\n\
     5. 不泄露用户数据给插件或外部服务\n\
     6. 用户询问功能使用方法时，基于软件信息回答"
        .to_string()
}

// ── public_data_version ─────────────────────────────────────

fn load_public_data_version(vault: &VaultStore, account_id: &str) -> Result<u64, String> {
    let profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p,
        _ => return Ok(0),
    };

    let data: serde_json::Value = match serde_json::from_slice(&profile.data) {
        Ok(d) => d,
        Err(_) => return Ok(0),
    };

    let version = data
        .get("preferences")
        .and_then(|p| p.get("llmPublicDataVersion"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    Ok(version)
}

/// 递增 public_data_version。由 object_create / object_update 在检测到 public 级别变更时调用。
pub fn bump_public_data_version(vault: &VaultStore, account_id: &str) -> Result<u64, String> {
    let current = load_public_data_version(vault, account_id)?;
    let next = current + 1;
    save_public_data_version(vault, account_id, next)?;
    Ok(next)
}

fn save_public_data_version(
    vault: &VaultStore,
    account_id: &str,
    version: u64,
) -> Result<(), String> {
    let mut profile = match vault.load_profile(account_id) {
        Ok(Some(p)) => p,
        Ok(None) => solosoul_vault::Profile::new_with_id(account_id, account_id, Vec::new()),
        Err(e) => return Err(format!("Load: {}", e)),
    };

    let mut data: serde_json::Value = if profile.data.is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(&profile.data).map_err(|e| format!("Parse: {}", e))?
    };

    let prefs = data
        .as_object_mut()
        .ok_or("Invalid")?
        .entry("preferences".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    prefs["llmPublicDataVersion"] = serde_json::Value::Number(serde_json::Number::from(version));

    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}

// ── Helpers ─────────────────────────────────────────────────

fn type_display_name(type_id: &str) -> String {
    let clean = type_id.strip_prefix("__preset_").unwrap_or(type_id);
    let mut result = String::new();
    for (i, c) in clean.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            result.push(' ');
        }
        if c == '_' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            let first = chars
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default();
            first + &chars.as_str().to_lowercase()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn property_key_to_label(key: &str) -> String {
    let mut result = String::new();
    for (i, c) in key.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            result.push(' ');
        }
        if c == '_' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            let first = chars
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default();
            first + &chars.as_str().to_lowercase()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn trim_to_limit(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    // 找到不超过 max_chars 的最近合法字符边界（避免在中文字符中间切片 panic）
    let mut end = max_chars;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut cut = &text[..end];
    if let Some(pos) = cut.rfind("\n\n") {
        cut = &text[..pos];
    } else if let Some(pos) = cut.rfind('\n') {
        if pos > max_chars * 7 / 10 {
            cut = &text[..pos];
        }
    }
    format!("{}\n\n（上下文过长，部分内容已省略）", cut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_vault::{Profile, VaultConfig, VaultStore};
    use tempfile::TempDir;

    fn setup_vault() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config =
            VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    #[test]
    fn test_build_section1_identity() {
        let identity = build_section1_identity();
        assert!(identity.contains("SoloSoul"));
        assert!(identity.contains("Solon"));
    }

    #[test]
    fn test_build_section2_software_info() {
        let info = build_section2_software_info("zh-CN");
        assert!(info.contains("SoloSoul"));
        assert!(info.contains("zh-CN"));
        assert!(info.contains("版本"));
    }

    #[test]
    fn test_build_section5_plugins() {
        let plugins = build_section5_plugins();
        assert!(plugins.contains("暂无已安装插件"));
    }

    #[test]
    fn test_build_section6_stats() {
        let stats = build_section6_stats(42, 1000, 2000, 3000);
        assert!(stats.contains("42"));
        assert!(stats.contains("1000"));
        assert!(stats.contains("2000"));
        assert!(stats.contains("3000"));
    }

    #[test]
    fn test_build_section7_guidelines() {
        let guidelines = build_section7_guidelines();
        assert!(guidelines.contains("Section 7"));
        assert!(guidelines.contains("敏感"));
    }

    #[test]
    fn test_type_display_name() {
        assert_eq!(type_display_name("note"), "Note");
        assert_eq!(type_display_name("travelDocument"), "Travel Document");
        assert_eq!(
            type_display_name("__preset_financialRecord"),
            "Financial Record"
        );
        assert_eq!(type_display_name("identity_card"), "Identity Card");
    }

    #[test]
    fn test_property_key_to_label() {
        assert_eq!(property_key_to_label("fullName"), "Full Name");
        assert_eq!(property_key_to_label("date_of_birth"), "Date Of Birth");
        assert_eq!(property_key_to_label("key"), "Key");
    }

    #[test]
    fn test_trim_to_limit_no_trim_needed() {
        let text = "short text";
        assert_eq!(trim_to_limit(text, 100), text);
    }

    #[test]
    fn test_trim_to_limit_trims_at_char_boundary() {
        let text = "这是一个很长的中文字符串，需要被截断。这里还有更多内容来确保总长度超过限制。";
        let result = trim_to_limit(text, 30);
        assert!(result.len() < text.len());
        assert!(result.contains("省略"));
    }

    #[test]
    fn test_trim_to_limit_trims_at_double_newline() {
        let text = "line1\n\nline2\n\nline3";
        let result = trim_to_limit(text, 15);
        assert!(result.contains("line1"));
        assert!(!result.contains("line3"));
    }

    #[test]
    fn test_extract_properties() {
        let props = serde_json::json!({
            "name": "Alice",
            "age": 30,
            "active": true,
            "nested": {"ignored": true}
        });
        let entries = extract_properties(&props);
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().any(|e| e.contains("Alice")));
        assert!(entries.iter().any(|e| e.contains("30")));
    }

    #[test]
    fn test_extract_properties_truncates_long_values() {
        let long_value = "a".repeat(200);
        let props = serde_json::json!({"content": long_value});
        let entries = extract_properties(&props);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].ends_with('…'));
    }

    #[test]
    fn test_load_public_data_version_no_profile() {
        let (vault, _dir) = setup_vault();
        let version = load_public_data_version(&vault, "test_account").unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_save_and_load_public_data_version() {
        let (vault, _dir) = setup_vault();
        let profile = Profile::new_with_id("test_account", "Test", Vec::new());
        vault.save_profile(&profile).unwrap();

        save_public_data_version(&vault, "test_account", 5).unwrap();
        let version = load_public_data_version(&vault, "test_account").unwrap();
        assert_eq!(version, 5);
    }

    #[test]
    fn test_bump_public_data_version() {
        let (vault, _dir) = setup_vault();
        let profile = Profile::new_with_id("test_account", "Test", Vec::new());
        vault.save_profile(&profile).unwrap();

        let v1 = bump_public_data_version(&vault, "test_account").unwrap();
        assert_eq!(v1, 1);

        let v2 = bump_public_data_version(&vault, "test_account").unwrap();
        assert_eq!(v2, 2);
    }

    #[test]
    fn test_build_static_prompt_with_empty_data() {
        let (vault, _dir) = setup_vault();
        let prompt = build_static_prompt("test_account", &vault, "zh-CN").unwrap();
        assert!(prompt.contains("Section 1"));
        assert!(prompt.contains("Section 2"));
        assert!(prompt.contains("Section 5"));
        // No public objects, Section 3 may be omitted
    }

    #[test]
    fn test_build_static_prompt_with_public_object() {
        let (vault, _dir) = setup_vault();
        let obj = solosoul_vault::ObjectRecord {
            contract_type_id: None,
            id: "obj-1".to_string(),
            account_id: "test_account".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: "My Note".to_string(),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: serde_json::json!({"content": "hello"}),
            property_labels: None,
            sensitivity_level: "public".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            version: 1,
        };
        vault.save_object(&obj).unwrap();

        let prompt = build_static_prompt("test_account", &vault, "zh-CN").unwrap();
        assert!(prompt.contains("Section 3"));
        assert!(prompt.contains("My Note"));
    }

    #[test]
    fn test_clear_cache() {
        // Just verify it doesn't panic
        clear_cache();
    }
}
