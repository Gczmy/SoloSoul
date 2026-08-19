use solosoul_vault::VaultStore;

// =============================================================================
// Provider helpers（llm_chat 命令已移除，本文件仅保留被 rag.rs 复用的内部工具）
// =============================================================================

use super::*;

/// P102：判断 `base_url` 是否为当前账户「已登记」的 provider 地址。
///
/// 已登记 = 内置默认 provider 的 base_url ∪ 已保存进加密 config 的 provider base_url。
/// `llm_send_message_stream` 在发起外连前必须通过本检查，确保聊天内容只能发往
/// 用户在设置中登记过的目标，XSS 无法借 LLM 通道把 Vault 数据外传到任意地址。
pub(crate) fn is_registered_provider_url(config: &LlmConfig, base_url: &str) -> bool {
    let normalized = base_url.trim_end_matches('/');
    if default_providers()
        .iter()
        .any(|p| p.base_url.trim_end_matches('/') == normalized)
    {
        return true;
    }
    config
        .providers
        .iter()
        .any(|p| p.base_url.trim_end_matches('/') == normalized)
}

// Helper: load providers with decrypted keys (internal reuse)
// P019: 合并逻辑收敛到 llm::merge_providers_with_keys，此处保留薄封装（返回真实密钥）。
pub fn load_providers_with_keys(
    vault: &VaultStore,
    account_id: &str,
) -> Result<Vec<ProviderWithKey>, String> {
    super::merge_providers_with_keys(vault, account_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with(providers: Vec<ProviderConfig>) -> LlmConfig {
        LlmConfig {
            providers,
            active_provider_id: None,
            ai_features_enabled: AiFeatures::default(),
            has_accepted_risk: false,
            include_system_prompt: true,
            use_local_embedding: false,
            local_embed_model_id: None,
        }
    }

    #[test]
    fn test_is_registered_builtin_default() {
        let cfg = cfg_with(vec![]);
        assert!(is_registered_provider_url(
            &cfg,
            "https://api.openai.com/v1"
        ));
        assert!(is_registered_provider_url(
            &cfg,
            "http://localhost:11434/v1"
        ));
        // 尾斜杠归一化
        assert!(is_registered_provider_url(
            &cfg,
            "https://api.openai.com/v1/"
        ));
    }

    #[test]
    fn test_is_registered_saved_provider() {
        let cfg = cfg_with(vec![ProviderConfig {
            id: "custom".into(),
            name: "Custom".into(),
            base_url: "https://my-proxy.example.com/v1".into(),
            model: "m".into(),
            is_enabled: true,
            is_built_in: false,
            api_type: ApiType::OpenAI,
            embedding_model: None,
        }]);
        assert!(is_registered_provider_url(
            &cfg,
            "https://my-proxy.example.com/v1"
        ));
        assert!(is_registered_provider_url(
            &cfg,
            "https://my-proxy.example.com/v1/"
        ));
    }

    #[test]
    fn test_is_registered_rejects_unrelated_url() {
        let cfg = cfg_with(vec![]);
        assert!(!is_registered_provider_url(
            &cfg,
            "https://evil.example.com/v1"
        ));
        assert!(!is_registered_provider_url(
            &cfg,
            "https://api.openai.com/v2"
        ));
        assert!(!is_registered_provider_url(&cfg, "https://api.openai.com"));
    }
}
