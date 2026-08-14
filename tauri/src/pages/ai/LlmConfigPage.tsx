import { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import i18n from '@/lib/i18n';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { listen } from '@tauri-apps/api/event';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';

import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { useConfirm } from '@/hooks/useConfirm';
import { BarChart3 } from 'lucide-react';
import { AiFeaturesCard } from '@/components/llm-config/AiFeaturesCard';
import { SystemPromptCard } from '@/components/llm-config/SystemPromptCard';
import { ProviderManagerPanel } from '@/components/llm-config/ProviderManagerPanel';
import type { ProviderConfig } from '@/types/llmProvider';
import { LocalEmbeddingsPanel } from '@/components/llm-config/LocalEmbeddingsPanel';
import { KnowledgeBaseCard } from '@/components/llm-config/KnowledgeBaseCard';
import { RiskAcceptanceDialog } from '@/components/llm-config/RiskAcceptanceDialog';
import { isMobilePlatformSync } from '@/lib/platform';
import { isOllama } from '@/types/llmChat';
import { logger } from '@/lib/logger';
import { ICON_SIZE } from '@/lib/constants';

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
interface EmbedModelWithStatus {
  id: string;
  name: string;
  description: string;
  disk_size: string;
  dimensions: number;
  download_url: string;
  checksum: string;
  installed: boolean;
}

export function LlmConfigPage() {
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
    setActiveId(id);
    await invoke('llm_set_active_provider', { accountId: accountId, providerId: id }).catch((err) =>
      logger.warn('[LLMConfig] Set active provider failed:', err),
    );
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
    if (accountId)
      await invoke('llm_set_ai_features', {
        accountId: accountId,
        features: { chat: next, ...ALWAYS_DISABLED_FEATURES },
      }).catch((err) => logger.warn('[LLMConfig] Set AI features failed:', err));
  };

  const handleAcceptRisk = async () => {
    if (!accountId) return;
    await invoke('llm_accept_risk', { accountId: accountId }).catch((err) =>
      logger.warn('[LLMConfig] Accept risk failed:', err),
    );
    setHasAcceptedRisk(true);
    setShowRiskDialog(false);
    setChatEnabled(true);
    await invoke('llm_set_ai_features', {
      accountId: accountId,
      features: { chat: true, ...ALWAYS_DISABLED_FEATURES },
    }).catch((err) => logger.warn('[LLMConfig] Set features after risk accept failed:', err));
  };

  const handleSystemPromptToggle = async () => {
    const next = !includeSystemPrompt;
    setIncludeSystemPrompt(next);
    if (accountId)
      await invoke('llm_set_system_prompt_switch', { accountId: accountId, enabled: next }).catch(
        (err) => logger.warn('[LLMConfig] Set system prompt switch failed:', err),
      );
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

  return (
    <AppShell title={t('settings:llm_config')} onBack={() => navigate(backPath)}>
      {confirmDialog}
      <PageContainer variant="xs" gap="default">
        {!hasAcceptedRisk && (
          <Card>
            <p
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                lineHeight: 1.5,
              }}
            >
              <span style={{ color: '#e67e22' }}>⚠</span> {t('settings:ai_risk_notice')}
            </p>
          </Card>
        )}

        <AiFeaturesCard chatEnabled={chatEnabled} onToggle={handleFeatureToggle} />

        <SystemPromptCard checked={includeSystemPrompt} onToggle={handleSystemPromptToggle} />

        <ProviderManagerPanel
          providers={providers}
          activeId={activeId}
          loading={loading}
          accountId={accountId}
          onSetActive={handleSetActive}
          onSaveProvider={handleSaveProvider}
          onDeleteProvider={handleDeleteProvider}
          onTestConnection={handleTestConnection}
        />

        {!isMobilePlatformSync() && (
          <LocalEmbeddingsPanel
            useLocalEmbedding={useLocalEmbedding}
            localModelId={localModelId}
            embedModels={embedModels}
            downloadingId={downloadingId}
            downloadProgress={downloadProgress}
            modelsLoading={modelsLoading}
            onToggle={handleToggleLocalEmbedding}
            onSelectModel={handleSelectLocalModel}
            onDownload={handleDownloadModel}
            onDelete={handleDeleteModel}
          />
        )}

        <KnowledgeBaseCard
          embeddingAvailable={embeddingAvailable}
          rebuilding={rebuilding}
          onRebuild={handleRebuildEmbeddings}
        />

        <Card
          interactive
          onClick={() => navigate('/settings/llm/stats', { state: { from: '/settings/llm' } })}
        >
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <BarChart3 size={ICON_SIZE.xl} color="var(--accent-primary)" />
              <div>
                <span style={{ fontSize: 'var(--text-sm)', fontWeight: 500 }}>
                  {t('settings:llm_stats_title')}
                </span>
                <div
                  style={{
                    fontSize: 'var(--text-badge)',
                    color: 'var(--text-tertiary)',
                    marginTop: 1,
                  }}
                >
                  {t('settings:llm_stats_desc')}
                </div>
              </div>
            </div>
            <span style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-md)' }}>›</span>
          </div>
        </Card>

        <RiskAcceptanceDialog
          open={showRiskDialog}
          onClose={() => setShowRiskDialog(false)}
          onAccept={handleAcceptRisk}
        />
      </PageContainer>
    </AppShell>
  );
}
