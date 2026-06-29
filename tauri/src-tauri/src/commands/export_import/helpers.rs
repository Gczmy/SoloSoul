use super::*;

// ── Internal helpers ──────────────────────────────────────────

pub struct ManifestData {
    pub salt_hex: String,
    pub has_attachments: bool,
    pub extra_files: Vec<String>,
}

// ── Cross-scope reference resolution ─────────────────────────

/// Build a set of all object IDs present in the imported package.
pub fn build_package_ids(payload: &serde_json::Value) -> std::collections::HashSet<String> {
    payload["objects"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|o| o["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Recursively scan a JSON value for RelationProperty references.
/// If a relation targets an object not in `package_ids`, downgrade it to a text remark.
pub fn resolve_value_references(
    value: &mut serde_json::Value,
    package_ids: &std::collections::HashSet<String>,
) {
    match value {
        serde_json::Value::Object(obj) => {
            // Check if this object looks like a RelationProperty
            let is_relation = obj
                .get("type")
                .or_else(|| obj.get("__type"))
                .or_else(|| obj.get("kind"))
                .and_then(|v| v.as_str())
                == Some("relation");
            if is_relation {
                let target_id = obj
                    .get("targetId")
                    .or_else(|| obj.get("id"))
                    .or_else(|| obj.get("objectId"))
                    .and_then(|v| v.as_str());
                if let Some(tid) = target_id {
                    if !package_ids.contains(tid) {
                        // Downgrade to text remark as per §4 cross-scope reference handling
                        *value = serde_json::Value::String(format!("[引用对象未导出: {}]", tid));
                        return;
                    }
                }
            }
            // Recurse into nested objects
            for (_k, v) in obj.iter_mut() {
                resolve_value_references(v, package_ids);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                resolve_value_references(item, package_ids);
            }
        }
        _ => {}
    }
}

/// Scan object properties and downgrade any cross-scope relation references.
pub fn resolve_cross_scope_references(
    properties: &mut serde_json::Value,
    package_ids: &std::collections::HashSet<String>,
) {
    if let Some(map) = properties.as_object_mut() {
        for (_key, value) in map.iter_mut() {
            resolve_value_references(value, package_ids);
        }
    }
}

pub fn read_manifest(file_path: &str) -> Result<ManifestData, String> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(import_err_with_detail("FILE_NOT_FOUND", file_path));
    }
    let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|_| import_err("INVALID_PACKAGE"))?;

    let mut entry = archive
        .by_name("manifest.json")
        .map_err(|_| import_err("MISSING_MANIFEST"))?;
    let mut buf = Vec::new();
    entry
        .read_to_end(&mut buf)
        .map_err(|e| format!("Read manifest: {}", e))?;
    let s = String::from_utf8_lossy(&buf);
    let v: serde_json::Value =
        serde_json::from_str(&s).map_err(|e| format!("Invalid manifest: {}", e))?;

    let extra_files: Vec<String> = v
        .get("extra_files")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(ManifestData {
        salt_hex: v["salt_hex"]
            .as_str()
            .ok_or(import_err("MISSING_SALT"))?
            .to_string(),
        has_attachments: v["has_attachments"].as_bool().unwrap_or(false),
        extra_files,
    })
}

/// ZIP 条目的最大解压大小限制（100 MB），防止 ZIP 炸弹 / OOM。
const MAX_ZIP_ENTRY_SIZE: u64 = 100 * 1024 * 1024;

/// 从 ZIP 中读取指定名称的文件内容，带大小限制。
///
/// # 安全
/// - 读取前检查 `entry.size()`（未压缩大小），超过 `MAX_ZIP_ENTRY_SIZE` 则拒绝。
/// - 使用 `.take()` 限制实际读取字节数，即使 `size()` 返回 0/错误也有第二道防线。
pub fn read_file_from_zip(file_path: &str, name: &str) -> Result<Vec<u8>, String> {
    let path = std::path::Path::new(file_path);
    let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Invalid ZIP".to_string())?;
    let entry = archive
        .by_name(name)
        .map_err(|_| format!("File not found: {}", name))?;

    // 检查 ZIP 条目声明的未压缩大小
    if entry.size() > MAX_ZIP_ENTRY_SIZE {
        return Err(format!(
            "ZIP entry '{}' is too large ({} bytes, max {} bytes)",
            name,
            entry.size(),
            MAX_ZIP_ENTRY_SIZE
        ));
    }

    let mut buf = Vec::new();
    // 使用 take() 作为第二道防线（即使 entry.size() 不准确）
    entry
        .take(MAX_ZIP_ENTRY_SIZE + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("Read {}: {}", name, e))?;

    Ok(buf)
}
