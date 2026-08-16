/**
 * LlmConfigPage 数据层 hook（P048 拆分：provider 管理 + 本地 embedding 全部逻辑与渲染分离）。
 */
import { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import i18n from '@/lib/i18n';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { listen } from '@tauri-apps/api/event';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { useConfirm } from '@/hooks/useConfirm';
import type { ProviderConfig } from '@/types/llmProvider';
import { isOllama } from '@/types/llmChat';
import { logger } from '@/lib/logger';

/**
 * P035: 云端 LLM 隐私确认标志（设备级 localStorage）——首次启用非本地 provider 前弹确认，
 * 用户同意后写入该标志，后续同一设备切换云端 provider 不再重复拦截。
 */
const LLM_CLOUD_PRIVACY_ACK = 'llm_cloud_privacy_accepted';

/** AI 功能中始终处于禁用状态的功能（UI 尚未提供开关） */
const ALWAYS_DISABLED_FEATURES = {
  smartFill: false,
  commandGen: false,
  naturalLanguageSearch: false,
} as const;

/**
 * 与后端 `llm_get_embed_models`（embed_model.rs `EmbedModelWithStatus`）实际序列化形状
 * 保持一致：`#[serde(flatten)]` 扁平字段 + snake_case + installed 标志。
 * 此前误设为嵌套 `{ info, installed }` + camelCase，真实数据到达后渲染
 * `m.info.id` 抛 TypeError → 整页卸载（页面无 ErrorBoundary）。
 */
export interface EmbedModelWithStatus {
  id: string;
  name: string;
  description: string;
  disk_size: string;
  dimensions: number;
  download_url: string;
  checksum: string;
  installed: boolean;
}

export function useLlmConfigPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const backPath = (location.state as { from?: string } | null)?.from || '/settings';
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { onError, onSuccess } = useToastError();
  const { requestConfirm, dialog: confirmDialog } = useConfirm();

  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [activeId, setActiveId] = useState<string>('');
  const [chatEnabled, setChatEnabled] = useState(false);
  const [includeSystemPrompt, setIncludeSystemPrompt] = useState(true);
  const [hasAcceptedRisk, setHasAcceptedRisk] = useState(false);
  const [loading, setLoading] = useState(true);
  const [showRiskDialog, setShowRiskDialog] = useState(false);
  const [rebuilding, setRebuilding] = useState(false);
  const [embeddingAvailable, setEmbeddingAvailable] = useState<boolean | null>(null);
  const [embedModels, setEmbedModels] = useState<EmbedModelWithStatus[]>([]);
  const [useLocalEmbedding, setUseLocalEmbedding] = useState(false);
  const [localModelId, setLocalModelId] = useState<string | null>(null);
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<number>(0);
  const [modelsLoading, setModelsLoading] = useState(false);

  useEffect(() => {
    if (!accountId) return;
    Promise.all([
      invoke<ProviderConfig[]>('llm_get_providers', { accountId: accountId }),
      invoke<{
        activeProviderId?: string;
        aiFeaturesEnabled?: { chat: boolean };
        includeSystemPrompt?: boolean;
        hasAcceptedRisk?: boolean;
        useLocalEmbedding?: boolean;
        localEmbedModelId?: string | null;
      }>('llm_get_config', { accountId: accountId }),
    ])
      .then(([provs, cfg]) => {
        setProviders(provs);
        if (cfg.activeProviderId) setActiveId(cfg.activeProviderId);
        if (cfg.aiFeaturesEnabled) setChatEnabled(cfg.aiFeaturesEnabled.chat);
        if (cfg.includeSystemPrompt !== undefined) setIncludeSystemPrompt(cfg.includeSystemPrompt);
        if (cfg.hasAcceptedRisk) setHasAcceptedRisk(true);
        if (cfg.useLocalEmbedding !== undefined) setUseLocalEmbedding(cfg.useLocalEmbedding);
        if (cfg.localEmbedModelId !== undefined) setLocalModelId(cfg.localEmbedModelId);
      })
      .catch((err) => logger.warn('[LLMConfig] Load providers failed:', err))
      .finally(() => setLoading(false));

    invoke<boolean>('llm_check_embedding_available', { accountId: accountId })
      .then((avail) => setEmbeddingAvailable(avail))
      .catch(() => setEmbeddingAvailable(false));

    loadEmbedModels();

    let unlisten: (() => void) | undefined;
    listen<{ modelId: string; progress: number }>('embed-download-progress', (event) => {
      if (event.payload.modelId === downloadingId) {
        setDownloadProgress(event.payload.progress);
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, [accountId, downloadingId]);

  const loadEmbedModels = async () => {
    setModelsLoading(true);
    try {
      const models = await invoke<EmbedModelWithStatus[]>('llm_get_embed_models');
      setEmbedModels(models);
    } catch (err) {
      // P227: 模型列表加载失败静默降级可接受（UI 仍有重试入口），但留痕。
      logger.warn('[LLMConfig] loadEmbedModels failed:', err);
    } finally {
      setModelsLoading(false);
    }
  };

  const handleDownloadModel = async (modelId: string) => {
    setDownloadingId(modelId);
    setDownloadProgress(0);
    try {
      await invoke('llm_download_embed_model', { modelId: modelId });
      onSuccess(t('settings:llm_model_downloaded'));
      await loadEmbedModels();
      if (!localModelId) {
        setLocalModelId(modelId);
        if (accountId) {
          await invoke('llm_set_local_embedding', {
            accountId: accountId,
            enabled: true,
            modelId: modelId,
          });
          setUseLocalEmbedding(true);
        }
      }
    } catch (e) {
      onError(e, t('settings:llm_model_download_failed'));
    } finally {
      setDownloadingId(null);
      setDownloadProgress(0);
    }
  };

  const handleDeleteModel = (modelId: string) => {
    requestConfirm(
      t('settings:llm_delete_model_title', { defaultValue: 'Delete model' }),
      t('settings:llm_confirm_delete_model', {
        defaultValue: 'Delete this local embedding model?',
      }),
      async () => {
        try {
          await invoke('llm_delete_embed_model', { modelId: modelId });
          onSuccess(t('settings:llm_model_deleted'));
          await loadEmbedModels();
          if (localModelId === modelId) {
            setLocalModelId(null);
            setUseLocalEmbedding(false);
            if (accountId) {
              await invoke('llm_set_local_embedding', {
                accountId: accountId,
                enabled: false,
                modelId: null,
              });
            }
          }
        } catch (e) {
          onError(e, t('settings:llm_delete_model_failed'));
        }
      },
      {
        confirmLabel: t('common:delete', { defaultValue: 'Delete' }),
        cancelLabel: t('common:cancel', { defaultValue: 'Cancel' }),
      },
    );
  };

  const handleToggleLocalEmbedding = async (enabled: boolean) => {
    if (!accountId) return;
    if (enabled && !localModelId && embedModels.length > 0) {
      const firstInstalled = embedModels.find((m) => m.installed);
      if (firstInstalled) {
        setLocalModelId(firstInstalled.id);
        await invoke('llm_set_local_embedding', {
          accountId: accountId,
          enabled: true,
          modelId: firstInstalled.id,
        });
      } else {
        onError(
          new Error(t('settings:llm_enable_local_first')),
          t('settings:llm_enable_local_failed'),
        );
        return;
      }
    } else {
      await invoke('llm_set_local_embedding', {
        accountId: accountId,
        enabled,
        modelId: localModelId,
      });
    }
    setUseLocalEmbedding(enabled);
  };

  const handleSelectLocalModel = async (modelId: string) => {
    if (!accountId) return;
    setLocalModelId(modelId);
    if (useLocalEmbedding) {
      await invoke('llm_set_local_embedding', {
        accountId: accountId,
        enabled: true,
        modelId: modelId,
      });
    }
  };

  const handleRebuildEmbeddings = async () => {
    if (!accountId) return;
    setRebuilding(true);
    try {
      const count = await invoke<number>('llm_rebuild_guide_embeddings', {
        accountId: accountId,
        language: i18n.language || 'zh-CN',
      });
      onSuccess(t('settings:llm_kb_rebuilt', { count: String(count) }));
    } catch (e) {
      onError(e, t('settings:llm_rebuild_kb'));
    } finally {
      setRebuilding(false);
    }
  };

  const applyActiveProvider = async (id: string) => {
    if (!accountId) return;
    const prevActive = activeId;
    setActiveId(id);
    try {
      await invoke('llm_set_active_provider', { accountId: accountId, providerId: id });
    } catch (err) {
      // P028-R1: 失败回滚需防竞态——仅当当前状态仍是本次操作写入的 id 时才回滚
      //（函数式比对），否则可能是后续更快的成功切换，误回滚会覆盖新状态。
      logger.warn('[LLMConfig] Set active provider failed:', err);
      setActiveId((cur) => (cur === id ? prevActive : cur));
      onError(err, t('settings:llm_set_active_failed'));
    }
  };

  const handleSetActive = (id: string) => {
    if (!accountId) return;
    const target = providers.find((p) => p.id === id);
    // P035: 首次启用云端（非 localhost/127.0.0.1）provider 前弹隐私确认；
    // 同意后写设备级标志，后续切换云端 provider 不再拦截（本地 LLM 服务器不拦截）。
    if (target && !isOllama(target.baseUrl) && !localStorage.getItem(LLM_CLOUD_PRIVACY_ACK)) {
      requestConfirm(
        t('settings:llm_cloud_privacy_confirm_title'),
        t('settings:llm_cloud_privacy_confirm_body', { name: target.name }),
        () => {
          localStorage.setItem(LLM_CLOUD_PRIVACY_ACK, '1');
          void applyActiveProvider(id);
        },
        { confirmLabel: t('common:confirm'), cancelLabel: t('common:cancel') },
      );
      return;
    }
    void applyActiveProvider(id);
  };

  const handleFeatureToggle = async () => {
    const next = !chatEnabled;
    if (!hasAcceptedRisk && next) {
      setShowRiskDialog(true);
      return;
    }
    setChatEnabled(next);
    if (accountId) {
      try {
        await invoke('llm_set_ai_features', {
          accountId: accountId,
          features: { chat: next, ...ALWAYS_DISABLED_FEATURES },
        });
      } catch (err) {
        // P028-R1: 失败回滚需防竞态——仅当当前状态仍是本次操作写入的 next 时才回滚
        //（函数式比对），连续快速切换时旧失败不得覆盖后一次成功操作。
        logger.warn('[LLMConfig] Set AI features failed:', err);
        setChatEnabled((cur) => (cur === next ? !next : cur));
        onError(err, t('settings:llm_set_features_failed'));
      }
    }
  };

  const handleAcceptRisk = async () => {
    if (!accountId) return;
    try {
      await invoke('llm_accept_risk', { accountId: accountId });
    } catch (err) {
      // P028-R1: accept_risk 失败不得置 hasAcceptedRisk/chatEnabled——
      // 否则后端未接受风险、UI 却放行 AI 功能（功能开关与后端真实状态不符）。
      logger.warn('[LLMConfig] Accept risk failed:', err);
      onError(err, t('settings:llm_accept_risk_failed'));
      return;
    }
    setHasAcceptedRisk(true);
    setShowRiskDialog(false);
    setChatEnabled(true);
    try {
      await invoke('llm_set_ai_features', {
        accountId: accountId,
        features: { chat: true, ...ALWAYS_DISABLED_FEATURES },
      });
    } catch (err) {
      // P028-R1: 启用功能失败需回滚 chatEnabled（函数式比对防竞态）
      logger.warn('[LLMConfig] Set features after risk accept failed:', err);
      setChatEnabled((cur) => (cur === true ? false : cur));
      onError(err, t('settings:llm_set_features_failed'));
    }
  };

  const handleSystemPromptToggle = async () => {
    const next = !includeSystemPrompt;
    setIncludeSystemPrompt(next);
    if (accountId) {
      try {
        await invoke('llm_set_system_prompt_switch', { accountId: accountId, enabled: next });
      } catch (err) {
        // P028-R1: 失败回滚需防竞态——仅当当前状态仍是本次操作写入的 next 时才回滚
        logger.warn('[LLMConfig] Set system prompt switch failed:', err);
        setIncludeSystemPrompt((cur) => (cur === next ? !next : cur));
        onError(err, t('settings:llm_set_prompt_failed'));
      }
    }
  };

  const handleSaveProvider = async (provider: ProviderConfig) => {
    if (!accountId) return;
    try {
      await invoke('llm_save_provider', { accountId: accountId, provider });
    } catch (err) {
      // N-4: 新外部 URL 登记需原生确认对话框；用户取消或校验失败时
      // 后端返回 Err——本地列表不得误报「保存成功」。取消属预期路径，
      // 其余失败给出 toast 提示（避免点击保存后无任何反馈）。
      logger.warn('[LLMConfig] Save provider failed:', err);
      const msg = err instanceof Error ? err.message : String(err);
      if (!msg.includes('已取消')) {
        onError(err, t('settings:llm_save_provider_failed'));
      }
      return;
    }
    setProviders((prev) => {
      const idx = prev.findIndex((p) => p.id === provider.id);
      const updated = { ...provider, apiKey: provider.apiKey ? '••••••••' : '' };
      if (idx >= 0) {
        const n = [...prev];
        n[idx] = updated;
        return n;
      }
      return [...prev, updated];
    });
    onSuccess(t('common:success'));
  };

  const handleDeleteProvider = (id: string) => {
    if (!accountId) return;
    requestConfirm(
      t('settings:llm_delete_provider_title', { defaultValue: 'Delete provider' }),
      t('settings:llm_delete_provider_body', {
        defaultValue: 'Delete this provider configuration?',
      }),
      async () => {
        try {
          await invoke('llm_delete_provider', { accountId: accountId, providerId: id });
        } catch (err) {
          // P007: 删除失败不得更新本地状态——否则后端未删、UI 已移除，重启后
          // 复现且误清 activeId 导致功能开关与实际不符；失败给出 toast 反馈。
          logger.warn('[LLMConfig] Delete provider failed:', err);
          onError(err, t('settings:llm_delete_provider_failed'));
          return;
        }
        setProviders((prev) => prev.filter((p) => p.id !== id));
        if (activeId === id) setActiveId('');
      },
      {
        confirmLabel: t('common:delete', { defaultValue: 'Delete' }),
        cancelLabel: t('common:cancel', { defaultValue: 'Cancel' }),
      },
    );
  };

  const handleTestConnection = async (provider: ProviderConfig, accId: string): Promise<string> => {
    let key = provider.apiKey;
    if (key === '••••••••') {
      key = await invoke<string>('llm_get_api_key', {
        accountId: accId,
        providerId: provider.id,
      });
    }
    const result = await invoke<string>('llm_test_provider', {
      baseUrl: provider.baseUrl,
      apiKey: key,
      model: provider.model,
      apiType: provider.apiType,
    });
    return t('settings:llm_test_ok') + ' "' + result.slice(0, 80) + '"';
  };

  return {
    navigate,
    backPath,
    t,
    accountId,
    confirmDialog,
    providers,
    activeId,
    chatEnabled,
    includeSystemPrompt,
    hasAcceptedRisk,
    loading,
    showRiskDialog,
    setShowRiskDialog,
    rebuilding,
    embeddingAvailable,
    embedModels,
    useLocalEmbedding,
    localModelId,
    downloadingId,
    downloadProgress,
    modelsLoading,
    handleDownloadModel,
    handleDeleteModel,
    handleToggleLocalEmbedding,
    handleSelectLocalModel,
    handleRebuildEmbeddings,
    handleSetActive,
    handleFeatureToggle,
    handleAcceptRisk,
    handleSystemPromptToggle,
    handleSaveProvider,
    handleDeleteProvider,
    handleTestConnection,
  };
}
