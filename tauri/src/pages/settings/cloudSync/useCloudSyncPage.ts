/**
 * P007：云同步设置页状态与处理器 hook（自 CloudSyncPage.tsx 拆出）。
 * 承载全部表单状态、配置加载/保存/删除/测试、立即同步与下行导入逻辑；
 * 纯展示的 section 子组件见同目录各 Section 文件。
 */
import { useState, useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { useToastError } from '@/hooks/useToastError';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import {
  DEFAULT_RETENTION,
  DEFAULT_WEBDAV_CONFIG,
  type RetentionPolicy,
  type SavedCloudSyncConfig,
} from './cloudSyncShared';

export function useCloudSyncPage() {
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
  const [savedConfig, setSavedConfig] = useState<SavedCloudSyncConfig | null>(null);
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
      const config = await invoke<
        (SavedCloudSyncConfig & { autoImport?: boolean }) | null
      >('cloud_sync_get_config', { accountId });
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
  const isFormValid = Boolean(configJson.baseUrl && configJson.username && configJson.password);

  return {
    // form state + setters（section 组件按需取用）
    connectorType,
    setConnectorType,
    configJson,
    setConfigJson,
    enabled,
    setEnabled,
    intervalSecs,
    setIntervalSecs,
    wifiOnly,
    setWifiOnly,
    autoImport,
    setAutoImport,
    retention,
    setRetention,
    // ui state
    isLoading,
    isTesting,
    testResult,
    savedConfig,
    incomingFiles,
    isSyncingNow,
    importingFile,
    showPasswordDialog,
    setShowPasswordDialog,
    // handlers
    handleSave,
    doSave,
    handleDelete,
    handleTestConnection,
    handleSyncNow,
    handleImportIncoming,
    handlePasswordCancelled,
    passwordVerifiedRef,
    isFormValid,
  };
}
