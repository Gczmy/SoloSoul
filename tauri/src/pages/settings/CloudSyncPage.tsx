import { useState, useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { TransferButton } from '@/components/transfer/TransferButton';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { useToastError } from '@/hooks/useToastError';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import {
  Shield,
  Trash2,
  CheckCircle,
  AlertCircle,
  Loader2,
  Clock,
  HardDrive,
  Info,
  DownloadCloud,
} from 'lucide-react';
import styles from './CloudSyncPage.module.css';

interface RetentionPolicy {
  recentFull: number;
  daily: boolean;
  weekly: boolean;
  monthly: boolean;
}

const CONNECTOR_OPTIONS = [
  { value: 'webdav', label: 'WebDAV (坚果云 / Nextcloud / Alist / 自建)' },
] as const;

const DEFAULT_RETENTION: RetentionPolicy = {
  recentFull: 10,
  daily: true,
  weekly: true,
  monthly: true,
};

const DEFAULT_WEBDAV_CONFIG: Record<string, unknown> = {
  baseUrl: 'https://dav.jianguoyun.com/dav/',
  username: '',
  password: '',
  rootPrefix: '/SoloSoul/',
};

export function CloudSyncPage() {
  const { t, i18n } = useTranslation(['settings', 'common']);
  const { onError, onSuccess } = useToastError();
  const accountId = useAuthStore((s) => s.currentAccount?.id ?? '');

  // Form state
  const [connectorType, setConnectorType] = useState('webdav');
  const [configJson, setConfigJson] = useState<Record<string, unknown>>(DEFAULT_WEBDAV_CONFIG);
  const [enabled, setEnabled] = useState(false);
  const [intervalSecs, setIntervalSecs] = useState(3600);
  const [wifiOnly, setWifiOnly] = useState(true);
  const [autoImport, setAutoImport] = useState(false);
  const [retention, setRetention] = useState<RetentionPolicy>(DEFAULT_RETENTION);

  // UI state
  const [isLoading, setIsLoading] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; error?: string } | null>(null);
  const [showPasswordDialog, setShowPasswordDialog] = useState(false);
  const [savedConfig, setSavedConfig] = useState<{
    connectorType: string;
    configJson: Record<string, unknown>;
    enabled: boolean;
    intervalSecs: number;
    wifiOnly: boolean;
    retention: RetentionPolicy;
    lastSyncAt?: string;
  } | null>(null);
  const [incomingFiles, setIncomingFiles] = useState<string[]>([]);
  const [isSyncingNow, setIsSyncingNow] = useState(false);
  const [importingFile, setImportingFile] = useState<string | null>(null);
  const passwordVerifiedRef = useRef(false);

  // 加载云端待导入快照列表 + 监听下行事件
  useEffect(() => {
    if (!accountId) return;
    invoke<string[]>('cloud_sync_list_incoming')
      .then((files) => setIncomingFiles(files ?? []))
      .catch(() => setIncomingFiles([]));
    const unlisten = listen<{ files: string[] }>('cloud-sync-incoming', (event) => {
      setIncomingFiles(event.payload.files ?? []);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [accountId]);

  const loadConfig = useCallback(async () => {
    try {
      setIsLoading(true);
      const config = await invoke<{
        connectorType: string;
        configJson: Record<string, unknown>;
        enabled: boolean;
        intervalSecs: number;
        wifiOnly: boolean;
        autoImport?: boolean;
        retention: RetentionPolicy;
        lastSyncAt?: string;
      } | null>('cloud_sync_get_config', { accountId });
      if (config) {
        setSavedConfig(config);
        setConnectorType(config.connectorType);
        setConfigJson(config.configJson);
        setEnabled(config.enabled);
        setIntervalSecs(config.intervalSecs);
        setWifiOnly(config.wifiOnly);
        setAutoImport(config.autoImport ?? false);
        setRetention(config.retention);
      }
    } catch (e) {
      onError(new Error(String(e)), t('settings:cloud_sync_load_failed'));
    } finally {
      setIsLoading(false);
    }
  }, [accountId, onError, t]);

  // Load existing config on mount
  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const handleSave = async () => {
    if (!passwordVerifiedRef.current) {
      setShowPasswordDialog(true);
      return;
    }
    await doSave();
  };

  const doSave = async () => {
    try {
      await invoke('cloud_sync_save_config', {
        accountId,
        connectorType,
        configJson,
        enabled,
        intervalSecs,
        wifiOnly,
        autoImport,
        retention,
      });
      onSuccess(t('settings:cloud_sync_saved'));
      loadConfig();
    } catch (e) {
      onError(new Error(String(e)), t('settings:cloud_sync_save_failed'));
    }
  };

  const handleDelete = async () => {
    if (!window.confirm(t('settings:cloud_sync_delete_confirm'))) return;
    try {
      await invoke('cloud_sync_delete_config', { accountId });
      onSuccess(t('settings:cloud_sync_deleted'));
      setSavedConfig(null);
      setConnectorType('webdav');
      setConfigJson(DEFAULT_WEBDAV_CONFIG);
      setEnabled(false);
      setIntervalSecs(3600);
      setWifiOnly(true);
      setRetention(DEFAULT_RETENTION);
    } catch (e) {
      onError(new Error(String(e)), t('settings:cloud_sync_delete_failed'));
    }
  };

  const handleTestConnection = async () => {
    setIsTesting(true);
    setTestResult(null);
    try {
      await invoke('cloud_sync_test_connection', {
        accountId,
        connectorType,
        configJson,
        enabled,
        intervalSecs,
        wifiOnly,
        autoImport,
        retention,
      });
      setTestResult({ success: true });
      onSuccess(t('settings:cloud_sync_test_success'));
    } catch (e) {
      setTestResult({ success: false, error: String(e) });
      onError(new Error(String(e)), t('settings:cloud_sync_test_failed'));
    } finally {
      setIsTesting(false);
    }
  };

  const handleSyncNow = async () => {
    setIsSyncingNow(true);
    try {
      await invoke('cloud_sync_now');
      // 调度器异步执行；稍后刷新待导入列表
      setTimeout(() => {
        invoke<string[]>('cloud_sync_list_incoming')
          .then((files) => setIncomingFiles(files ?? []))
          .catch(() => {});
      }, 3000);
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('settings:cloud_sync_sync_now_failed'));
    } finally {
      setIsSyncingNow(false);
    }
  };

  const handleImportIncoming = async (file: string) => {
    // 文件名 {hlc}.solosoul，父目录名即来源 device_id
    const parts = file.split('/');
    const hlc = (parts.pop() ?? '').replace(/\.solosoul$/, '');
    const deviceId = parts.pop() ?? '';
    if (!hlc || !deviceId) return;
    const snapshotPw = (configJson.password as string) || '';
    if (!snapshotPw) {
      onError(new Error('missing password'), t('settings:cloud_sync_import_failed'));
      return;
    }
    setImportingFile(file);
    try {
      await invoke('import_execute_advanced', {
        accountId,
        req: {
          selections: null,
          strategy: 'skipExisting',
          sourcePath: file,
          password: snapshotPw,
          selectedAttachmentIds: null,
          objectStrategies: null,
          locale: i18n.language || 'zh-CN',
        },
      });
      await invoke('cloud_sync_mark_applied', { deviceId, hlc });
      onSuccess(t('settings:cloud_sync_import_success'));
      setIncomingFiles((prev) => prev.filter((f) => f !== file));
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('settings:cloud_sync_import_failed'));
    } finally {
      setImportingFile(null);
    }
  };

  const handlePasswordCancelled = () => {
    setShowPasswordDialog(false);
  };

  // Validation
  const isFormValid = Boolean(
    configJson.baseUrl && configJson.username && configJson.password
  );

  return (
    <PageShell title={t('settings:cloud_sync_title')}>
      <PageContainer>
        <div className={styles.container}>
          <h1 className={styles.title}>{t('settings:cloud_sync_title')}</h1>
          <p className={styles.subtitle}>{t('settings:cloud_sync_subtitle')}</p>

          <Card className={styles.card}>
            <h2 className={styles.sectionTitle}>
              <Shield size={20} style={{ marginRight: 8 }} />
              {t('settings:cloud_sync_connection')}
            </h2>

            <div className={styles.fieldGroup}>
              <label className={styles.label}>{t('settings:cloud_sync_connector')}</label>
              <select
                value={connectorType}
                onChange={(e) => setConnectorType(e.target.value)}
                className={styles.select}
              >
                {CONNECTOR_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
              <p className={styles.hint}>{t('settings:cloud_sync_connector_hint')}</p>
            </div>

            <div className={styles.fieldGroup}>
              <label className={styles.label}>{t('settings:cloud_sync_server_url')}</label>
              <input
                type="url"
                value={(configJson.baseUrl as string) || ''}
                onChange={(e) => setConfigJson({ ...(configJson as Record<string, unknown>), baseUrl: e.target.value })}
                placeholder="https://dav.jianguoyun.com/dav/"
                className={styles.input}
              />
            </div>

            <div className={styles.fieldGroup}>
              <label className={styles.label}>{t('settings:cloud_sync_username')}</label>
              <input
                type="text"
                value={(configJson.username as string) || ''}
                onChange={(e) => setConfigJson({ ...(configJson as Record<string, unknown>), username: e.target.value })}
                placeholder="user@example.com"
                className={styles.input}
              />
            </div>

            <div className={styles.fieldGroup}>
              <label className={styles.label}>{t('settings:cloud_sync_password')}</label>
              <input
                type="password"
                value={(configJson.password as string) || ''}
                onChange={(e) => setConfigJson({ ...(configJson as Record<string, unknown>), password: e.target.value })}
                placeholder="••••••••"
                className={styles.input}
                autoComplete="current-password"
              />
              <p className={styles.hint}>{t('settings:cloud_sync_password_hint')}</p>
            </div>

            <div className={styles.fieldGroup}>
              <label className={styles.label}>{t('settings:cloud_sync_root_prefix')}</label>
              <input
                type="text"
                value={(configJson.rootPrefix as string) || ''}
                onChange={(e) => setConfigJson({ ...(configJson as Record<string, unknown>), rootPrefix: e.target.value })}
                placeholder="/SoloSoul/"
                className={styles.input}
              />
            </div>
          </Card>

          <Card className={styles.card}>
            <h2 className={styles.sectionTitle}>
              <HardDrive size={20} style={{ marginRight: 8 }} />
              {t('settings:cloud_sync_schedule')}
            </h2>

            <div className={styles.fieldGroup}>
              <label className={styles.label}>
                <input
                  type="checkbox"
                  checked={enabled}
                  onChange={(e) => setEnabled(e.target.checked)}
                  className={styles.checkbox}
                />
                {t('settings:cloud_sync_auto_sync')}
              </label>
            </div>

            {enabled && (
              <>
                <div className={styles.fieldGroup}>
                  <label className={styles.label}>
                    {t('settings:cloud_sync_interval')}
                    <input
                      type="number"
                      min={60}
                      max={86400}
                      value={intervalSecs}
                      onChange={(e) => setIntervalSecs(Math.max(60, parseInt(e.target.value) || 60))}
                      style={{ ...(styles.input as React.CSSProperties), width: 100 }}
                    />
                    {' '}{t('settings:cloud_sync_interval_hint')}
                  </label>
                </div>

                <div className={styles.fieldGroup}>
                  <label className={styles.label}>
                    <input
                      type="checkbox"
                      checked={wifiOnly}
                      onChange={(e) => setWifiOnly(e.target.checked)}
                      className={styles.checkbox}
                    />
                    {t('settings:cloud_sync_wifi_only')}
                  </label>
                </div>

                <div className={styles.fieldGroup}>
                  <label className={styles.label}>
                    <input
                      type="checkbox"
                      checked={autoImport}
                      onChange={(e) => setAutoImport(e.target.checked)}
                      className={styles.checkbox}
                    />
                    {t('settings:cloud_sync_auto_import')}
                  </label>
                  <p className={styles.hint}>{t('settings:cloud_sync_auto_import_hint')}</p>
                </div>
              </>
            )}
          </Card>

          <Card className={styles.card}>
            <h2 className={styles.sectionTitle}>
              <HardDrive size={20} style={{ marginRight: 8 }} />
              {t('settings:cloud_sync_retention')}
            </h2>

            <div className={styles.retentionGrid}>
              <label className={styles.retentionItem}>
                <input
                  type="number"
                  min={1}
                  max={100}
                  value={retention.recentFull}
                  onChange={(e) => setRetention({ ...retention, recentFull: Math.max(1, parseInt(e.target.value) || 1) })}
                  style={{ ...(styles.input as React.CSSProperties), width: 80 }}
                />
                <span>{t('settings:cloud_sync_recent_full')}</span>
              </label>

              <label className={styles.retentionItem}>
                <input
                  type="checkbox"
                  checked={retention.daily}
                  onChange={(e) => setRetention({ ...retention, daily: e.target.checked })}
                  className={styles.checkbox}
                />
                {t('settings:cloud_sync_daily')}
              </label>

              <label className={styles.retentionItem}>
                <input
                  type="checkbox"
                  checked={retention.weekly}
                  onChange={(e) => setRetention({ ...retention, weekly: e.target.checked })}
                  className={styles.checkbox}
                />
                {t('settings:cloud_sync_weekly')}
              </label>

              <label className={styles.retentionItem}>
                <input
                  type="checkbox"
                  checked={retention.monthly}
                  onChange={(e) => setRetention({ ...retention, monthly: e.target.checked })}
                  className={styles.checkbox}
                />
                {t('settings:cloud_sync_monthly')}
              </label>
            </div>
          </Card>

          <Card className={styles.card}>
            <h2 className={styles.sectionTitle}>
              <HardDrive size={20} style={{ marginRight: 8 }} />
              {t('settings:cloud_sync_actions')}
            </h2>

            <div className={styles.actionButtons}>
              <TransferButton
                variant="plain"
                onClick={handleSyncNow}
                disabled={!savedConfig || isTesting}
                busy={isSyncingNow}
              >
                {isSyncingNow ? t('settings:cloud_sync_syncing') : t('settings:cloud_sync_sync_now')}
              </TransferButton>

              <TransferButton
                variant="accent"
                onClick={handleSave}
                disabled={isLoading || isTesting || !isFormValid}
                busy={isLoading}
              >
                {savedConfig ? t('settings:cloud_sync_update') : t('settings:cloud_sync_save')}
              </TransferButton>

              <TransferButton
                variant="plain"
                onClick={handleTestConnection}
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

              {savedConfig && (
                <TransferButton
                  variant="warning"
                  onClick={handleDelete}
                  disabled={isLoading || isTesting}
                >
                  <Trash2 size={16} style={{ marginRight: 4 }} />
                  {t('settings:cloud_sync_delete')}
                </TransferButton>
              )}
            </div>

            {testResult && (
              <div
                className={styles.testResult}
                style={{
                  backgroundColor: testResult.success
                    ? 'var(--success-soft, #d1fae5)'
                    : 'var(--error-soft, #fee2e2)',
                  color: testResult.success
                    ? 'var(--success, #065f46)'
                    : 'var(--error, #991b1b)',
                  borderColor: testResult.success
                    ? 'var(--success, #065f46)'
                    : 'var(--error, #991b1b)',
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

          <Card className={[styles.card, styles.infoCard].join(' ')}>
            <h2 className={styles.sectionTitle}>
              <Info size={20} style={{ marginRight: 8 }} />
              {t('settings:cloud_sync_info_title')}
            </h2>
            <ul className={styles.infoList}>
              <li>{t('settings:cloud_sync_info_1')}</li>
              <li>{t('settings:cloud_sync_info_2')}</li>
              <li>{t('settings:cloud_sync_info_3')}</li>
              <li>{t('settings:cloud_sync_info_4')}</li>
              <li>{t('settings:cloud_sync_info_5')}</li>
            </ul>
          </Card>

          {incomingFiles.length > 0 && (
            <Card className={styles.card}>
              <h2 className={styles.sectionTitle}>
                <DownloadCloud size={20} style={{ marginRight: 8 }} />
                {t('settings:cloud_sync_incoming_title')}
              </h2>
              <p className={styles.hint}>{t('settings:cloud_sync_incoming_hint')}</p>
              <div className={styles.incomingList}>
                {incomingFiles.map((file) => {
                  const nameParts = file.split('/');
                  const fileName = nameParts[nameParts.length - 1] || file;
                  return (
                    <div key={file} className={styles.incomingItem}>
                      <span className={styles.incomingName}>{fileName}</span>
                      <TransferButton
                        variant="accent"
                        onClick={() => handleImportIncoming(file)}
                        busy={importingFile === file}
                        disabled={!!importingFile}
                      >
                        {t('settings:cloud_sync_import')}
                      </TransferButton>
                    </div>
                  );
                })}
              </div>
            </Card>
          )}

          {savedConfig && (
            <Card className={styles.statusCard}>
              <h2 className={styles.sectionTitle}>
                <Clock size={20} style={{ marginRight: 8 }} />
                {t('settings:cloud_sync_status')}
              </h2>
              <div className={styles.statusGrid}>
                <div className={styles.statusItem}>
                  <span className={styles.statusLabel}>{t('settings:cloud_sync_last_sync')}</span>
                  <span className={styles.statusValue}>
                    {savedConfig.lastSyncAt
                      ? new Date(savedConfig.lastSyncAt).toLocaleString()
                      : t('settings:cloud_sync_never')}
                  </span>
                </div>
                <div className={styles.statusItem}>
                  <span className={styles.statusLabel}>{t('settings:cloud_sync_connector')}</span>
                  <span className={styles.statusValue}>
                    {CONNECTOR_OPTIONS.find((o) => o.value === savedConfig?.connectorType)?.label ||
                      savedConfig?.connectorType}
                  </span>
                </div>
                <div className={styles.statusItem}>
                  <span className={styles.statusLabel}>{t('settings:cloud_sync_auto_sync')}</span>
                  <span className={styles.statusValue}>
                    {savedConfig?.enabled ? t('common:enabled') : t('common:disabled')}
                  </span>
                </div>
                <div className={styles.statusItem}>
                  <span className={styles.statusLabel}>{t('settings:cloud_sync_interval')}</span>
                  <span className={styles.statusValue}>
                    {savedConfig?.intervalSecs ? `${Math.round(savedConfig.intervalSecs / 60)} 分钟` : '—'}
                  </span>
                </div>
              </div>
            </Card>
          )}

          <PasswordVerificationDialog
            open={showPasswordDialog}
            onClose={handlePasswordCancelled}
            onVerify={async (_password: string) => {
              passwordVerifiedRef.current = true;
              setShowPasswordDialog(false);
              await doSave();
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