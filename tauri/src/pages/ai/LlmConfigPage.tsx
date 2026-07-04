import { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import i18n from '@/lib/i18n';
import { invoke } from '@tauri-apps/api/core';
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
import {
  ProviderManagerPanel,
  type ProviderConfig,
} from '@/components/llm-config/ProviderManagerPanel';
import { LocalEmbeddingsPanel } from '@/components/llm-config/LocalEmbeddingsPanel';
import { KnowledgeBaseCard } from '@/components/llm-config/KnowledgeBaseCard';
import { RiskAcceptanceDialog } from '@/components/llm-config/RiskAcceptanceDialog';
import { ICON_SIZE } from '@/lib/constants';

/** AI 功能中始终处于禁用状态的功能（UI 尚未提供开关） */
const ALWAYS_DISABLED_FEATURES = {
  smartFill: false,
  commandGen: false,
  naturalLanguageSearch: false,
} as const;

interface EmbedModelInfo {
  id: string;
  name: string;
  description: string;
  diskSize: string;
  dimensions: number;
  downloadUrl: string;
  checksum: string;
}

interface EmbedModelWithStatus {
  info: EmbedModelInfo;
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
      invoke<ProviderConfig[]>('llm_get_providers', { accountId }),
      invoke<{
        activeProviderId?: string;
        aiFeaturesEnabled?: { chat: boolean }
        includeSystemPrompt?: boolean;
        hasAcceptedRisk?: boolean;
        useLocalEmbedding?: boolean;
        localEmbedModelId?: string | null;
      }>('llm_get_config', { accountId }),
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
      .catch((err) => console.warn('[LLMConfig] Load providers failed:', err))
      .finally(() => setLoading(false));

    invoke<boolean>('llm_check_embedding_available', { accountId })
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
    } catch {
      /* silently ignore */
    } finally {
      setModelsLoading(false);
    }
  };

  const handleDownloadModel = async (modelId: string) => {
    setDownloadingId(modelId);
    setDownloadProgress(0);
    try {
      await invoke('llm_download_embed_model', { modelId });
      onSuccess(t('settings:llm_model_downloaded'));
      await loadEmbedModels();
      if (!localModelId) {
        setLocalModelId(modelId);
        if (accountId) {
          await invoke('llm_set_local_embedding', { accountId, enabled: true, modelId });
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
      t('settings:llm_delete_model_title') || 'Delete model',
      t('settings:llm_confirm_delete_model') || 'Delete this local embedding model?',
      async () => {
        try {
          await invoke('llm_delete_embed_model', { modelId });
          onSuccess(t('settings:llm_model_deleted'));
          await loadEmbedModels();
          if (localModelId === modelId) {
            setLocalModelId(null);
            setUseLocalEmbedding(false);
            if (accountId) {
              await invoke('llm_set_local_embedding', { accountId, enabled: false, modelId: null });
            }
          }
        } catch (e) {
          onError(e, t('settings:llm_delete_model_failed'));
        }
      },
      { confirmLabel: t('common:delete') || 'Delete', cancelLabel: t('common:cancel') || 'Cancel' },
    );
  };

  const handleToggleLocalEmbedding = async (enabled: boolean) => {
    if (!accountId) return;
    if (enabled && !localModelId && embedModels.length > 0) {
      const firstInstalled = embedModels.find((m) => m.installed);
      if (firstInstalled) {
        setLocalModelId(firstInstalled.info.id);
        await invoke('llm_set_local_embedding', {
          accountId,
          enabled: true,
          modelId: firstInstalled.info.id,
        });
      } else {
        onError(
          new Error(t('settings:llm_enable_local_first')),
          t('settings:llm_enable_local_failed'),
        );
        return;
      }
    } else {
      await invoke('llm_set_local_embedding', { accountId, enabled, modelId: localModelId });
    }
    setUseLocalEmbedding(enabled);
  };

  const handleSelectLocalModel = async (modelId: string) => {
    if (!accountId) return;
    setLocalModelId(modelId);
    if (useLocalEmbedding) {
      await invoke('llm_set_local_embedding', { accountId, enabled: true, modelId });
    }
  };

  const handleRebuildEmbeddings = async () => {
    if (!accountId) return;
    setRebuilding(true);
    try {
      const count = await invoke<number>('llm_rebuild_guide_embeddings', {
        accountId,
        language: i18n.language || 'zh-CN',
      });
      onSuccess(t('settings:llm_kb_rebuilt', { count: String(count) }));
    } catch (e) {
      onError(e, t('settings:llm_rebuild_kb'));
    } finally {
      setRebuilding(false);
    }
  };

  const handleSetActive = async (id: string) => {
    if (!accountId) return;
    setActiveId(id);
    await invoke('llm_set_active_provider', { accountId, providerId: id }).catch((err) =>
      console.warn('[LLMConfig] Set active provider failed:', err),
    );
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
        accountId,
        features: { chat: next, ...ALWAYS_DISABLED_FEATURES },
      }).catch((err) =>
        console.warn('[LLMConfig] Set AI features failed:', err),
      );
  };

  const handleAcceptRisk = async () => {
    if (!accountId) return;
    await invoke('llm_accept_risk', { accountId }).catch((err) =>
      console.warn('[LLMConfig] Accept risk failed:', err),
    );
    setHasAcceptedRisk(true);
    setShowRiskDialog(false);
    setChatEnabled(true);
    await invoke('llm_set_ai_features', {
      accountId,        features: { chat: true, ...ALWAYS_DISABLED_FEATURES },
    }).catch((err) =>
      console.warn('[LLMConfig] Set features after risk accept failed:', err),
    );
  };

  const handleSystemPromptToggle = async () => {
    const next = !includeSystemPrompt;
    setIncludeSystemPrompt(next);
    if (accountId)
      await invoke('llm_set_system_prompt_switch', { accountId, enabled: next }).catch((err) =>
        console.warn('[LLMConfig] Set system prompt switch failed:', err),
      );
  };

  const handleSaveProvider = async (provider: ProviderConfig) => {
    if (!accountId) return;
    await invoke('llm_save_provider', { accountId, provider }).catch((err) =>
      console.warn('[LLMConfig] Save provider failed:', err),
    );
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
      t('settings:llm_delete_provider_title') || 'Delete provider',
      t('settings:llm_delete_provider_body') || 'Delete this provider configuration?',
      async () => {
        await invoke('llm_delete_provider', { accountId, providerId: id }).catch((err) =>
          console.warn('[LLMConfig] Delete provider failed:', err),
        );
        setProviders((prev) => prev.filter((p) => p.id !== id));
        if (activeId === id) setActiveId('');
      },
      { confirmLabel: t('common:delete') || 'Delete', cancelLabel: t('common:cancel') || 'Cancel' },
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
