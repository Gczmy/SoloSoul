import { useTranslation } from 'react-i18next';
import { Copy, X, Loader2 } from 'lucide-react';
import { ExpandableSection } from './ExpandableSection';
import { CopyButton } from './CopyButton';
import type { RunningPlugin } from '@/stores/pluginStore';
import styles from './PluginLogSection.module.css';

interface PluginLogSectionProps {
  logs: RunningPlugin['logs'];
  error?: string;
  completed: boolean;
  onStop?: () => void;
  onClear?: () => void;
  /** 'page' = PluginCard (full page), 'sidebar' = QuickRunningInfo (side panel) */
  variant: 'page' | 'sidebar';
}

export function PluginLogSection({
  logs,
  error,
  completed,
  onStop,
  onClear,
  variant,
}: PluginLogSectionProps) {
  const { t } = useTranslation('plugin');
  const size = variant === 'page' ? 'md' : 'sm';

  const statusLabel = completed
    ? error
      ? t('status_failed', { defaultValue: 'Failed' })
      : t('status_completed', { defaultValue: 'Completed' })
    : t('status_running', { defaultValue: 'Running' });

  const statusClass = completed
    ? error
      ? styles.statusFailed
      : styles.statusCompleted
    : styles.statusRunning;

  // ─── Log copy handler ────────────────────────────────────────────
  const getLogContent = () =>
    logs.map((log) => `[${log.level}] ${log.message}`).join('\n');

  // ─── inlineActions content (page variant) ─────────────────────────
  const pageActions = completed ? (
    <>
      <CopyButton
        getContent={getLogContent}
        label={t('copy', { defaultValue: 'Copy' })}
        icon={<Copy size={size === 'md' ? 12 : 10} />}
        size={size}
      />
      <span className={`${styles.completedStatus} ${error ? styles.failed : styles.success}`}>
        {statusLabel}
      </span>
    </>
  ) : (
    <button className={styles.stopBtn} onClick={onStop}>
      {t('stop', { defaultValue: 'Stop' })}
    </button>
  );

  return (
    <>
      {/* ── Sidebar variant: separate status + error row before logs ── */}
      {variant === 'sidebar' && (
        <>
          <div className={styles.statusRow}>
            <span className={`${styles.statusBadge} ${statusClass}`}>
              {!completed && <Loader2 size={10} className={styles.spin} />}
              {statusLabel}
            </span>
            {completed ? (
              <button className={styles.clearBtn} onClick={onClear}>
                <X size={10} />
                {t('clear', { defaultValue: 'Clear' })}
              </button>
            ) : (
              <button className={styles.stopBtn} onClick={onStop}>
                {t('stop', { defaultValue: 'Stop' })}
              </button>
            )}
          </div>
          {error && <div className={styles.errorText}>{error}</div>}
        </>
      )}

      {/* ── Log expandable section ──────────────────────────────────── */}
      {logs.length > 0 && (
        <ExpandableSection
          title={t('inline_output', { defaultValue: 'Plugin Log' })}
          count={logs.length}
          actions={
            variant === 'page' ? (
              pageActions
            ) : (
              completed && <CopyButton
                getContent={getLogContent}
                label={t('copy', { defaultValue: 'Copy' })}
                icon={<Copy size={10} />}
                size="sm"
              />
            )
          }
        >
          <div className={styles.inlineLogs}>
            {logs.map((log) => (
              <div key={log.id} className={styles.logLine}>
                <span className={styles.logLevel} data-level={log.level}>
                  {t(`log_level_${log.level}`, { defaultValue: log.level })}
                </span>
                <span className={styles.logMessage}>{log.message}</span>
              </div>
            ))}
          </div>

          {/* Page variant: error inside collapsible */}
          {variant === 'page' && error && (
            <div className={styles.inlineError}>{error}</div>
          )}
        </ExpandableSection>
      )}
    </>
  );
}
