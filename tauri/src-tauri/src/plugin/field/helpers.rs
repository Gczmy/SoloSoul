/// 将 `address[0].street` 简化为 `address.street` 用于权限匹配
pub(crate) fn normalize_for_permission(field_id: &str) -> Option<String> {
    let mut result = String::with_capacity(field_id.len());
    let mut chars = field_id.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            // 跳过 [...] 内容
            let mut closed = false;
            for c in chars.by_ref() {
                if c == ']' {
                    closed = true;
                    break;
                }
                if !c.is_ascii_digit() {
                    return None;
                }
            }
            if !closed {
                return None;
            }
            continue;
        }
        if ch.is_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
            result.push(ch);
        } else {
            return None;
        }
    }
    Some(result)
}

/// 匹配权限模式：精确匹配、`*.prop` 后缀、`type.*` 前缀、`*` 通配
pub(crate) fn pattern_matches(pattern: &str, field: &str) -> bool {
    if pattern == "*" || pattern == field {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        if field == prefix || field.starts_with(&format!("{}.", prefix)) {
            return true;
        }
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        if field == suffix || field.ends_with(&format!(".{}", suffix)) {
            return true;
        }
    }
    false
}

/// 解析 `<typeId>[<index>].<prop>`
pub(crate) fn parse_indexed_field(field_id: &str) -> Option<(String, usize, String)> {
    let bracket_open = field_id.find('[')?;
    let bracket_close = field_id.find(']')?;
    if bracket_close < bracket_open || bracket_close + 1 >= field_id.len() {
        return None;
    }
    let type_id = &field_id[..bracket_open];
    let index_str = &field_id[bracket_open + 1..bracket_close];
    let index: usize = index_str.parse().ok()?;
    if !field_id[bracket_close + 1..].starts_with('.') {
        return None;
    }
    let prop_path = field_id[bracket_close + 2..].to_string();
    Some((type_id.to_string(), index, prop_path))
}

/// 解析 `<typeId>.<prop>`（不是 count 且不含下标）
pub(crate) fn parse_type_property(field_id: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = field_id.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let type_id = parts[0].to_string();
    let prop_path = parts[1..].join(".");
    Some((type_id, prop_path))
}

/// 从 JSON 属性中提取标量值（嵌套路径用 '.' 分隔）
pub(crate) fn extract_property(props: &serde_json::Value, prop_path: &str) -> String {
    let mut value = props;
    for key in prop_path.split('.') {
        if key.is_empty() {
            return String::new();
        }
        value = match value.get(key) {
            Some(v) => v,
            None => return String::new(),
        };
    }
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
