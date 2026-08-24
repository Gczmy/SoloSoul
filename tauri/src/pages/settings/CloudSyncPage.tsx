/**
 * 云同步设置页（P007 拆分后主组件）。
 *
 * 状态与处理器集中在 `useCloudSyncPage`（同目录），展示按 section 拆分为
 * cloudSync/ 下的子组件：连接配置 / 同步计划 / 保留策略 / 操作 / 信息卡 /
 * 下行待导入 / 状态卡。本文件仅做编排与密码验证对话框。
 */
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { useCloudSyncPage } from './cloudSync/useCloudSyncPage';
import { CloudSyncConnectionSection } from './cloudSync/CloudSyncConnectionSection';
import { CloudSyncScheduleSection } from './cloudSync/CloudSyncScheduleSection';
import { CloudSyncRetentionSection } from './cloudSync/CloudSyncRetentionSection';
import { CloudSyncActionsSection } from './cloudSync/CloudSyncActionsSection';
import { CloudSyncInfoCard } from './cloudSync/CloudSyncInfoCard';
import { CloudSyncIncomingSection } from './cloudSync/CloudSyncIncomingSection';
import { CloudSyncStatusCard } from './cloudSync/CloudSyncStatusCard';
import styles from './CloudSyncPage.module.css';

export function CloudSyncPage() {
  const { t } = useTranslation(['settings']);
  const navigate = useNavigate();
  const s = useCloudSyncPage();

  return (
    <PageShell title={t('settings:cloud_sync_title')} onBack={() => navigate('/settings')}>
      <PageContainer variant="form" gap="default">
        <div>
          <h1 className={styles.title}>{t('settings:cloud_sync_title')}</h1>
          <p className={styles.subtitle}>{t('settings:cloud_sync_subtitle')}</p>

          <CloudSyncConnectionSection
            connectorType={s.connectorType}
            onConnectorTypeChange={s.setConnectorType}
            configJson={s.configJson}
            onConfigJson={s.setConfigJson}
          />

          <CloudSyncScheduleSection
            enabled={s.enabled}
            onEnabledChange={s.setEnabled}
            intervalSecs={s.intervalSecs}
            onIntervalSecs={s.setIntervalSecs}
            wifiOnly={s.wifiOnly}
            onWifiOnlyChange={s.setWifiOnly}
            autoImport={s.autoImport}
            onAutoImportChange={s.setAutoImport}
          />

          <CloudSyncRetentionSection
            retention={s.retention}
            onRetentionChange={s.setRetention}
          />

          <CloudSyncActionsSection
            hasSavedConfig={!!s.savedConfig}
            isLoading={s.isLoading}
            isTesting={s.isTesting}
            isSyncingNow={s.isSyncingNow}
            isFormValid={s.isFormValid}
            onSyncNow={s.handleSyncNow}
            onSave={s.handleSave}
            onTestConnection={s.handleTestConnection}
            onDelete={s.handleDelete}
            testResult={s.testResult}
          />

          <CloudSyncInfoCard />

          {s.incomingFiles.length > 0 && (
            <CloudSyncIncomingSection
              incomingFiles={s.incomingFiles}
              importingFile={s.importingFile}
              onImport={s.handleImportIncoming}
            />
          )}

          {s.savedConfig && <CloudSyncStatusCard savedConfig={s.savedConfig} />}

          <PasswordVerificationDialog
            open={s.showPasswordDialog}
            onClose={s.handlePasswordCancelled}
            onVerify={async (_password: string) => {
              s.passwordVerifiedRef.current = true;
              s.setShowPasswordDialog(false);
              await s.doSave();
              return true;
            }}
            title={t('settings:cloud_sync_password_dialog_title')}
            description={t('settings:cloud_sync_password_dialog_desc')}
          />
        </div>
      </PageContainer>
    </PageShell>
  );
}
