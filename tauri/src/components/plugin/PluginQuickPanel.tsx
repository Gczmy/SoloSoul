import { useEffect, useMemo, useState, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';import { Puzzle,
  RefreshCw,
  X,
  Play,
  Download,
  Trash2,
  Loader2,
  ArrowUpRight,
  ChevronDown,
  Copy,
  Check,
} from 'lucide-react';
import { usePluginStore, type RunningPlugin } from '@/stores/pluginStore';
import { usePluginQuickStore, type QuickPanelTab } from '@/stores/pluginQuickStore';
import { useConfirm } from '@/hooks/useConfirm';
import { PluginResultPanel } from './PluginResultPanel';
import { isDevOrDebug } from '@/lib/env';
import styles from './PluginQuickPanel.module.css';

interface PluginQuickPanelProps {
  position: { top: number } | null;
  onClose: () => void;
  placement?: 'left' | 'right' | 'bottom' | 'top';
}

export function PluginQuickPanel({
  position,
  onClose,
  placement = 'left',
}: PluginQuickPanelProps) {
  const { t, i18n } = useTranslation(['plugin', 'common']);
  const navigate = useNavigate();
  const locale = i18n.language?.startsWith('zh') ? 'zh' : 'en';

  const { activeTab, setActiveTab } = usePluginQuickStore();
  const { requestConfirm, dialog: uninstallDialog } = useConfirm();

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
  } = usePluginStore();

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
        (p) => p.pluginId === 'com.solosoul.official.address-fmt',
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
    () =>
      Object.values(runningPlugins).filter((rp) => !rp.completed).length,
    [runningPlugins],
  );

  const handleGoFull = () => {
    onClose();
    navigate('/plugins');
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
          <Puzzle size={16} style={{ color: 'var(--accent-primary)' }} />
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
            <RefreshCw
              size={14}
              className={isLoadingMarket ? undefined : undefined}
            />
          </button>
          <button
            className={styles.headerBtn}
            onClick={handleGoFull}
            title={t('plugin:go_full', { defaultValue: 'Open full page' })}
          >
            <ArrowUpRight size={14} />
          </button>
          <button
            className={styles.headerBtn}
            onClick={onClose}
            title={t('common:close')}
          >
            <X size={14} />
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
            const displayName =
              info.registryEntry.i18n?.[locale]?.name ?? info.registryEntry.name;
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
                        <Loader2 size={10} />
                        {t('plugin:status_running', { defaultValue: 'Running' })}
                      </span>
                    )}
                    {installed && info.isCompatible && (
                      <button
                        className={styles.runBtn}
                        onClick={() => {
                          const name =
                            info.registryEntry.i18n?.[locale]?.name ??
                            info.registryEntry.name;
                          runPlugin(info.pluginId, name);
                        }}
                        disabled={isRunning}
                      >
                        {isRunning ? (
                          <Loader2 size={12} />
                        ) : (
                          <Play size={12} />
                        )}
                        {t('plugin:run', { defaultValue: 'Run' })}
                      </button>
                    )}
                    {!installed && info.isCompatible && (
                      <button
                        className={styles.installBtn}
                        onClick={() =>
                          installPlugin(
                            info.pluginId,
                            info.registryEntry.latestVersion,
                          )
                        }
                      >
                        <Download size={12} />
                        {t('plugin:install', { defaultValue: 'Install' })}
                      </button>
                    )}
                    {installed && (
                      <button
                        className={styles.uninstallBtn}
                        onClick={() => requestConfirm(
                          t('plugin:uninstall_confirm_title', { defaultValue: 'Uninstall Plugin' }),
                          t('plugin:uninstall_confirm_message', { defaultValue: 'Are you sure you want to uninstall "{{name}}"? This action will remove the plugin and its local data.', name: displayName }),
                          () => uninstallPlugin(info.pluginId),
                        )}
                      >
                        <Trash2 size={12} />
                      </button>
                    )}
                  </div>
                </div>

                {/* Running state inline */}
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
  const { t } = useTranslation('plugin');
  const [logExpanded, setLogExpanded] = useState(false);
  const [resultExpanded, setResultExpanded] = useState(false);
  const [logCopied, setLogCopied] = useState(false);

  const handleCopyLogs = async () => {
    try {
      const text = running.logs
        .map((log) => `[${log.level}] ${log.message}`)
        .join('\n');
      await navigator.clipboard.writeText(text);
      setLogCopied(true);
      setTimeout(() => setLogCopied(false), 1500);
    } catch {
      // 静默忽略
    }
  };

  const statusBadge = running.completed
    ? { className: running.error ? styles.statusError : styles.statusCompleted, label: running.error ? t('plugin:status_failed', { defaultValue: 'Failed' }) : t('plugin:status_completed', { defaultValue: 'Completed' }) }
    : { className: styles.statusRunning, label: t('plugin:status_running', { defaultValue: 'Running' }) };

  return (
    <div className={styles.runningInfo}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 8,
        }}
      >
        <span className={statusBadge.className}>
          {statusBadge.label}
        </span>
        {running.completed ? (
          <button className={styles.actionBtn} onClick={onClear} style={{ fontSize: 10, padding: '2px 6px' }}>
            <X size={10} />
            {t('plugin:clear', { defaultValue: 'Clear' })}
          </button>
        ) : (
          <button className={styles.actionBtn} onClick={onStop}>
            {t('plugin:stop', { defaultValue: 'Stop' })}
          </button>
        )}
      </div>

      {running.error && (
        <div className={styles.errorText}>{running.error}</div>
      )}

      {/* Logs section - collapsible */}
      {running.logs.length > 0 && (
        <>
          <button
            className={styles.inlineHeader}
            onClick={() => setLogExpanded(!logExpanded)}
          >
            <div className={styles.inlineTitleRow}>
              <ChevronDown
                size={12}
                className={`${styles.chevron} ${logExpanded ? styles.chevronOpen : ''}`}
              />
              <span className={styles.inlineTitle}>
                {t('plugin:inline_output', { defaultValue: 'Plugin Log' })}
              </span>
              <span className={styles.logCount}>{running.logs.length}</span>
            </div>
            <div className={styles.inlineActions} onClick={(e) => e.stopPropagation()}>
              {running.completed && logExpanded && (
                <button
                  className={`${styles.copyLogBtn} ${logCopied ? styles.copyGlow : ''}`}
                  onClick={handleCopyLogs}
                >
                  {logCopied ? <Check size={10} /> : <Copy size={10} />}
                  {logCopied ? t('copied', { defaultValue: 'Copied' }) : t('copy', { defaultValue: 'Copy' })}
                </button>
              )}
            </div>
          </button>
          <div className={`${styles.collapsible} ${logExpanded ? styles.collapsibleOpen : ''}`}>
            <div className={styles.inlineLogs}>
              {running.logs.slice(-10).map((log) => (
                <div key={log.id} className={styles.logLine}>
                  <span className={styles.logLevel} data-level={log.level}>
                    {t(`plugin:log_level_${log.level}`, {
                      defaultValue: log.level,
                    })}
                  </span>
                  <span className={styles.logMessage}>{log.message}</span>
                </div>
              ))}
            </div>
          </div>
        </>
      )}

      {/* Results section - collapsible */}
      {running.results.length > 0 && (
        <>
          <button
            className={styles.inlineHeader}
            onClick={() => setResultExpanded(!resultExpanded)}
          >
            <div className={styles.inlineTitleRow}>
              <ChevronDown
                size={12}
                className={`${styles.chevron} ${resultExpanded ? styles.chevronOpen : ''}`}
              />
              <span className={styles.inlineTitle}>
                {t('plugin:inline_result', { defaultValue: 'Plugin Result' })}
              </span>
            </div>
          </button>
          <div className={`${styles.collapsible} ${resultExpanded ? styles.collapsibleOpen : ''}`}>
            <PluginResultPanel results={running.results} />
          </div>
        </>
      )}
    </div>
  );
}
