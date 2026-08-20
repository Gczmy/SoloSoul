/**
 * LlmConfigPage 数据层 hook（P048 拆分：provider 管理 + 本地 embedding 全部逻辑与渲染分离；
 * P021c 再拆：三域逻辑下沉 useLlmProviders / useLlmChatFeatureSettings / useLlmLocalEmbedding，
 * 本 hook 仅保留初始加载编排与导航/文案基础设施）。
 */
import { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { useConfirm } from '@/hooks/useConfirm';
import { useLlmProviders } from '@/hooks/useLlmProviders';
import { useLlmChatFeatureSettings } from '@/hooks/useLlmChatFeatureSettings';
import { useLlmLocalEmbedding } from '@/hooks/useLlmLocalEmbedding';
import { logger } from '@/lib/logger';
import type { ProviderConfig } from '@/types/llmProvider';

export function useLlmConfigPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const backPath = (location.state as { from?: string } | null)?.from || '/settings';
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { onError, onSuccess } = useToastError();
  const { requestConfirm, dialog: confirmDialog } = useConfirm();

  // 三域子 hook：provider 管理 / AI 功能开关 / 本地 embedding
  const providersApi = useLlmProviders({ accountId, t, onError, onSuccess, requestConfirm });
  const featureApi = useLlmChatFeatureSettings({ accountId, t, onError });
  const embeddingApi = useLlmLocalEmbedding({ accountId, t, onError, onSuccess, requestConfirm });

  const [loading, setLoading] = useState(true);

  // 初始加载：单次拉取 providers + 全量配置，分发到各域状态
  const { setProviders, setActiveId } = providersApi;
  const { setChatEnabled, setIncludeSystemPrompt, setHasAcceptedRisk } = featureApi;
  const { setUseLocalEmbedding, setLocalModelId } = embeddingApi;

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
  }, [
    accountId,
    setProviders,
    setActiveId,
    setChatEnabled,
    setIncludeSystemPrompt,
    setHasAcceptedRisk,
    setUseLocalEmbedding,
    setLocalModelId,
  ]);

  return {
    navigate,
    backPath,
    t,
    accountId,
    confirmDialog,
    loading,
    ...providersApi,
    ...featureApi,
    ...embeddingApi,
  };
}
