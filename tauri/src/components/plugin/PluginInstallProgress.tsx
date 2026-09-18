import { Loader2, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './PluginInstallProgress.module.css';

/** 页面与侧栏共用安装反馈；取消只通知正在执行的任务，不重复发起安装。 */
export function PluginInstallProgress({
  onCancel,
  compact = false,
}: {
  onCancel: () => void;
  compact?: boolean;
}) {
  const { t } = useTranslation('plugin');
  return (
    <button
      type="button"
      className={styles.progress}
      data-compact={compact || undefined}
      aria-label={t('cancel_install')}
      title={t('cancel_install')}
      onClick={onCancel}
    >
      <Loader2 className={styles.spinner} size={compact ? 28 : 34} aria-hidden="true" />
      <X size={compact ? 14 : 16} aria-hidden="true" />
    </button>
  );
}
