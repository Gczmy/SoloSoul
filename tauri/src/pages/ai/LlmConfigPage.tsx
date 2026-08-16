import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { BarChart3 } from 'lucide-react';
import { AiFeaturesCard } from '@/components/llm-config/AiFeaturesCard';
import { SystemPromptCard } from '@/components/llm-config/SystemPromptCard';
import { ProviderManagerPanel } from '@/components/llm-config/ProviderManagerPanel';
import { LocalEmbeddingsPanel } from '@/components/llm-config/LocalEmbeddingsPanel';
import { KnowledgeBaseCard } from '@/components/llm-config/KnowledgeBaseCard';
import { RiskAcceptanceDialog } from '@/components/llm-config/RiskAcceptanceDialog';
import { isMobilePlatformSync } from '@/lib/platform';
import { ICON_SIZE } from '@/lib/constants';
// P048: 全部状态与逻辑抽到 hook
import { useLlmConfigPage } from './useLlmConfigPage';

export function LlmConfigPage() {
  const {
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
  } = useLlmConfigPage();

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
