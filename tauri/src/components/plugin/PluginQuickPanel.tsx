import { useEffect, useMemo, useRef } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { Puzzle, RefreshCw, X, Play, Download, Loader2, ArrowUpRight } from 'lucide-react';
import { usePluginStore, type RunningPlugin } from '@/stores/pluginStore';
import { hasUsableWatermarkSelection, WATERMARK_PLUGIN_ID } from '@/lib/plugin';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { usePluginQuickStore, type QuickPanelTab } from '@/stores/pluginQuickStore';
import { useConfirm } from '@/hooks/useConfirm';
import { PluginLogSection } from './shared/PluginLogSection';
import { PluginResultSection } from './shared/PluginResultSection';
import { WatermarkPluginConfig } from './WatermarkPluginConfig';
import { useUiStore } from '@/stores/uiStore';
import { isDevOrDebug } from '@/lib/utils';
import styles from './PluginQuickPanel.module.css';
import { ICON_SIZE } from '@/lib/constants';

interface PluginQuickPanelProps {
  position: { top: number } | null;
  onClose: () => void;
  placement?: 'left' | 'right' | 'bottom' | 'top';
}

export function PluginQuickPanel({ position, onClose, placement = 'left' }: PluginQuickPanelProps) {
  const { t, i18n } = useTranslation(['plugin', 'common']);
  const navigate = useNavigate();
  const locale = i18n.language?.startsWith('zh') ? 'zh' : 'en';

  const { activeTab, setActiveTab } = usePluginQuickStore();
  const { requestConfirm, dialog: uninstallDialog } = useConfirm();

  // 存储插件运行参数（用于水印插件等的侧边栏配置）
  const pluginRunParamsRef = useRef<Record<string, Record<string, string>>>({});

  // P215: useShallow 字段级选择——侧边栏消费大部分字段，但避免 isLoading* 单字段翻转整面板重渲染。
  const {
    marketPlugins,
    installedPlugins,
    runningPlugins,
    isLoadingMarket,
    isLoadingInstalled,
    loadMarket,
    loadInstalled,
    installPlugin,
    uninstallPlugin,
    runPlugin,
    stopPlugin,
    clearPluginOutput,
    refreshRegistry,
  } = usePluginStore(
    useShallow((s) => ({
      marketPlugins: s.marketPlugins,
      installedPlugins: s.installedPlugins,
      runningPlugins: s.runningPlugins,
      isLoadingMarket: s.isLoadingMarket,
      isLoadingInstalled: s.isLoadingInstalled,
      loadMarket: s.loadMarket,
      loadInstalled: s.loadInstalled,
      installPlugin: s.installPlugin,
      uninstallPlugin: s.uninstallPlugin,
      runPlugin: s.runPlugin,
      stopPlugin: s.stopPlugin,
      clearPluginOutput: s.clearPluginOutput,
      refreshRegistry: s.refreshRegistry,
    })),
  );

  const cardRef = useRef<HTMLDivElement>(null);
  const outsideClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    loadMarket();
    loadInstalled();
  }, [loadMarket, loadInstalled]);

  // Close on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (cardRef.current && !cardRef.current.contains(e.target as Node)) {
        if ((e.target as HTMLElement).closest('[data-plugin-button]')) return;
        onClose();
      }
    };
    outsideClickTimeoutRef.current = setTimeout(
      () => document.addEventListener('mousedown', handler),
      0,
    );
    return () => {
      if (outsideClickTimeoutRef.current) clearTimeout(outsideClickTimeoutRef.current);
      document.removeEventListener('mousedown', handler);
    };
  }, [onClose]);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [onClose]);

  const displayedPlugins = useMemo(() => {
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
      case 'running': {
        const runningIds = new Set(
          Object.entries(runningPlugins)
            .filter(([, rp]) => !rp.completed)
            .map(([id]) => id),
        );
        return filtered.filter((p) => runningIds.has(p.pluginId));
      }
      case 'all':
      default:
        return filtered;
    }
  }, [marketPlugins, runningPlugins, activeTab]);

  const activeRunningCount = useMemo(
    () => Object.values(runningPlugins).filter((rp) => !rp.completed).length,
    [runningPlugins],
  );

  const handleGoFull = () => {
    onClose();
    navigate('/plugins');
  };

  // P043/P044：运行逻辑移出 JSX 回调（降嵌套），水印附件前置校验走共享 helper
  const handleRunPlugin = (info: (typeof marketPlugins)[number]) => {
    const name = info.registryEntry.i18n?.[locale]?.name ?? info.registryEntry.name;
    const savedParams = pluginRunParamsRef.current[info.pluginId];
    if (
      info.pluginId === WATERMARK_PLUGIN_ID &&
      !hasUsableWatermarkSelection(savedParams)
    ) {
      useUiStore.getState().showToast({
        type: 'warning',
        message: t('plugin:watermark.select_attachments_first', {
          defaultValue: '请先选择附件再运行',
        }),
        duration: 4000,
      });
      return;
    }
    runPlugin(info.pluginId, name, savedParams);
  };

  const isFloating = placement === 'bottom' || placement === 'top';
  const isRight = placement === 'right';

  const cardStyle: React.CSSProperties = {
    position: 'fixed',
    ...(isFloating
      ? { right: 12, left: 'auto' }
      : isRight
        ? { right: 52, left: 'auto' }
        : { left: 52, right: 'auto' }),
    top: position?.top ?? 100,
  };

  return (
    <div ref={cardRef} className={styles.card} style={cardStyle}>
      {/* Header */}
      <div className={styles.header}>
        <div className={styles.headerLeft}>
          <Puzzle size={ICON_SIZE.md} style={{ color: 'var(--accent-primary)' }} />
          <span className={styles.headerTitle}>
            {t('common:plugins', { defaultValue: 'Plugins' })}
          </span>
        </div>
        <div className={styles.headerActions}>
          <button
            className={styles.headerBtn}
            onClick={() => refreshRegistry()}
            disabled={isLoadingMarket}
            title={t('plugin:refresh', { defaultValue: 'Refresh registry' })}
          >
            <RefreshCw size={ICON_SIZE.sm} className={isLoadingMarket ? undefined : undefined} />
          </button>
          <button
            className={styles.headerBtn}
            onClick={handleGoFull}
            title={t('plugin:go_full', { defaultValue: 'Open full page' })}
          >
            <ArrowUpRight size={ICON_SIZE.sm} />
          </button>
          <button className={styles.headerBtn} onClick={onClose} title={t('common:close')}>
            <X size={ICON_SIZE.sm} />
          </button>
        </div>
      </div>

      {/* Tabs */}
      <div className={styles.tabs}>
        {(['all', 'installed', 'running'] as QuickPanelTab[]).map((tab) => (
          <button
            key={tab}
            className={`${styles.tab} ${activeTab === tab ? styles.tabActive : ''}`}
            onClick={() => setActiveTab(tab)}
          >
            {t(`plugin:tab_${tab}`, { defaultValue: tab })}
            {tab === 'installed' && installedPlugins.length > 0 && (
              <span className={styles.tabBadge}>{installedPlugins.length}</span>
            )}
            {tab === 'running' && activeRunningCount > 0 && (
              <span className={styles.tabBadge}>{activeRunningCount}</span>
            )}
          </button>
        ))}
      </div>

      {/* Body */}
      <div className={styles.body}>
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
            const running = runningPlugins[info.pluginId];
            const isRunning = running && !running.completed;
            const displayName = info.registryEntry.i18n?.[locale]?.name ?? info.registryEntry.name;
            return (
              <div key={info.pluginId} className={styles.pluginCard}>
                <div className={styles.pluginRow}>
                  <div className={styles.pluginInfo}>
                    <span className={styles.pluginName}>{displayName}</span>
                    <span className={styles.pluginMeta}>
                      {info.registryEntry.author} · v{info.registryEntry.latestVersion}
                    </span>
                  </div>
                  <div className={styles.pluginActions}>
                    {installed && (
                      <span className={styles.statusInstalled}>
                        {t('plugin:status_installed', { defaultValue: 'Installed' })}
                      </span>
                    )}
                    {isRunning && (
                      <span className={styles.statusRunning}>
                        <Loader2 size={ICON_SIZE['2xs']} />
                        {t('plugin:status_running', { defaultValue: 'Running' })}
                      </span>
                    )}
                    {installed && info.isCompatible && (
                      <button
                        className={styles.runBtn}
                        onClick={() => handleRunPlugin(info)}
                        disabled={isRunning}
                      >
                        {isRunning ? <Loader2 size={ICON_SIZE.xs} /> : <Play size={ICON_SIZE.xs} />}
                        {t('plugin:run', { defaultValue: 'Run' })}
                      </button>
                    )}
                    {!installed && info.isCompatible && (
                      <button
                        className={styles.installBtn}
                        onClick={() =>
                          installPlugin(info.pluginId, info.registryEntry.latestVersion)
                        }
                      >
                        <Download size={ICON_SIZE.xs} />
                        {t('plugin:install', { defaultValue: 'Install' })}
                      </button>
                    )}
                    {installed && (
                      <DeleteButton
                        onClick={() =>
                          requestConfirm(
                            t('plugin:uninstall_confirm_title', {
                              defaultValue: 'Uninstall Plugin',
                            }),
                            t('plugin:uninstall_confirm_message', {
                              defaultValue:
                                'Are you sure you want to uninstall "{{name}}"? This action will remove the plugin and its local data.',
                              name: displayName,
                            }),
                            () => uninstallPlugin(info.pluginId),
                          )
                        }
                        title={t('plugin:uninstall', { defaultValue: 'Uninstall' })}
                        iconOnly
                      />
                    )}
                  </div>
                </div>

                {/* ── 水印插件配置区（侧边栏内联配置，始终在日志上方） ──── */}
                {installed && info.pluginId === WATERMARK_PLUGIN_ID && (
                  <WatermarkPluginConfig
                    onParamsChange={(params) => {
                      pluginRunParamsRef.current[info.pluginId] = params;
                    }}
                  />
                )}
                {/* Running state inline（运行日志 + 结果，在配置区下方） */}
                {running && (
                  <QuickRunningInfo
                    pluginId={info.pluginId}
                    running={running}
                    onStop={() => stopPlugin(info.pluginId)}
                    onClear={() => clearPluginOutput(info.pluginId)}
                  />
                )}
              </div>
            );
          })
        )}
      </div>

      {uninstallDialog}

      <style>{`
        @keyframes pluginQuickPulse {
          0%, 100% { opacity: 0.6; }
          50% { opacity: 1; }
        }
      `}</style>
    </div>
  );
}

function QuickRunningInfo({
  pluginId: _pluginId,
  running,
  onStop,
  onClear,
}: {
  pluginId: string;
  running: RunningPlugin;
  onStop: () => void;
  onClear: () => void;
}) {
  return (
    <div className={styles.runningInfo}>
      <PluginLogSection
        logs={running.logs}
        error={running.error}
        completed={running.completed}
        onStop={onStop}
        onClear={onClear}
        variant="sidebar"
      />
      {running.results.length > 0 && (
        <PluginResultSection results={running.results} defaultExpanded variant="sidebar" />
      )}
    </div>
  );
}
