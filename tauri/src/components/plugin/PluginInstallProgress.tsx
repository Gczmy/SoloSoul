import { Check, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { PluginInstallProgress as InstallProgress } from '@/lib/plugin';
import styles from './PluginInstallProgress.module.css';

/** 圆环只反映后端阶段/字节进度，中心按钮保留取消；完成后短暂显示满环。 */
export function PluginInstallProgress({
  progress,
  onCancel,
  compact = false,
}: {
  progress: InstallProgress;
  onCancel: () => void;
  compact?: boolean;
}) {
  const { t } = useTranslation('plugin');
  const percent = Math.min(100, Math.max(0, Math.floor(progress.percent)));
  const complete = progress.phase === 'completed';
  const description = `${t(`install_progress.${progress.phase}`)} · ${percent}%`;
  return (
    <div className={styles.progress} data-compact={compact || undefined}>
      <svg
        className={styles.ring}
        viewBox="0 0 36 36"
        width={compact ? 28 : 34}
        height={compact ? 28 : 34}
        role="progressbar"
        aria-label={t('install_progress.label')}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        aria-valuetext={description}
      >
        <circle className={styles.track} cx="18" cy="18" r="15" />
        <circle
          className={styles.fill}
          cx="18"
          cy="18"
          r="15"
          pathLength="100"
          strokeDasharray="100"
          strokeDashoffset={100 - percent}
        />
      </svg>
      <button
        type="button"
        className={styles.cancel}
        aria-label={complete ? t('install_progress.completed') : t('cancel_install')}
        title={complete ? description : `${description}\n${t('cancel_install')}`}
        disabled={complete}
        onClick={onCancel}
      >
        {complete ? (
          <Check size={compact ? 14 : 16} aria-hidden="true" />
        ) : (
          <X size={compact ? 14 : 16} aria-hidden="true" />
        )}
      </button>
    </div>
  );
}
