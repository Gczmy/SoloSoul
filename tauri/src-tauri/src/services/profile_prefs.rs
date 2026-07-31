//! Profile preferences 读-改-写共享实现。
//!
//! 多个命令/服务（LLM 配置、使用统计、对话、public_data_version、用户偏好）此前各自
//! 复制了同一段「load_profile → 解析 data → entry(preferences) → 写 key → 序列化 →
//! version+=1 → save_profile」样板（P028 去重）。此处收敛为单一实现。

use solosoul_vault::VaultStore;

/// 更新账户 Profile 的 `preferences` 段（读-改-写原子流程）。
///
/// - Profile 不存在时自动创建；
/// - `preferences` 键不存在时自动创建空对象；
/// - 闭包内以 `&mut Map` 形式写入任意 key；返回 `Err` 则中止保存；
/// - 保存前统一推进 `updated_at` 与 `version`。
pub fn update_profile_prefs(
    vault: &VaultStore,
    account_id: &str,
    update: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>) -> Result<(), String>,
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
    // 防御：preferences 若为异常非对象值，替换为空对象（与原 settings.rs 行为一致）
    if !prefs.is_object() {
        *prefs = serde_json::Value::Object(serde_json::Map::new());
    }
    let prefs_map = prefs.as_object_mut().ok_or("Invalid")?;
    update(prefs_map)?;
    profile.data = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    profile.updated_at = chrono::Utc::now();
    profile.version += 1;
    vault.save_profile(&profile)
}
