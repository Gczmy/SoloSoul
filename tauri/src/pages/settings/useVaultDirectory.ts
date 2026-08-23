/**
 * P010：Vault 目录设置区状态与处理器 hook（自 VaultDirectorySection.tsx 拆出）。
 * 承载目录信息加载、SAF 选择/迁移/回切、双向同步、进度事件监听与重启流程；
 * 展示层见 VaultDirectorySection.tsx。
 */
import { useEffect, useState, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useToastError } from '@/hooks/useToastError';
import { getPlatform } from '@/lib/platform';
import { relaunch } from '@tauri-apps/plugin-process';
import { listen } from '@tauri-apps/api/event';
import {
  getVaultDirectory,
  setVaultDirectory,
  pickVaultDirectory,
  syncVaultToRemote,
  syncVaultFromRemote,
  type VaultDirectoryInfo,
} from '@/lib/vaultDirectory';
import { useUiStore } from '@/stores/uiStore';

export interface SyncProgress {
  phase: 'sync_to_remote' | 'sync_from_remote' | 'migrate' | 'auto_sync';
  current: number;
  total: number;
}

export function useVaultDirectory() {
  const { t } = useTranslation(['settings', 'common']);
  const { onError, onSuccess } = useToastError();

  const [info, setInfo] = useState<VaultDirectoryInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const [acting, setActing] = useState(false);
  const [needsRestart, setNeedsRestart] = useState(false);
  const [showResetConfirm, setShowResetConfirm] = useState(false);
  const [platformName, setPlatformName] = useState<string>('');
  const [syncProgress, setSyncProgress] = useState<SyncProgress | null>(null);
  const unmountedRef = useRef(false);

  const loadInfo = useCallback(async () => {
    try {
      setLoading(true);
      setLoadError(false);
      const data = await getVaultDirectory();
      setInfo(data);
    } catch (e) {
      setLoadError(true);
      onError(e, t('settings:vault_directory_load_failed'));
    } finally {
      setLoading(false);
    }
  }, [t, onError]);

  useEffect(() => {
    getPlatform().then((p) => {
      setPlatformName(p);
      if (p === 'android') {
        loadInfo();
      } else {
        setLoading(false);
      }
    });

    let unlisten: (() => void) | null = null;
    listen<SyncProgress>('sync-progress', (event) => {
      const { phase, current, total } = event.payload;
      if (phase === 'auto_sync') return;
      if (current >= total) {
        setSyncProgress({ phase, current, total });
        setTimeout(() => {
          if (!unmountedRef.current) setSyncProgress(null);
        }, 2000);
      } else {
        setSyncProgress({ phase, current, total });
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unmountedRef.current = true;
      if (unlisten) unlisten();
    };
  }, [loadInfo]);

  /** 切换目录成功后的共同收尾：清授权失效横幅 + 刷新信息。 */
  const afterDirectorySwitched = useCallback(async () => {
    setNeedsRestart(true);
    // 目录已切换成功（SAF 授权已恢复或切回本地）→ 清除授权失效常驻横幅，
    // 避免 GlobalSyncIndicator 的红色「SAF 目录访问已失效」横幅一直悬挂。
    useUiStore.getState().setSafAuthRevoked(false);
    useUiStore.getState().setSafAuthToastShown(false);
    useUiStore.getState().setSafSyncError(null);
    useUiStore.getState().setSafSyncState('idle');
    await loadInfo();
  }, [loadInfo]);

  const handlePickAndSet = async () => {
    const { pause, resume } = await import('@/stores/autoLockPauseStore').then(
      (m) => m.useAutoLockPauseStore.getState(),
    );
    pause();
    try {
      setActing(true);
      const uri = await pickVaultDirectory();
      if (!uri) return;
      setSyncProgress({ phase: 'migrate', current: 0, total: 3 });
      const result = await setVaultDirectory(uri);
      if (result.success) {
        onSuccess(t('settings:vault_directory_set_success'));
        await afterDirectorySwitched();
      } else {
        onError(new Error(result.message), t('settings:vault_directory_set_failed'));
      }
    } catch (e) {
      onError(e, t('settings:vault_directory_set_failed'));
    } finally {
      resume();
      setActing(false);
    }
  };

  const handleResetLocal = () => {
    setShowResetConfirm(true);
  };

  const handleConfirmResetLocal = async () => {
    setShowResetConfirm(false);
    try {
      setActing(true);
      const result = await setVaultDirectory(null);
      if (result.success) {
        onSuccess(t('settings:vault_directory_reset_success'));
        // 切回本地目录同样清除授权失效横幅状态
        await afterDirectorySwitched();
      } else {
        onError(new Error(result.message), t('settings:vault_directory_reset_failed'));
      }
    } catch (e) {
      onError(e, t('settings:vault_directory_reset_failed'));
    } finally {
      setActing(false);
    }
  };

  const handleSyncToRemote = async () => {
    try {
      setActing(true);
      setSyncProgress({ phase: 'sync_to_remote', current: 0, total: 1 });
      await syncVaultToRemote();
      onSuccess(t('settings:vault_directory_sync_to_remote_success'));
    } catch (e) {
      setSyncProgress(null);
      onError(e, t('settings:vault_directory_sync_to_remote_failed'));
    } finally {
      setActing(false);
    }
  };

  const handleSyncFromRemote = async () => {
    try {
      setActing(true);
      setSyncProgress({ phase: 'sync_from_remote', current: 0, total: 1 });
      await syncVaultFromRemote();
      onSuccess(t('settings:vault_directory_sync_from_remote_success'));
      setNeedsRestart(true);
    } catch (e) {
      setSyncProgress(null);
      onError(e, t('settings:vault_directory_sync_from_remote_failed'));
    } finally {
      setActing(false);
    }
  };

  const getProgressLabel = useCallback(
    (phase: SyncProgress['phase']): string => {
      switch (phase) {
        case 'sync_to_remote': return t('settings:vault_directory_sync_to_remote');
        case 'sync_from_remote': return t('settings:vault_directory_sync_from_remote');
        case 'migrate': return t('settings:vault_directory_migrating_title');
        default: return '';
      }
    },
    [t],
  );

  const handleRestart = async () => {
    try {
      await relaunch();
    } catch (e) {
      onError(e, t('settings:vault_directory_restart_failed'));
    }
  };

  return {
    info,
    loading,
    loadError,
    acting,
    needsRestart,
    showResetConfirm,
    setShowResetConfirm,
    platformName,
    syncProgress,
    getProgressLabel,
    loadInfo,
    handlePickAndSet,
    handleResetLocal,
    handleConfirmResetLocal,
    handleSyncToRemote,
    handleSyncFromRemote,
    handleRestart,
  };
}
