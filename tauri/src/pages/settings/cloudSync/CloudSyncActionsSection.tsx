/**
 * P007：云同步设置页——操作 section（立即同步 / 保存 / 测试 / 删除 + 测试结果横幅）。
 */
import { HardDrive, Trash2, CheckCircle, AlertCircle, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { TransferButton } from '@/components/transfer/TransferButton';
import styles from '../CloudSyncPage.module.css';

interface CloudSyncActionsSectionProps {
  hasSavedConfig: boolean;
  isLoading: boolean;
  isTesting: boolean;
  isSyncingNow: boolean;
  isFormValid: boolean;
  onSyncNow: () => void;
  onSave: () => void;
  onTestConnection: () => void;
  onDelete: () => void;
  testResult: { success: boolean; error?: string } | null;
}

export function CloudSyncActionsSection({
  hasSavedConfig,
  isLoading,
  isTesting,
  isSyncingNow,
  isFormValid,
  onSyncNow,
  onSave,
  onTestConnection,
  onDelete,
  testResult,
}: CloudSyncActionsSectionProps) {
  const { t } = useTranslation(['settings']);
  return (
    <Card>
      <h2 className={styles.sectionTitle}>
        <HardDrive size={20} style={{ marginRight: 8 }} />
        {t('settings:cloud_sync_actions')}
      </h2>

      <div className={styles.actionButtons}>
        <TransferButton
          variant="plain"
          onClick={onSyncNow}
          disabled={!hasSavedConfig || isTesting}
          busy={isSyncingNow}
        >
          {isSyncingNow ? t('settings:cloud_sync_syncing') : t('settings:cloud_sync_sync_now')}
        </TransferButton>

        <TransferButton
          variant="accent"
          onClick={onSave}
          disabled={isLoading || isTesting || !isFormValid}
          busy={isLoading}
        >
          {hasSavedConfig ? t('settings:cloud_sync_update') : t('settings:cloud_sync_save')}
        </TransferButton>

        <TransferButton
          variant="plain"
          onClick={onTestConnection}
          disabled={isLoading || isTesting || !isFormValid}
          busy={isTesting}
        >
          {isTesting ? (
            <>
              <Loader2 size={16} style={{ animation: 'spin 1s linear infinite' }} />
              {t('settings:cloud_sync_testing')}
            </>
          ) : (
            t('settings:cloud_sync_test')
          )}
        </TransferButton>

        {hasSavedConfig && (
          <TransferButton variant="warning" onClick={onDelete} disabled={isLoading || isTesting}>
            <Trash2 size={16} style={{ marginRight: 4 }} />
            {t('settings:cloud_sync_delete')}
          </TransferButton>
        )}
      </div>

      {testResult && (
        <div
          className={styles.testResult}
          style={{
            // P007 核验补修：改用主题令牌（--success-soft/--error-soft 未定义，
            // 旧 fallback 为浅色硬编码，深色模式下刺眼）
            backgroundColor: testResult.success
              ? 'var(--success-subtle)'
              : 'var(--danger-subtle)',
            color: testResult.success ? 'var(--success)' : 'var(--danger)',
            borderColor: testResult.success
              ? 'color-mix(in srgb, var(--success) 45%, transparent)'
              : 'color-mix(in srgb, var(--danger) 45%, transparent)',
          }}
        >
          {testResult.success ? (
            <>
              <CheckCircle size={16} style={{ marginRight: 6 }} />
              {t('settings:cloud_sync_test_success')}
            </>
          ) : (
            <>
              <AlertCircle size={16} style={{ marginRight: 6 }} />
              {t('settings:cloud_sync_test_failed')}: {testResult.error}
            </>
          )}
        </div>
      )}
    </Card>
  );
}
