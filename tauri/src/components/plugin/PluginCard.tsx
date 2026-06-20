import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Download, RefreshCw, Trash2, Play, Loader2, X, ChevronDown, Copy, Check, FileJson, FileText } from 'lucide-react';
import { PluginResultPanel } from './PluginResultPanel';
import styles from './PluginCard.module.css';
import type { MarketPluginInfo, PluginManifest } from '@/lib/plugin';
import type { RunningPlugin } from '@/stores/pluginStore';

interface PluginCardProps {
  info: MarketPluginInfo;
  manifest?: PluginManifest;
  isRunning: boolean;
  runningPlugin?: RunningPlugin;
  showResults?: boolean;
  onInstall: () => void;
  onUpdate: () => void;
  onUninstall: () => void;
  onRun: () => void;
  onStop?: () => void;
  onClear?: () => void;
}

export function PluginCard({
  info,
  manifest: _manifest,
  isRunning,
  runningPlugin,
  showResults,
  onInstall,
  onUpdate,
  onUninstall,
  onRun,
  onStop,
  onClear,
}: PluginCardProps) {
  const { t, i18n } = useTranslation('plugin');
  const [expanded, setExpanded] = useState(false);
  const [resultExpanded, setResultExpanded] = useState(true);
  const [logCopied, setLogCopied] = useState(false);
  const [jsonCopied, setJsonCopied] = useState(false);
  const [mdCopied, setMdCopied] = useState(false);
  const locale = i18n.language?.startsWith('zh') ? 'zh' : 'en';
  const installed = !!info.installedVersion;
  const latestVersion = info.registryEntry.latestVersion;

  const status = (() => {
    if (!info.isCompatible) {
      return {
        label: t('status_incompatible', { defaultValue: 'Incompatible' }),
        className: styles.statusIncompatible,
      };
    }
    if (isRunning) {
      return {
        label: t('status_running', { defaultValue: 'Running' }),
        className: styles.statusRunning,
      };
    }
    if (info.hasUpdate) {
      return {
        label: t('status_update', {
          defaultValue: `Update: ${info.installedVersion} → ${latestVersion}`,
        }),
        className: styles.statusUpdate,
      };
    }
    if (installed) {
      return {
        label: t('status_installed', { defaultValue: 'Installed' }),
        className: styles.statusInstalled,
      };
    }
    return {
      label: t('status_not_installed', { defaultValue: 'Not Installed' }),
      className: styles.statusNotInstalled,
    };
  })();

  const displayName = info.registryEntry.i18n?.[locale]?.name ?? info.registryEntry.name;
  const displayDesc =
    info.registryEntry.i18n?.[locale]?.description ?? info.registryEntry.description;

  return (
    <div className={styles.card}>
      <div className={styles.header}>
        <div className={styles.nameRow}>
          <h3 className={styles.name}>{displayName}</h3>
          <span className={`${styles.status} ${status.className}`}>{status.label}</span>
        </div>
        <p className={styles.description}>{displayDesc}</p>
        <div className={styles.meta}>
          <span className={styles.version}>
            {t('version_label', { defaultValue: 'v' })}
            {latestVersion}
          </span>
          <span className={styles.author}>{info.registryEntry.author}</span>
          <span className={styles.badge}>{info.tier.toUpperCase()}</span>
          <span className={styles.badge}>{info.category}</span>
        </div>
      </div>

      <div className={styles.actions}>
        <div className={styles.actionsLeft}>
          {installed && info.isCompatible && (
            <button className={styles.runBtn} onClick={onRun} disabled={isRunning}>
              {isRunning ? <Loader2 size={14} className={styles.spin} /> : <Play size={14} />}
              {isRunning
                ? t('status_running', { defaultValue: 'Running' })
                : t('run', { defaultValue: 'Run' })}
            </button>
          )}
          {installed && showResults && runningPlugin?.completed && (
            <button className={styles.clearActionBtn} onClick={onClear}>
              <X size={14} />
              {t('clear', { defaultValue: 'Clear' })}
            </button>
          )}
          {!installed && info.isCompatible && (
            <button className={styles.installBtn} onClick={onInstall}>
              <Download size={14} />
              {t('install', { defaultValue: 'Install' })}
            </button>
          )}
          {installed && info.hasUpdate && info.isCompatible && (
            <button className={styles.updateBtn} onClick={onUpdate}>
              <RefreshCw size={14} />
              {t('update', { defaultValue: 'Update' })}
            </button>
          )}
        </div>
        {installed && (
          <div className={styles.actionsRight}>
            <button className={styles.uninstallBtn} onClick={onUninstall}>
              <Trash2 size={14} />
              {t('uninstall', { defaultValue: 'Uninstall' })}
            </button>
          </div>
        )}
      </div>

      {showResults && runningPlugin && (
        <div className={styles.inlineResults}>
          <div
            className={styles.inlineHeader}
            onClick={() => setExpanded(!expanded)}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                setExpanded(!expanded);
              }
            }}
          >
            <div className={styles.inlineTitleRow}>
              <ChevronDown
                size={14}
                className={`${styles.chevron} ${expanded ? styles.chevronOpen : ''}`}
              />
              <span className={styles.inlineTitle}>
                {t('inline_output', { defaultValue: 'Plugin Log' })}
              </span>
              {runningPlugin.logs.length > 0 && (
                <span className={styles.logCount}>{runningPlugin.logs.length}</span>
              )}
            </div>
            <div className={styles.inlineActions} onClick={(e) => e.stopPropagation()}>
              {runningPlugin.completed ? (
                <>
                  {expanded && (
                    <button
                      className={`${styles.copyLogBtn} ${logCopied ? styles.copyGlow : ''}`}
                      onClick={() => {
                        const text = runningPlugin.logs
                          .map((log) => `[${log.level}] ${log.message}`)
                          .join('\n');
                        navigator.clipboard.writeText(text).then(() => {
                          setLogCopied(true);
                          setTimeout(() => setLogCopied(false), 1500);
                        }).catch(() => {});
                      }}
                    >
                      {logCopied ? <Check size={12} /> : <Copy size={12} />}
                      {logCopied ? t('copied', { defaultValue: 'Copied' }) : t('copy', { defaultValue: 'Copy' })}
                    </button>
                  )}
                  <span className={`${styles.completedStatus} ${runningPlugin.error ? styles.failed : styles.success}`}>
                    {runningPlugin.error
                      ? t('status_failed', { defaultValue: 'Failed' })
                      : t('status_completed', { defaultValue: 'Completed' })}
                  </span>
                </>
              ) : (
                <button className={styles.stopBtn} onClick={onStop}>
                  {t('stop', { defaultValue: 'Stop' })}
                </button>
              )}
            </div>
          </div>
          <div className={`${styles.collapsible} ${expanded ? styles.collapsibleOpen : ''}`}>
            <div className={styles.inlineLogs}>
              {runningPlugin.logs.slice(-10).map((log) => (
                <div key={log.id} className={styles.logLine}>
                  <span className={styles.logLevel} data-level={log.level}>
                    {log.level}
                  </span>
                  <span className={styles.logMessage}>{log.message}</span>
                </div>
              ))}
            </div>
            {runningPlugin.error && (
              <div className={styles.inlineError}>{runningPlugin.error}</div>
            )}
          </div>
          <div
            className={styles.inlineHeader}
            onClick={() => setResultExpanded(!resultExpanded)}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                setResultExpanded(!resultExpanded);
              }
            }}
          >
            <div className={styles.inlineTitleRow}>
              <ChevronDown
                size={14}
                className={`${styles.chevron} ${resultExpanded ? styles.chevronOpen : ''}`}
              />
              <span className={styles.inlineTitle}>
                {t('inline_result', { defaultValue: 'Plugin Result' })}
              </span>
            </div>
            <div className={styles.inlineActions} onClick={(e) => e.stopPropagation()}>
              {resultExpanded && (
                <>
                  <button
                    className={`${styles.copyLogBtn} ${jsonCopied ? styles.copyGlow : ''}`}
                    onClick={() => {
                      navigator.clipboard.writeText(
                        JSON.stringify(runningPlugin.results, null, 2)
                      ).then(() => {
                        setJsonCopied(true);
                        setTimeout(() => setJsonCopied(false), 1500);
                      }).catch(() => {});
                    }}
                  >
                    <FileJson size={12} />
                    {jsonCopied ? t('copied', { defaultValue: 'Copied' }) : 'JSON'}
                  </button>
                  <button
                    className={`${styles.copyLogBtn} ${mdCopied ? styles.copyGlow : ''}`}
                    onClick={() => {
                      const text = runningPlugin.results
                        .map((r) => {
                          if (r.type === 'key_value') {
                            const rows = r.pairs
                              .map((p) => `| ${p.key} | ${p.value} |`)
                              .join('\n');
                            const header = r.title
                              ? `### ${r.title}\n\n`
                              : '';
                            return `${header}| Key | Value |\n| --- | --- |\n${rows}`;
                          }
                          return JSON.stringify(r, null, 2);
                        })
                        .join('\n\n---\n\n');
                      navigator.clipboard.writeText(text).then(() => {
                        setMdCopied(true);
                        setTimeout(() => setMdCopied(false), 1500);
                      }).catch(() => {});
                    }}
                  >
                    <FileText size={12} />
                    {mdCopied ? t('copied', { defaultValue: 'Copied' }) : 'Markdown'}
                  </button>
                </>
              )}
            </div>
          </div>
          <div className={`${styles.collapsible} ${resultExpanded ? styles.collapsibleOpen : ''}`}>
            <PluginResultPanel results={runningPlugin.results} />
          </div>
        </div>
      )}

      {!info.isCompatible && (
        <p className={styles.hint}>
          {t('incompatible_hint', { defaultValue: 'Update SoloSoul to use this plugin' })}
        </p>
      )}
    </div>
  );
}
