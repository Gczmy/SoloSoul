import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { PluginCard } from '@/components/plugin/PluginCard';
import { PluginConsentDialog } from '@/components/plugin/PluginConsentDialog';
import { PluginResultPanel } from '@/components/plugin/PluginResultPanel';
import { PluginDialog } from '@/components/plugin/PluginDialog';
import { PluginRunParamsDialog } from '@/components/plugin/PluginRunParamsDialog';
import { usePluginStore } from '@/stores/pluginStore';
import { pluginCommands, PluginParam, PluginTier } from '@/lib/plugin';
import { useToastError } from '@/hooks/useToastError';
import styles from './PluginDashboardPage.module.css';

type Tab = 'all' | 'installed' | 'running' | 'logs';
const TIERS: PluginTier[] = ['p0', 'p1', 'p2', 'p3', 'p4'];

export function PluginDashboardPage() {
  const navigate = useNavigate();
  const { t, i18n } = useTranslation(['plugin', 'settings', 'common']);
  const [activeTab, setActiveTab] = useState<Tab>('all');
  const [pendingRun, setPendingRun] = useState<{
    pluginId: string;
    pluginName: string;
    params: PluginParam[];
  } | null>(null);

  const {
    marketPlugins,
    installedPlugins,
    runningPlugins,
    selectedTier,
    enabledTiers,
    isLoadingMarket,
    isLoadingInstalled,
    error,
    loadMarket,
    loadInstalled,
    setSelectedTier,
    installPlugin,
    updatePlugin,
    uninstallPlugin,
    runPlugin,
    stopPlugin,
    clearPluginOutput,
    resolveDialog,
    clearError,
  } = usePluginStore();

  const pluginOutputs = useMemo(
    () =>
      Object.entries(runningPlugins).sort(([, a], [, b]) => b.startTime - a.startTime),
    [runningPlugins],
  );

  const { onError } = useToastError();

  useEffect(() => {
    if (error) {
      onError(error, t('plugin:operation_failed', { defaultValue: 'Plugin operation failed' }));
      clearError();
    }
  }, [error, onError, clearError, t]);

  useEffect(() => {
    loadMarket();
    loadInstalled();
  }, [loadMarket, loadInstalled]);

  const installedMap = useMemo(() => {
    const map: Record<string, (typeof installedPlugins)[number]> = {};
    for (const p of installedPlugins) {
      map[p.id] = p;
    }
    return map;
  }, [installedPlugins]);

  const pendingConsents = useMemo(() => {
    const out = [];
    for (const plugin of Object.values(runningPlugins)) {
      if (plugin.completed) continue;
      out.push(...plugin.consentRequests);
    }
    return out;
  }, [runningPlugins]);

  const pendingDialogs = useMemo(() => {
    const out = [];
    for (const plugin of Object.values(runningPlugins)) {
      if (plugin.completed) continue;
      out.push(...plugin.dialogRequests);
    }
    return out;
  }, [runningPlugins]);

  const displayedPlugins = useMemo(() => {
    switch (activeTab) {
      case 'installed':
        return marketPlugins.filter((p) => p.installedVersion);
      case 'running':
        return marketPlugins.filter(
          (p) => runningPlugins[p.pluginId] && !runningPlugins[p.pluginId].completed,
        );
      case 'all':
      default: {
        let list = marketPlugins;
        if (selectedTier !== 'all') {
          list = list.filter((p) => p.tier === selectedTier);
        } else {
          list = list.filter((p) => enabledTiers.includes(p.tier));
        }
        return list;
      }
    }
  }, [marketPlugins, runningPlugins, activeTab, selectedTier, enabledTiers]);

  const handleRun = async (pluginId: string) => {
    const info = marketPlugins.find((p) => p.pluginId === pluginId);
    if (!info) return;
    const locale = i18n.language?.startsWith('zh') ? 'zh' : 'en';
    const name = info.registryEntry.i18n?.[locale]?.name ?? info.registryEntry.name;
    const params = info.registryEntry.params?.length
      ? info.registryEntry.params
      : installedMap[pluginId]?.params ?? [];
    if (params.length > 0) {
      setPendingRun({ pluginId, pluginName: name, params });
      return;
    }
    await runPlugin(pluginId, name);
  };

  const handleRunWithParams = async (values: Record<string, string>) => {
    if (!pendingRun) return;
    const { pluginId, pluginName } = pendingRun;
    setPendingRun(null);
    await runPlugin(pluginId, pluginName, values);
  };

  const handleConsentApprove = async (requestId: string) => {
    await pluginCommands.consentResponse(requestId, true);
  };

  const handleConsentDeny = async (requestId: string) => {
    await pluginCommands.consentResponse(requestId, false);
  };

  const activeRunning = Object.entries(runningPlugins).filter(([, plugin]) => !plugin.completed);

  return (
    <AppShell
      title={t('settings:items.plugins', { defaultValue: 'Plugins' })}
      onBack={() => navigate('/settings')}
    >
      <div className={styles.container}>
        <Card className={styles.tabsCard}>
          <div className={styles.tabs}>
            {(['all', 'installed', 'running', 'logs'] as Tab[]).map((tab) => (
              <button
                key={tab}
                className={`${styles.tab} ${activeTab === tab ? styles.tabActive : ''}`}
                onClick={() => setActiveTab(tab)}
              >
                {t(`plugin:tab_${tab}`, { defaultValue: tab })}
                {tab === 'installed' && (
                  <span className={styles.tabBadge}>{installedPlugins.length}</span>
                )}
                {tab === 'running' && activeRunning.length > 0 && (
                  <span className={styles.tabBadge}>{activeRunning.length}</span>
                )}
              </button>
            ))}
          </div>
        </Card>

        {activeTab === 'all' && (
          <Card className={styles.tierCard}>
            <div className={styles.tierChips}>
              <button
                className={`${styles.tierChip} ${selectedTier === 'all' ? styles.tierChipActive : ''}`}
                onClick={() => setSelectedTier('all')}
              >
                {t('plugin:tier_all', { defaultValue: 'All' })}
              </button>
              {TIERS.map((tier) => {
                const enabled = enabledTiers.includes(tier);
                return (
                  <button
                    key={tier}
                    className={`${styles.tierChip} ${selectedTier === tier ? styles.tierChipActive : ''} ${!enabled ? styles.tierChipDisabled : ''}`}
                    onClick={() => enabled && setSelectedTier(tier)}
                    disabled={!enabled}
                    title={
                      enabled
                        ? tier.toUpperCase()
                        : t('plugin:tier_coming_soon', { defaultValue: 'Coming soon' })
                    }
                  >
                    {tier.toUpperCase()}
                  </button>
                );
              })}
            </div>
          </Card>
        )}

        {activeTab === 'logs' ? (
          <PluginLogPanel />
        ) : (
          <div className={styles.list}>
            {isLoadingMarket || isLoadingInstalled ? (
              <div className={styles.loading}>
                {t('common:loading', { defaultValue: 'Loading...' })}
              </div>
            ) : displayedPlugins.length === 0 ? (
              <div className={styles.empty}>
                {t('plugin:empty_list', { defaultValue: 'No plugins found' })}
              </div>
            ) : (
              displayedPlugins.map((info) => (
                <PluginCard
                  key={info.pluginId}
                  info={info}
                  manifest={installedMap[info.pluginId]}
                  isRunning={!!runningPlugins[info.pluginId] && !runningPlugins[info.pluginId].completed}
                  onInstall={() => installPlugin(info.pluginId, info.registryEntry.latestVersion)}
                  onUpdate={() => updatePlugin(info.pluginId)}
                  onUninstall={() => uninstallPlugin(info.pluginId)}
                  onRun={() => handleRun(info.pluginId)}
                />
              ))
            )}
          </div>
        )}

        {pluginOutputs.map(([pluginId, plugin]) => (
          <Card key={pluginId} className={styles.outputCard}>
            <div className={styles.outputHeader}>
              <h4>{plugin.pluginName}</h4>
              <div className={styles.outputActions}>
                {plugin.completed ? (
                  <>
                    <span className={styles.completedStatus}>
                      {plugin.error
                        ? t('plugin:status_failed', { defaultValue: 'Failed' })
                        : t('plugin:status_completed', { defaultValue: 'Completed' })}
                    </span>
                    <button className={styles.clearBtn} onClick={() => clearPluginOutput(pluginId)}>
                      {t('plugin:clear', { defaultValue: 'Clear' })}
                    </button>
                  </>
                ) : (
                  <button className={styles.stopBtn} onClick={() => stopPlugin(pluginId)}>
                    {t('plugin:stop', { defaultValue: 'Stop' })}
                  </button>
                )}
              </div>
            </div>
            <div className={styles.logs}>
              {plugin.logs.slice(-10).map((log) => (
                <div key={log.id} className={styles.logLine}>
                  <span className={styles.logLevel} data-level={log.level}>
                    {log.level}
                  </span>
                  <span className={styles.logMessage}>{log.message}</span>
                </div>
              ))}
            </div>
            {plugin.error && (
              <div className={styles.errorHint}>{plugin.error}</div>
            )}
            <PluginResultPanel results={plugin.results} />
          </Card>
        ))}
      </div>

      {pendingConsents.length > 0 && (
        <PluginConsentDialog
          pluginName={pendingConsents[0].pluginName}
          requests={pendingConsents}
          onApprove={handleConsentApprove}
          onDeny={handleConsentDeny}
        />
      )}

      {pendingDialogs.length > 0 && (
        <PluginDialog
          pluginName={pendingDialogs[0].pluginName}
          request={pendingDialogs[0]}
          onSubmit={(value) => resolveDialog(pendingDialogs[0].pluginId, pendingDialogs[0].requestId, value)}
          onCancel={() => resolveDialog(pendingDialogs[0].pluginId, pendingDialogs[0].requestId, undefined)}
        />
      )}

      {pendingRun && (
        <PluginRunParamsDialog
          pluginName={pendingRun.pluginName}
          params={pendingRun.params}
          onSubmit={handleRunWithParams}
          onCancel={() => setPendingRun(null)}
        />
      )}
    </AppShell>
  );
}

function PluginLogPanel() {
  const { t } = useTranslation(['plugin', 'common']);
  const [logs, setLogs] = useState<{ level: string; message: string; timestamp: string }[]>([]);

  useEffect(() => {
    pluginCommands.auditLog(50).then((entries) => {
      const lines = entries.map((e) => ({
        level: 'info',
        message: `${e.action.action} — ${e.pluginId}`,
        timestamp: e.timestamp,
      }));
      setLogs(lines);
    });
  }, []);

  return (
    <Card className={styles.logPanel}>
      <h4>{t('plugin:audit_log', { defaultValue: 'Audit Log' })}</h4>
      {logs.length === 0 ? (
        <div className={styles.empty}>{t('plugin:no_logs', { defaultValue: 'No audit logs yet' })}</div>
      ) : (
        <div className={styles.auditList}>
          {logs.map((log, i) => (
            <div key={i} className={styles.auditRow}>
              <span className={styles.auditTime}>{log.timestamp}</span>
              <span className={styles.auditMessage}>{log.message}</span>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}
