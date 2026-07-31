use solosoul_vault::VaultStore;

// =============================================================================
// Provider helpers（llm_chat 命令已移除，本文件仅保留被 rag.rs 复用的内部工具）
// =============================================================================

use super::*;

// Helper: load providers with decrypted keys (internal reuse)
pub fn load_providers_with_keys(
    vault: &VaultStore,
    account_id: &str,
) -> Result<Vec<ProviderWithKey>, String> {
    let config = load_config(vault, account_id)?;
    let keys = load_api_keys(vault, account_id)?;
    let mut defaults = default_providers();
    for saved in &config.providers {
        if let Some(d) = defaults.iter_mut().find(|d| d.id == saved.id) {
            d.name = saved.name.clone();
            d.base_url = saved.base_url.clone();
            d.model = saved.model.clone();
            d.is_enabled = saved.is_enabled;
            d.api_type = saved.api_type.clone();
            d.embedding_model = saved.embedding_model.clone();
            d.api_key = keys.get(&saved.id).cloned().unwrap_or_default();
        } else {
            defaults.push(ProviderWithKey {
                id: saved.id.clone(),
                name: saved.name.clone(),
                base_url: saved.base_url.clone(),
                model: saved.model.clone(),
                is_enabled: saved.is_enabled,
                is_built_in: false,
                api_key: keys.get(&saved.id).cloned().unwrap_or_default(),
                api_type: saved.api_type.clone(),
                embedding_model: saved.embedding_model.clone(),
            });
        }
    }
    Ok(defaults)
}
