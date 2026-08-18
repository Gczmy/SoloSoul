import { useEffect, useMemo, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { RefreshCw, Info, LayoutGrid, Download, Settings } from 'lucide-react';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { PluginCard } from '@/components/plugin/PluginCard';
import { PluginLogSection } from '@/components/plugin/shared/PluginLogSection';
import { PluginResultSection } from '@/components/plugin/shared/PluginResultSection';
import { WatermarkPluginConfig } from '@/components/plugin/WatermarkPluginConfig';
import { PluginConsentDialog } from '@/components/plugin/PluginConsentDialog';
import { PluginDialog } from '@/components/plugin/PluginDialog';
import { PluginRunParamsDialog } from '@/components/plugin/PluginRunParamsDialog';
import { usePluginStore } from '@/stores/pluginStore';
import {
  hasUsableWatermarkSelection,
  pluginCommands,
  PluginParam,
  PluginTier,
  WATERMARK_PLUGIN_ID,
} from '@/lib/plugin';
import { isDevOrDebug } from '@/lib/utils';
import { logger } from '@/lib/logger';
import { useUiStore } from '@/stores/uiStore';
import { useToastError } from '@/hooks/useToastError';
import styles from './PluginDashboardPage.module.css';
import { ICON_SIZE } from '@/lib/constants';
import { PageGuideButton } from '@/components/guide/PageGuideButton';

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

  // P215: useShallow 字段级选择——本页虽消费大部分字段，但避免 isLoadingMarket/
  // isLoadingInstalled/error 等单一字段翻转时整页重渲染（浅比较仅字段值变化触发）。
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
    refreshRegistry,
  } = usePluginStore(
    useShallow((s) => ({
      marketPlugins: s.marketPlugins,
      installedPlugins: s.installedPlugins,
      runningPlugins: s.runningPlugins,
      selectedTier: s.selectedTier,
      enabledTiers: s.enabledTiers,
      isLoadingMarket: s.isLoadingMarket,
      isLoadingInstalled: s.isLoadingInstalled,
      error: s.error,
      loadMarket: s.loadMarket,
      loadInstalled: s.loadInstalled,
      setSelectedTier: s.setSelectedTier,
      installPlugin: s.installPlugin,
      updatePlugin: s.updatePlugin,
      uninstallPlugin: s.uninstallPlugin,
      runPlugin: s.runPlugin,
      stopPlugin: s.stopPlugin,
      clearPluginOutput: s.clearPluginOutput,
      resolveDialog: s.resolveDialog,
      clearError: s.clearError,
      refreshRegistry: s.refreshRegistry,
    })),
  );

  const { onError } = useToastError();

  // 存储内联配置（水印插件等）的运行参数
  const inlineParamsRef = useRef<Record<string, Record<string, string>>>({});

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
    // 发布版本仅显示地址格式化器，开发/调试版本始终显示全部
    let filtered = marketPlugins;
    if (!isDevOrDebug()) {
      filtered = filtered.filter(
        (p) =>
          p.pluginId === 'com.solosoul.official.address-fmt' ||
          p.pluginId === WATERMARK_PLUGIN_ID ||
          p.pluginId === 'com.solosoul.official.expiry-guardian',
      );
    }

    switch (activeTab) {
      case 'installed':
        return filtered.filter((p) => p.installedVersion);
      case 'running':
        return filtered.filter(
          (p) => runningPlugins[p.pluginId] && !runningPlugins[p.pluginId].completed,
        );
      case 'all':
      default: {
        let list = filtered;
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
    // 在「全部」tab 点击运行时自动切换到「已安装」tab，避免出现旧样式卡片
    if (activeTab === 'all') {
      setActiveTab('installed');
    }
    const locale = i18n.language?.startsWith('zh') ? 'zh' : 'en';
    const name = info.registryEntry.i18n?.[locale]?.name ?? info.registryEntry.name;

    // 水印插件：优先使用侧边栏式内联配置参数
    if (pluginId === WATERMARK_PLUGIN_ID) {
      const savedParams = inlineParamsRef.current[pluginId];
      if (savedParams) {
        if (!hasUsableWatermarkSelection(savedParams)) {
          useUiStore.getState().showToast({
            type: 'warning',
            message: t('plugin:watermark.select_attachments_first', {
              defaultValue: '请先选择附件再运行',
            }),
            duration: 4000,
          });
          return;
        }
        await runPlugin(pluginId, name, savedParams);
        return;
      }
    }

    const params = info.registryEntry.params?.length
      ? info.registryEntry.params
      : (installedMap[pluginId]?.params ?? []);
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

  const pluginGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_plugin_title', { defaultValue: 'Plugin Guide' }),
        steps: [
          {
            icon: LayoutGrid,
            title: t('common:guide_plugin_step1_title', { defaultValue: 'Browse & Filter' }),
            description:
              t('common:guide_plugin_step1_desc', { defaultValue: 'Use the tabs to view all, installed, running, or log entries. Tier filters help you find plugins by phase.' }),
          },
          {
            icon: Download,
            title: t('common:guide_plugin_step2_title', { defaultValue: 'Install & Run' }),
            description:
              t('common:guide_plugin_step2_desc', { defaultValue: 'Install a plugin from the market, then run it. Some plugins require parameters or consent before execution.' }),
          },
          {
            icon: Settings,
            title: t('common:guide_plugin_step3_title', { defaultValue: 'Manage & Refresh' }),
            description:
              t('common:guide_plugin_step3_desc', { defaultValue: 'Update, uninstall, or stop plugins as needed. Refresh the registry to see the latest available plugins.' }),
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_plugins', { defaultValue: 'Plugins' }),
            description:
              t('common:guide_help_plugins_desc', { defaultValue: 'Discover, install, and run plugins in SoloSoul' }),
            href: '/help?id=plugins',
          },
        ],
      },
    ],
    [t],
  );

  return (
    <PageShell
      title={t('settings:items.plugins', { defaultValue: 'Plugins' })}
      onBack={() => navigate('/settings')}
      actions={<PageGuideButton pages={pluginGuidePages} />}
    >
      <PageContainer variant="wide" gap="section">
        <div className={styles.header}>
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
          <button
            className={styles.refreshBtn}
            onClick={() => refreshRegistry()}
            disabled={isLoadingMarket}
            aria-busy={isLoadingMarket}
            title={t('plugin:refresh', { defaultValue: 'Refresh registry' })}
            aria-label={t('plugin:refresh', { defaultValue: 'Refresh registry' })}
          >
            <RefreshCw
              size={ICON_SIZE.md}
              className={`${styles.refreshIcon} ${isLoadingMarket ? styles.spinning : ''}`}
            />
            <span className={styles.refreshLabel}>
              {t('plugin:refresh', { defaultValue: 'Refresh' })}
            </span>
          </button>
        </div>

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
              displayedPlugins.map((info) => {
                const installed = !!info.installedVersion;
                const isWatermark = info.pluginId === WATERMARK_PLUGIN_ID;
                const running = runningPlugins[info.pluginId];
                const showWatermarkRunning = activeTab === 'installed' || activeTab === 'running';
                return (
                  <div key={info.pluginId} className={styles.pluginWrapper}>
                    <PluginCard
                      info={info}
                      manifest={installedMap[info.pluginId]}
                      isRunning={!!running && !running.completed}
                      runningPlugin={running}
                      // 水印插件：由外部接管日志/结果渲染，卡片内不显示
                      showResults={
                        isWatermark ? false : activeTab === 'installed' || activeTab === 'running'
                      }
                      onInstall={() =>
                        installPlugin(info.pluginId, info.registryEntry.latestVersion)
                      }
                      onUpdate={() => updatePlugin(info.pluginId)}
                      onUninstall={() => uninstallPlugin(info.pluginId)}
                      onRun={() => handleRun(info.pluginId)}
                      onStop={() => stopPlugin(info.pluginId)}
                      onClear={() => clearPluginOutput(info.pluginId)}
                    />
                    {/* 水印插件：配置区 → 日志区 → 结果区（与侧边栏顺序一致） */}
                    {installed && isWatermark && (
                      <>
                        <div
                          className={styles.inlineConfig}
                          style={showWatermarkRunning && running ? { borderRadius: 0 } : undefined}
                        >
                          <WatermarkPluginConfig
                            onParamsChange={(params) => {
                              inlineParamsRef.current[info.pluginId] = params;
                            }}
                          />
                        </div>
                        {showWatermarkRunning && running && (
                          <div className={styles.inlineWatermarkRunning}>
                            <PluginLogSection
                              logs={running.logs}
                              error={running.error}
                              completed={running.completed}
                              onStop={() => stopPlugin(info.pluginId)}
                              onClear={() => clearPluginOutput(info.pluginId)}
                              variant="page"
                            />
                            {running.results.length > 0 && (
                              <PluginResultSection
                                results={running.results}
                                defaultExpanded
                                showCopyButtons
                                variant="page"
                              />
                            )}
                          </div>
                        )}
                      </>
                    )}
                  </div>
                );
              })
            )}
          </div>
        )}
      </PageContainer>

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
          onSubmit={(value) =>
            resolveDialog(pendingDialogs[0].pluginId, pendingDialogs[0].requestId, value)
          }
          onCancel={() =>
            resolveDialog(pendingDialogs[0].pluginId, pendingDialogs[0].requestId, undefined)
          }
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
    </PageShell>
  );
}

/** 将 RFC3339 时间戳拆为「日期 + 时间」两行：去掉时区后缀，时间精确到秒。 */
function formatAuditTimestamp(iso: string): { date: string; time: string } {
  const [date, rest = ''] = iso.split('T');
  const time = rest.replace(/(Z|[+-]\d{2}:?\d{2})$/, '').replace(/\.\d+$/, '');
  return { date, time };
}

function PluginLogPanel() {
  const { t } = useTranslation(['plugin', 'common']);
  const [logs, setLogs] = useState<{ level: string; message: string; timestamp: string }[]>([]);
  const [loadFailed, setLoadFailed] = useState(false);

  useEffect(() => {
    pluginCommands
      .auditLog(50)
      .then((entries) => {
        const lines = entries.map((e) => ({
          level: 'info',
          message: `${e.action.action} — ${e.pluginId}`,
          timestamp: e.timestamp,
        }));
        setLogs(lines);
      })
      .catch((err) => {
        // P023: 加载失败不再静默——落日志并显示错误态提示
        logger.warn('[PluginDashboardPage] Load audit log failed:', err);
        setLoadFailed(true);
      });
  }, []);

  return (
    <Card className={styles.logPanel}>
      <h4>{t('plugin:audit_log', { defaultValue: 'Audit Log' })}</h4>
      {loadFailed ? (
        <div className={styles.empty}>
          {t('plugin:audit_log_load_failed', {
            defaultValue: 'Failed to load audit logs',
          })}
        </div>
      ) : logs.length === 0 ? (
        <div className={styles.empty}>
          {t('plugin:no_logs', { defaultValue: 'No audit logs yet' })}
        </div>
      ) : (
        <div className={styles.auditList}>
          {logs.map((log, i) => {
            const { date, time } = formatAuditTimestamp(log.timestamp);
            return (
              <div key={i} className={styles.auditRow}>
                <span className={styles.auditTime}>
                  {date}
                  {time && (
                    <>
                      <br />
                      {time}
                    </>
                  )}
                </span>
                <span className={styles.auditMessage}>{log.message}</span>
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}
