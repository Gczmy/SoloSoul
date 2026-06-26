import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Download, RefreshCw, Trash2, Play, Loader2, X } from 'lucide-react';
import { PluginLogSection } from './shared/PluginLogSection';
import { PluginResultSection } from './shared/PluginResultSection';
import { useConfirm } from '@/hooks/useConfirm';
import styles from './PluginCard.module.css';
import type { MarketPluginInfo, PluginManifest } from '@/lib/plugin';
import type { RunningPlugin } from '@/stores/pluginStore';
import { ICON_SIZE } from '@/lib/iconSizes';


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
  const { requestConfirm, dialog } = useConfirm();

  const locale = i18n.language?.startsWith('zh') ? 'zh' : 'en';
  const installed = !!info.installedVersion;
  const latestVersion = info.registryEntry.latestVersion;

  const statuses = useMemo(() => {
    const result: Array<{ label: string; className: string }> = [];
    if (!info.isCompatible) {
      result.push({
        label: t('status_incompatible', { defaultValue: 'Incompatible' }),
        className: styles.statusIncompatible,
      });
      return result;
    }
    if (installed) {
      result.push({
        label: t('status_installed', { defaultValue: 'Installed' }),
        className: styles.statusInstalled,
      });
    }
    if (isRunning) {
      result.push({
        label: t('status_running', { defaultValue: 'Running' }),
        className: styles.statusRunning,
      });
    }
    if (installed && info.hasUpdate) {
      result.push({
        label: t('status_update', {
          defaultValue: `Update: ${info.installedVersion} → ${latestVersion}`,
        }),
        className: styles.statusUpdate,
      });
    }
    if (!installed) {
      result.push({
        label: t('status_not_installed', { defaultValue: 'Not Installed' }),
        className: styles.statusNotInstalled,
      });
    }
    return result;
  }, [info, isRunning]);

  const displayName = info.registryEntry.i18n?.[locale]?.name ?? info.registryEntry.name;
  const displayDesc =
    info.registryEntry.i18n?.[locale]?.description ?? info.registryEntry.description;

  return (
    <div className={styles.card}>
      <div className={styles.header}>
        <div className={styles.nameRow}>
          <h3 className={styles.name}>{displayName}</h3>
          <div className={styles.badgeGroup}>
            {statuses.map((s) => (
              <span key={s.className} className={`${styles.status} ${s.className}`}>{s.label}</span>
            ))}
          </div>
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
              {isRunning ? <Loader2 size={ICON_SIZE.sm} className={styles.spin} /> : <Play size={ICON_SIZE.sm} />}
              {isRunning
                ? t('status_running', { defaultValue: 'Running' })
                : t('run', { defaultValue: 'Run' })}
            </button>
          )}
          {installed && showResults && runningPlugin?.completed && (
            <button className={styles.clearActionBtn} onClick={onClear}>
              <X size={ICON_SIZE.sm} />
              {t('clear', { defaultValue: 'Clear' })}
            </button>
          )}
          {!installed && info.isCompatible && (
            <button className={styles.installBtn} onClick={onInstall}>
              <Download size={ICON_SIZE.sm} />
              {t('install', { defaultValue: 'Install' })}
            </button>
          )}
          {installed && info.hasUpdate && info.isCompatible && (
            <button className={styles.updateBtn} onClick={onUpdate}>
              <RefreshCw size={ICON_SIZE.sm} />
              {t('update', { defaultValue: 'Update' })}
            </button>
          )}
        </div>
        {installed && (
          <div className={styles.actionsRight}>
            <button className={styles.uninstallBtn} onClick={() => requestConfirm(
                t('uninstall_confirm_title', { defaultValue: 'Uninstall Plugin' }),
                t('uninstall_confirm_message', { defaultValue: 'Are you sure you want to uninstall "{{name}}"? This action will remove the plugin and its local data.', name: displayName }),
                onUninstall,
              )}>
              <Trash2 size={ICON_SIZE.sm} />
              {t('uninstall', { defaultValue: 'Uninstall' })}
            </button>
          </div>
        )}
      </div>

      {showResults && runningPlugin && (
        <div className={styles.inlineResults}>
          <PluginLogSection
            logs={runningPlugin.logs}
            error={runningPlugin.error}
            completed={runningPlugin.completed}
            onStop={onStop}
            onClear={onClear}
            variant="page"
          />
          {runningPlugin.results.length > 0 && (
            <PluginResultSection
              results={runningPlugin.results}
              defaultExpanded
              showCopyButtons
              variant="page"
            />
          )}
        </div>
      )}

      {!info.isCompatible && (
        <p className={styles.hint}>
          {t('incompatible_hint', { defaultValue: 'Update SoloSoul to use this plugin' })}
        </p>
      )}
      {dialog}
    </div>
  );
}
