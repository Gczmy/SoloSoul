import { useTranslation } from 'react-i18next';
import { Download, RefreshCw, Trash2, Play, Loader2 } from 'lucide-react';
import styles from './PluginCard.module.css';
import type { MarketPluginInfo, PluginManifest } from '@/lib/plugin';

interface PluginCardProps {
  info: MarketPluginInfo;
  manifest?: PluginManifest;
  isRunning: boolean;
  onInstall: () => void;
  onUpdate: () => void;
  onUninstall: () => void;
  onRun: () => void;
}

export function PluginCard({
  info,
  manifest: _manifest,
  isRunning,
  onInstall,
  onUpdate,
  onUninstall,
  onRun,
}: PluginCardProps) {
  const { t, i18n } = useTranslation('plugin');
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
        {installed && (
          <button className={styles.uninstallBtn} onClick={onUninstall}>
            <Trash2 size={14} />
            {t('uninstall', { defaultValue: 'Uninstall' })}
          </button>
        )}
        {installed && info.isCompatible && (
          <button className={styles.runBtn} onClick={onRun} disabled={isRunning}>
            {isRunning ? <Loader2 size={14} className={styles.spin} /> : <Play size={14} />}
            {isRunning
              ? t('status_running', { defaultValue: 'Running' })
              : t('run', { defaultValue: 'Run' })}
          </button>
        )}
      </div>

      {!info.isCompatible && (
        <p className={styles.hint}>
          {t('incompatible_hint', { defaultValue: 'Update SoloSoul to use this plugin' })}
        </p>
      )}
    </div>
  );
}
