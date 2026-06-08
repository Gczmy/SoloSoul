import { useEffect, useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { useAuthStore } from '@/stores/authStore';
import { useLlmStatsStore } from '@/stores/llmStatsStore';
import { ModelInfoCard } from '@/components/llm/ModelInfoCard';
import { StatsGrid } from '@/components/llm/StatsGrid';
import { AccountStatsCard } from '@/components/llm/AccountStatsCard';
import { TokenBreakdownCard } from '@/components/llm/TokenBreakdownCard';
import { DailySparklineCard } from '@/components/llm/DailySparklineCard';
import { ModelUsageCard } from '@/components/llm/ModelUsageCard';
import { BarChart3, RotateCcw } from 'lucide-react';

interface ProviderConfig { id: string; name: string; baseUrl: string; model: string; isEnabled: boolean; isBuiltIn: boolean; apiKey: string; apiType: 'openAI' | 'anthropic'; }

export function LlmStatsPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { stats, loading, loadStats, resetStats } = useLlmStatsStore();
  const [activeProvider, setActiveProvider] = useState<{ name: string; model: string; apiType: string } | null>(null);
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [showResetDialog, setShowResetDialog] = useState(false);

  const backPath = (location.state as { from?: string } | null)?.from || '/settings';

  useEffect(() => {
    if (!accountId) return;
    (async () => {
      try {
        const cfg = await invoke<{ activeProviderId?: string }>('llm_get_config', { accountId });
        const providers = await invoke<ProviderConfig[]>('llm_get_providers', { accountId });
        const active = providers.find((p) => p.id === cfg.activeProviderId);
        if (active) {
          setActiveProvider({ name: active.name, model: active.model, apiType: active.apiType });
          let key = '';
          try { key = await invoke<string>('llm_get_api_key', { accountId, providerId: active.id }); } catch { /* no key */ }
          const online = await invoke<boolean>('llm_check_connection', {
            baseUrl: active.baseUrl, apiKey: key, model: active.model, apiType: active.apiType,
          });
          setIsOnline(online);
        }
      } catch { /* ignore */ }
    })();
  }, [accountId]);

  useEffect(() => {
    if (accountId) {
      loadStats(accountId);
    }
  }, [accountId, loadStats]);

  const handleReset = async () => {
    if (!accountId) return;
    await resetStats(accountId);
    setShowResetDialog(false);
    if (accountId) {
      await loadStats(accountId);
    }
  };

  if (loading && !stats) {
    return (
      <AppShell title="LLM 使用统计" onBack={() => navigate(backPath)}>
        <div style={{ maxWidth: 640, margin: '0 auto', textAlign: 'center', padding: '48px 24px' }}>
          <BarChart3 size={48} style={{ marginBottom: 16, opacity: 0.3 }} />
          <p style={{ color: 'var(--text-secondary)' }}>{t('common:loading')}</p>
        </div>
      </AppShell>
    );
  }

  const hasData = stats && (stats.usageCount > 0 || stats.totalTokens > 0);

  return (
    <AppShell title="LLM 使用统计" onBack={() => navigate(backPath)}>
      <div style={{ maxWidth: 640, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 20, padding: '0 16px 32px' }}>
        {/* Model Info */}
        <section>
          <SectionTitle>当前模型</SectionTitle>
          <ModelInfoCard
            providerName={activeProvider?.name || '—'}
            modelName={activeProvider?.model || '—'}
            providerType={activeProvider?.apiType || '—'}
            isOnline={isOnline}
          />
        </section>

        {!hasData ? (
          <div style={{ textAlign: 'center', padding: '48px 24px' }}>
            <BarChart3 size={48} style={{ marginBottom: 16, opacity: 0.25, color: 'var(--text-tertiary)' }} />
            <p style={{ fontSize: 14, color: 'var(--text-tertiary)' }}>暂无使用数据</p>
            <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginTop: 4 }}>开始 AI 对话后将自动统计</p>
          </div>
        ) : (
          <>
            {/* Session Stats */}
            <section>
              <SectionTitle>会话统计</SectionTitle>
              <StatsGrid
                usageCount={stats?.usageCount || 0}
                totalTokens={stats?.totalTokens || 0}
                promptTokens={stats?.promptTokens || 0}
                completionTokens={stats?.completionTokens || 0}
                modelUsages={stats?.perModelStats || []}
              />
            </section>

            {/* Account Stats */}
            <section>
              <SectionTitle>账户累计</SectionTitle>
              <AccountStatsCard
                usageCount={stats?.usageCount || 0}
                totalTokens={stats?.totalTokens || 0}
                modelUsages={stats?.perModelStats || []}
              />
            </section>

            {/* Token Breakdown */}
            {(stats?.promptTokens || 0) > 0 || (stats?.completionTokens || 0) > 0 ? (
              <section>
                <SectionTitle>Token 分解</SectionTitle>
                <TokenBreakdownCard
                  sessionPrompt={stats?.promptTokens || 0}
                  sessionCompletion={stats?.completionTokens || 0}
                  accountPrompt={stats?.promptTokens || 0}
                  accountCompletion={stats?.completionTokens || 0}
                />
              </section>
            ) : null}

            {/* Daily Sparkline */}
            {(stats?.dailyStats?.length || 0) > 0 ? (
              <section>
                <SectionTitle>每日趋势（近14天）</SectionTitle>
                <DailySparklineCard daily={stats?.dailyStats || []} />
              </section>
            ) : null}

            {/* Model Usage */}
            {(stats?.perModelStats?.length || 0) > 0 ? (
              <section>
                <SectionTitle>模型使用排行</SectionTitle>
                <ModelUsageCard perModel={stats?.perModelStats || []} />
              </section>
            ) : null}

            {/* Reset */}
            <section>
              <Button
                variant="secondary"
                onClick={() => setShowResetDialog(true)}
                style={{ width: '100%', color: '#e74c3c', borderColor: '#e74c3c' }}
              >
                <RotateCcw size={14} style={{ marginRight: 4 }} /> 重置统计
              </Button>
            </section>
          </>
        )}
      </div>

      {/* Reset Confirm Dialog */}
      <Dialog
        isOpen={showResetDialog}
        onClose={() => setShowResetDialog(false)}
        title="重置统计"
      >
        <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 16 }}>
          确定要重置所有 LLM 使用统计吗？此操作不可恢复。
        </p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={() => setShowResetDialog(false)}>
            {t('common:cancel')}
          </Button>
          <Button onClick={handleReset} style={{ background: '#e74c3c' }}>
            重置
          </Button>
        </div>
      </Dialog>
    </AppShell>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3 style={{
      fontSize: 13,
      fontWeight: 600,
      color: 'var(--text-secondary)',
      textTransform: 'uppercase',
      letterSpacing: '0.05em',
      marginBottom: 8,
      paddingLeft: 4,
    }}>
      {children}
    </h3>
  );
}
