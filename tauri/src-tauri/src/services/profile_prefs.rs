//! Profile preferences 读-改-写共享实现。
//!
//! 多个命令/服务（LLM 配置、使用统计、对话、public_data_version、用户偏好）此前各自
//! 复制了同一段「load_profile → 解析 data → entry(preferences) → 写 key → 序列化 →
//! version+=1 → save_profile」样板（P028 去重）。此处收敛为对 vault 层原子 API 的
//! 薄转发。
//!
//! P006：原实现在服务层「load_profile（取锁→释放）→ 闭包改内存 → save_profile
//! （重新取锁）」跨两次独立锁获取，并发写者互相覆盖（lost update）。读-改-写已下沉为
//! `VaultStore::update_profile_prefs` 单次持锁原子实现，本函数保持原签名与语义不变。

use solosoul_vault::VaultStore;

/// 更新账户 Profile 的 `preferences` 段（原子读-改-写）。
///
/// 委托 `VaultStore::update_profile_prefs`——单次 `conn` 锁内完成读取→变更→保存，
/// 消除跨两次独立锁获取的并发覆盖：
///
/// - Profile 不存在时自动创建；
/// - `preferences` 键不存在时自动创建空对象；
/// - 闭包内以 `&mut Map` 形式写入任意 key；返回 `Err` 则中止并回滚保存；
/// - 保存前统一推进 `updated_at` 与 `version`。
pub fn update_profile_prefs(
    vault: &VaultStore,
    account_id: &str,
    update: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>) -> Result<(), String>,
) -> Result<(), String> {
    vault.update_profile_prefs(account_id, update)
}
