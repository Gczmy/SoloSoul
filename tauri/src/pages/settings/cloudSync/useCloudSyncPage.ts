/**
 * P007：云同步设置页状态与处理器 hook（自 CloudSyncPage.tsx 拆出）。
 * 承载全部表单状态、配置加载/保存/删除/测试、立即同步与下行导入逻辑；
 * 纯展示的 section 子组件见同目录各 Section 文件。
 */
import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { useToastError } from '@/hooks/useToastError';
import { useAuthStore } from '@/stores/authStore';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { importOutcomeError } from '@/lib/importOutcome';
import type { ImportResult } from '@/types/exportImport';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
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
  const requests = useMemo(createSessionRequests, []);

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
  const refreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const clear = () => {
      requests.invalidate();
      if (refreshTimer.current !== null) clearTimeout(refreshTimer.current);
      refreshTimer.current = null;
      setSavedConfig(null);
      setConnectorType('webdav');
      setConfigJson(DEFAULT_WEBDAV_CONFIG);
      setEnabled(false);
      setIntervalSecs(3600);
      setWifiOnly(true);
      setAutoImport(false);
      setRetention(DEFAULT_RETENTION);
      setIncomingFiles([]);
      setImportingFile(null);
      setIsSyncingNow(false);
      setIsLoading(false);
      setIsTesting(false);
      setTestResult(null);
      setShowPasswordDialog(false);
    };
    const unsubscribe = onRequestSessionChange(clear);
    return () => {
      unsubscribe();
      requests.invalidate();
      if (refreshTimer.current !== null) clearTimeout(refreshTimer.current);
    };
  }, [requests]);

  const loadIncoming = useCallback(async () => {
    const request = requests.begin('incoming', accountId);
    try {
      const files = await request.invoke<string[]>('cloud_sync_list_incoming');
      setIncomingFiles(files ?? []);
    } catch {
      if (request.isCurrent()) setIncomingFiles([]);
    }
  }, [accountId, requests]);

  // 加载云端待导入快照列表 + 监听下行事件
  useEffect(() => {
    if (!accountId) return;
    const lifetime = requests.begin(undefined, accountId);
    void loadIncoming();
    const unlisten = listen<{ accountId: string; sessionGeneration: number }>(
      'cloud-sync-incoming',
      (event) => {
        if (!lifetime.isCurrent() || event.payload.accountId !== accountId) return;
        // 事件只触发读取，不直接采用可能排队迟到的旧会话文件列表。
        void loadIncoming();
      },
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [accountId, loadIncoming, requests]);

  const loadConfig = useCallback(async () => {
    const request = requests.begin('config', accountId);
    try {
      setIsLoading(true);
      const config = await request.invoke<(SavedCloudSyncConfig & { autoImport?: boolean }) | null>(
        'cloud_sync_get_config',
        { accountId },
      );
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
      if (request.isCurrent()) onError(new Error(String(e)), t('settings:cloud_sync_load_failed'));
    } finally {
      if (request.isCurrent()) setIsLoading(false);
    }
  }, [accountId, onError, t, requests]);

  // Load existing config on mount
  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const handleSave = () => setShowPasswordDialog(true);

  const handleVerifyAndSave = async (password: string): Promise<boolean> => {
    const request = requests.begin('save', accountId);
    if (!accountId || !request.isCurrent()) return false;
    let saved: boolean;
    try {
      saved = await request.invoke<boolean>('cloud_sync_save_config', {
        payload: {
          accountId,
          connectorType,
          configJson,
          enabled,
          intervalSecs,
          wifiOnly,
          autoImport,
          retention,
        },
        password,
      });
    } catch (error) {
      if (!request.isCurrent()) return false;
      throw error;
    }
    if (!saved || !request.isCurrent()) return false;
    setShowPasswordDialog(false);
    onSuccess(t('settings:cloud_sync_saved'));
    void loadConfig();
    return true;
  };

  const handleDelete = async () => {
    if (!window.confirm(t('settings:cloud_sync_delete_confirm'))) return;
    const request = requests.begin('delete', accountId);
    if (!accountId || !request.isCurrent()) return;
    try {
      await request.invoke('cloud_sync_delete_config', { accountId });
      if (!request.isCurrent()) return;
      onSuccess(t('settings:cloud_sync_deleted'));
      setSavedConfig(null);
      setConnectorType('webdav');
      setConfigJson(DEFAULT_WEBDAV_CONFIG);
      setEnabled(false);
      setIntervalSecs(3600);
      setWifiOnly(true);
      setRetention(DEFAULT_RETENTION);
    } catch (e) {
      if (request.isCurrent())
        onError(new Error(String(e)), t('settings:cloud_sync_delete_failed'));
    }
  };

  const handleTestConnection = async () => {
    const request = requests.begin('connection-test', accountId);
    if (!accountId || !request.isCurrent()) return;
    setIsTesting(true);
    setTestResult(null);
    try {
      await request.invoke('cloud_sync_test_connection', {
        payload: {
          accountId,
          connectorType,
          configJson,
          enabled,
          intervalSecs,
          wifiOnly,
          autoImport,
          retention,
        },
      });
      if (!request.isCurrent()) return;
      setTestResult({ success: true });
      onSuccess(t('settings:cloud_sync_test_success'));
    } catch (e) {
      if (request.isCurrent()) {
        setTestResult({ success: false, error: String(e) });
        onError(new Error(String(e)), t('settings:cloud_sync_test_failed'));
      }
    } finally {
      if (request.isCurrent()) setIsTesting(false);
    }
  };

  const handleSyncNow = async () => {
    const request = requests.begin('sync-now', accountId);
    setIsSyncingNow(true);
    try {
      await request.invoke('cloud_sync_now');
      // 调度器异步执行；稍后刷新待导入列表
      if (refreshTimer.current !== null) clearTimeout(refreshTimer.current);
      refreshTimer.current = setTimeout(() => {
        if (request.isCurrent()) void loadIncoming();
      }, 3000);
    } catch (e) {
      if (request.isCurrent())
        onError(new Error(resolveBackendErrorMessage(e)), t('settings:cloud_sync_sync_now_failed'));
    } finally {
      if (request.isCurrent()) setIsSyncingNow(false);
    }
  };

  const handleImportIncoming = async (file: string) => {
    const request = requests.begin('import', accountId);
    // 文件名 {hlc}.solosoul，父目录名即来源 device_id
    const parts = file.split(/[\\/]/);
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
      const result = await request.invoke<ImportResult>('import_execute_advanced', {
        accountId,
        req: {
          selections: null,
          strategy: 'skipExisting',
          sourcePath: file,
          password: snapshotPw,
          selectedAttachmentIds: null,
          objectStrategies: {},
          locale: i18n.language || 'zh-CN',
        },
      });
      const incomplete = importOutcomeError(result, t);
      if (incomplete) {
        onError(new Error(incomplete), t('settings:cloud_sync_import_failed'));
        return;
      }
      await request.invoke('cloud_sync_mark_applied', {
        accountId,
        sessionGeneration: result.sessionGeneration,
        sourcePath: file,
      });
      onSuccess(t('settings:cloud_sync_import_success'));
      setIncomingFiles((prev) => prev.filter((f) => f !== file));
    } catch (e) {
      if (request.isCurrent())
        onError(new Error(resolveBackendErrorMessage(e)), t('settings:cloud_sync_import_failed'));
    } finally {
      if (request.isCurrent()) setImportingFile(null);
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
    handleVerifyAndSave,
    handleDelete,
    handleTestConnection,
    handleSyncNow,
    handleImportIncoming,
    handlePasswordCancelled,
    isFormValid,
  };
}
