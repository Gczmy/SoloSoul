import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useShallow } from 'zustand/react/shallow';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import styles from './LlmStatsPage.module.css';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useAuthStore } from '@/stores/authStore';
import { useLlmStatsStore } from '@/stores/llmStatsStore';
import type { ProviderConfig } from '@/types/llmProvider';
import { ModelInfoCard } from '@/components/llm/ModelInfoCard';
import { StatsGrid } from '@/components/llm/StatsGrid';
import { AccountStatsCard } from '@/components/llm/AccountStatsCard';
import { TokenBreakdownCard } from '@/components/llm/TokenBreakdownCard';
import { DailySparklineCard } from '@/components/llm/DailySparklineCard';
import { ModelUsageCard } from '@/components/llm/ModelUsageCard';
import { BarChart3, RotateCcw } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

export function LlmStatsPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  // P022: useShallow 字段级选择——避免 store 无关字段翻转时整页重渲染
  const { stats, loading, loadStats, resetStats } = useLlmStatsStore(
    useShallow((s) => ({
      stats: s.stats,
      loading: s.loading,
      loadStats: s.loadStats,
      resetStats: s.resetStats,
    })),
  );
  const [activeProvider, setActiveProvider] = useState<{
    name: string;
    model: string;
    apiType: string;
  } | null>(null);
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [showResetDialog, setShowResetDialog] = useState(false);
  const [lastLoadTime, setLastLoadTime] = useState<string | undefined>();

  const backPath = (location.state as { from?: string } | null)?.from || '/settings';

  const lastUsedTime = useMemo(() => {
    const times = stats?.perModelStats?.map((m) => m.lastUsedTime).filter((t): t is string => !!t);
    if (!times || times.length === 0) return undefined;
    return times.reduce((max, cur) => (cur > max ? cur : max));
  }, [stats]);

  useEffect(() => {
    if (!accountId) return;
    (async () => {
      try {
        const cfg = await invoke<{ activeProviderId?: string }>('llm_get_config', { accountId: accountId });
        const providers = await invoke<ProviderConfig[]>('llm_get_providers', { accountId: accountId });
        const active = providers.find((p) => p.id === cfg.activeProviderId);
        if (active) {
          setActiveProvider({ name: active.name, model: active.model, apiType: active.apiType });
          let key = '';
          try {
            key = await invoke<string>('llm_get_api_key', { accountId: accountId, providerId: active.id });
          } catch {
            /* no key */
          }
          const online = await invoke<boolean>('llm_check_connection', {
            baseUrl: active.baseUrl,
            apiKey: key,
            model: active.model,
            apiType: active.apiType,
          });
          setIsOnline(online);
        }
      } catch {
        /* ignore */
      }
    })();
  }, [accountId]);

  useEffect(() => {
    if (accountId) {
      loadStats(accountId).then(() => setLastLoadTime(new Date().toISOString()));
    }
  }, [accountId, loadStats]);

  const handleReset = async () => {
    if (!accountId) return;
    await resetStats(accountId);
    setShowResetDialog(false);
    if (accountId) {
      await loadStats(accountId);
      setLastLoadTime(new Date().toISOString());
    }
  };

  if (loading && !stats) {
    return (
      <AppShell title={t('settings:llm_stats_page_title')} onBack={() => navigate(backPath)}>
        <LoadingPlaceholder variant="base" />
      </AppShell>
    );
  }

  const hasData = stats && (stats.usageCount > 0 || stats.totalTokens > 0);

  return (
    <AppShell title={t('settings:llm_stats_page_title')} onBack={() => navigate(backPath)}>
      <PageContainer variant="medium" gap="section" className={styles.page}>
        {/* Model Info */}
        <section>
          <SectionTitle>{t('settings:llm_current_model')}</SectionTitle>
          <ModelInfoCard
            providerName={activeProvider?.name || '—'}
            modelName={activeProvider?.model || '—'}
            apiType={activeProvider?.apiType || '—'}
            isOnline={isOnline}
            t={t}
          />
        </section>

        {!hasData ? (
          <div style={{ textAlign: 'center', padding: '48px 24px' }}>
            <BarChart3
              size={ICON_SIZE['5xl']}
              style={{ marginBottom: 16, opacity: 0.25, color: 'var(--text-tertiary)' }}
            />
            <p style={{ fontSize: 'var(--text-sm)', color: 'var(--text-tertiary)' }}>
              {t('settings:llm_no_data')}
            </p>
            <p
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                marginTop: 4,
              }}
            >
              {t('settings:llm_no_data_hint')}
            </p>
          </div>
        ) : (
          <>
            {/* Session Stats */}
            <section>
              <SectionTitle>{t('settings:llm_session_stats')}</SectionTitle>
              <StatsGrid
                usageCount={stats?.usageCount || 0}
                totalTokens={stats?.totalTokens || 0}
                promptTokens={stats?.promptTokens || 0}
                completionTokens={stats?.completionTokens || 0}
                modelUsages={stats?.perModelStats || []}
                lastLoadTime={lastLoadTime}
                lastUsedTime={lastUsedTime}
                t={t}
              />
            </section>

            {/* Account Stats */}
            <section>
              <SectionTitle>{t('settings:llm_account_stats')}</SectionTitle>
              <AccountStatsCard
                usageCount={stats?.usageCount || 0}
                totalTokens={stats?.totalTokens || 0}
                modelUsages={stats?.perModelStats || []}
                t={t}
              />
            </section>

            {/* Token Breakdown */}
            {(stats?.promptTokens || 0) > 0 || (stats?.completionTokens || 0) > 0 ? (
              <section>
                <SectionTitle>{t('settings:llm_token_breakdown')}</SectionTitle>
                <TokenBreakdownCard
                  sessionPrompt={stats?.promptTokens || 0}
                  sessionCompletion={stats?.completionTokens || 0}
                  accountPrompt={stats?.promptTokens || 0}
                  accountCompletion={stats?.completionTokens || 0}
                  t={t}
                />
              </section>
            ) : null}

            {/* Daily Sparkline */}
            {(stats?.dailyStats?.length || 0) > 0 ? (
              <section>
                <SectionTitle>{t('settings:llm_daily_trend')}</SectionTitle>
                <DailySparklineCard daily={stats?.dailyStats || []} t={t} />
              </section>
            ) : null}

            {/* Model Usage */}
            {(stats?.perModelStats?.length || 0) > 0 ? (
              <section>
                <SectionTitle>{t('settings:llm_model_ranking')}</SectionTitle>
                <ModelUsageCard perModel={stats?.perModelStats || []} t={t} />
              </section>
            ) : null}

            {/* Reset */}
            <section>
              <Button
                variant="danger"
                onClick={() => setShowResetDialog(true)}
                style={{ width: '100%' }}
              >
                <RotateCcw size={ICON_SIZE.sm} style={{ marginRight: 4 }} />{' '}
                {t('settings:llm_reset_stats')}
              </Button>
            </section>
          </>
        )}
      </PageContainer>

      {/* Reset Confirm Dialog */}
      <Dialog
        isOpen={showResetDialog}
        onClose={() => setShowResetDialog(false)}
        title={t('settings:llm_reset_confirm_title')}
      >
        <p
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            marginBottom: 16,
          }}
        >
          {t('settings:llm_reset_confirm_desc')}
        </p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={() => setShowResetDialog(false)}>
            {t('common:cancel')}
          </Button>
          <Button variant="danger" onClick={handleReset}>
            {t('settings:llm_reset_btn')}
          </Button>
        </div>
      </Dialog>
    </AppShell>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3
      style={{
        fontSize: 'var(--text-body-sm)',
        fontWeight: 600,
        color: 'var(--text-secondary)',
        textTransform: 'uppercase',
        letterSpacing: '0.05em',
        marginBottom: 8,
        paddingLeft: 4,
      }}
    >
      {children}
    </h3>
  );
}
