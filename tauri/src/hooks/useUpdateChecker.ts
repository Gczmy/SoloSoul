import { useEffect, useState, useCallback } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useUpdateStore } from '@/stores/updateStore';
import { logger } from '@/lib/logger';
export interface AppInfo {
  appName: string;
  version: string;
  os: string;
  arch: string;
}

export interface VersionInfo {
  currentVersion: string;
  latestVersion: string | null;
  state: 'up-to-date' | 'available' | 'error';
  body?: string;
  error?: string;
  downloadUrl?: string | null;
  checksum?: string;
  /** P012: 校验和不可用原因，展示为警告 */
  checksumWarning?: string;
  mandatory?: boolean;
}

/** “关于”页只订阅全局任务，切页和横幅共用同一取消/安装操作。 */
export function useUpdateChecker() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const store = useUpdateStore();
  useEffect(() => {
    let alive = true;
    invoke<AppInfo>('get_app_info')
      .then((app) => {
        if (alive) setInfo(app);
      })
      .catch((error) => logger.warn('[updater] app info:', error))
      .finally(() => {
        if (alive) setLoading(false);
      });
    void useUpdateStore.getState().check(true);
    return () => {
      alive = false;
    };
  }, []);
  const state = store.updateState;
  const visible = state.kind !== 'hidden' ? state : null;
  const runCheck = useCallback(() => useUpdateStore.getState().check(true), []);
  const handleUpdate = useCallback(async () => {
    const task = useUpdateStore.getState();
    if (task.updateState.kind === 'downloaded') await task.installUpdate();
    else {
      const version = task.updateState.kind !== 'hidden' ? task.updateState.version : null;
      if (task.controller) return;
      await task.startDownload();
      const after = useUpdateStore.getState();
      if (after.updateState.kind === 'downloaded' && after.updateState.version === version)
        await after.installUpdate();
    }
  }, []);
  const versionInfo: VersionInfo | null = visible
    ? {
        currentVersion: info?.version ?? '',
        latestVersion: visible.version,
        state: 'available',
        body: visible.releaseNotes ?? undefined,
        downloadUrl: visible.androidInfo?.downloadUrl,
        checksumWarning: visible.checksumWarning ?? undefined,
        mandatory: visible.mandatory,
      }
    : store.lastChecked || store.checkError
      ? {
          currentVersion: info?.version ?? '',
          latestVersion: null,
          state: store.checkError ? 'error' : 'up-to-date',
          error: store.checkError,
        }
      : null;
  return {
    info,
    versionInfo,
    loading,
    checking: store.checking,
    downloading: ['downloading', 'cancelling', 'installing'].includes(state.kind),
    cancelling: state.kind === 'cancelling',
    installing: state.kind === 'installing',
    downloaded: state.kind === 'downloaded',
    downloadProgress: null,
    transfer: visible?.transfer,
    downloadedBytes: visible?.downloadedBytes ?? 0,
    totalBytes: visible?.totalBytes ?? 0,
    downloadError: visible?.error ?? null,
    progressPercent: Math.round(visible?.progressPercent ?? 0),
    isMandatory: visible?.mandatory ?? false,
    runCheck,
    handleUpdate,
    cancelDownload: store.cancelDownload,
  };
}
