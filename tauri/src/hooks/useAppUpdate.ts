import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { relaunch } from '@tauri-apps/plugin-process';
import type { Update } from '@tauri-apps/plugin-updater';
import {
  checkForUpdate,
  androidCheckForUpdate,
  androidInstallApk,
  ensureApkDownloaded,
  downloadDesktopUpdate,
  isUpdateDownloadCancelled,
  type DownloadedDesktopUpdate,
  type AndroidUpdateInfo,
} from '@/lib/updater';
import { isMobilePlatformSync } from '@/lib/platform';
import { useAuthStore } from '@/stores/authStore';
import { ST_SKIPPED_VERSION } from '@/lib/constants';
import { logger } from '@/lib/logger';

export type AppUpdateState =
  | { kind: 'hidden' }
  | {
      kind: 'available' | 'downloading' | 'cancelling' | 'downloaded' | 'installing' | 'error';
      update: Update | null;
      androidInfo: AndroidUpdateInfo | null;
      version: string;
      /** 最新版本 release notes（桌面端 update.body / Android androidInfo.releaseNotes），可能为空 */
      releaseNotes: string | null;
      downloadedBytes: number;
      totalBytes: number;
      progressPercent: number;
      mandatory: boolean;
      /** P012: APK 校验和不可用原因（Android），供横幅展示可感知警告 */
      checksumWarning?: string | null;
      error?: string;
    };

/**
 * P041: 从 AppRoutes 拆出的统一更新状态机。
 * 桌面端持有 Tauri Update 对象，Android 端持有 GitHub Release 信息；
 * `isMobilePlatform` 区分两条下载/安装路径，`mandatory` 透传给 UpdateBanner 隐藏跳过/关闭按钮。
 */
export function useAppUpdate() {
  const isMobilePlatform = isMobilePlatformSync();
  const { t } = useTranslation(['settings']);
  // P027: 解锁后重查——App 挂载时 Vault 通常未解锁，启动检查曾被守卫拦截；
  // 豁免后挂载检查可执行，同时依赖 isAuthenticated 保证解锁完成后必有一次重查
  // （挂载期网络失败/被拦截的兜底），横幅在解锁后必然出现。
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);

  const [updateState, setUpdateState] = useState<AppUpdateState>({ kind: 'hidden' });
  const activeDownload = useRef<AbortController | null>(null);
  const downloadedUpdate = useRef<DownloadedDesktopUpdate | null>(null);
  const checkedUpdate = useRef<Update | null>(null);
  const checkGeneration = useRef(0);
  const readyToInstall = useRef(false);
  const installing = useRef(false);

  useEffect(
    () => () => {
      checkGeneration.current += 1;
      activeDownload.current?.abort();
      activeDownload.current = null;
      void downloadedUpdate.current
        ?.close()
        .catch((error) => logger.warn('[updater] cleanup:', error));
      void checkedUpdate.current
        ?.close()
        .catch((error) => logger.warn('[updater] cleanup:', error));
    },
    [],
  );

  // 启动时检查更新并显示非侵入式横幅（桌面端 + Android）；解锁后重查一次
  useEffect(() => {
    // 解锁时重查不能覆盖正在下载/已下载/安装中的操作。
    if (activeDownload.current || readyToInstall.current || installing.current) return;
    const generation = ++checkGeneration.current;
    let disposed = false;
    const isCurrent = () => !disposed && generation === checkGeneration.current;
    if (isMobilePlatform) {
      androidCheckForUpdate().then((result) => {
        if (result.kind !== 'available' || !isCurrent()) return;
        const info = result.info;
        const skipped = localStorage.getItem(ST_SKIPPED_VERSION);
        if (!info.mandatory && skipped === info.latestVersion) return;
        setUpdateState({
          kind: 'available',
          update: null,
          androidInfo: info,
          version: info.latestVersion,
          releaseNotes: info.releaseNotes,
          downloadedBytes: 0,
          totalBytes: 0,
          progressPercent: 0,
          mandatory: info.mandatory,
          checksumWarning: info.checksumWarning ?? null,
        });
      });
    } else {
      checkForUpdate().then((result) => {
        if (result.kind !== 'available') return;
        if (!isCurrent()) {
          void result.update
            .close()
            .catch((error) => logger.warn('[updater] stale check cleanup:', error));
          return;
        }
        const skipped = localStorage.getItem(ST_SKIPPED_VERSION);
        if (skipped === result.info.version) {
          void result.update
            .close()
            .catch((error) => logger.warn('[updater] skipped check cleanup:', error));
          return;
        }
        void checkedUpdate.current
          ?.close()
          .catch((error) => logger.warn('[updater] replace check:', error));
        checkedUpdate.current = result.update;
        setUpdateState({
          kind: 'available',
          update: result.update,
          androidInfo: null,
          version: result.info.version,
          // 桌面端 release notes 来自 updater 插件检查结果（update.body，GitHub Release
          // 正文）；latest.json 中 notes 为空时 result.info.body 为空，横幅不显示查看按钮。
          releaseNotes: result.info.body ?? null,
          downloadedBytes: 0,
          totalBytes: 0,
          progressPercent: 0,
          mandatory: false,
          checksumWarning: null,
        });
      });
    }
    return () => {
      disposed = true;
    };
  }, [isMobilePlatform, isAuthenticated]);

  const startDownload = useCallback(async () => {
    if (
      (updateState.kind !== 'available' && updateState.kind !== 'error') ||
      activeDownload.current
    )
      return;
    const controller = new AbortController();
    activeDownload.current = controller;
    checkGeneration.current += 1; // 废弃在点击下载之前发出的迟到检查结果。
    readyToInstall.current = false;
    const isCurrent = () => activeDownload.current === controller;
    setUpdateState({
      ...updateState,
      kind: 'downloading',
      error: undefined,
      downloadedBytes: 0,
      totalBytes: 0,
      progressPercent: 0,
    });
    try {
      await downloadedUpdate.current?.close();
      downloadedUpdate.current = null;
      if (isMobilePlatform) {
        if (!updateState.androidInfo?.downloadUrl) throw new Error('No download URL available');
        await ensureApkDownloaded(
          updateState.version,
          (progress) => {
            if (!isCurrent() || controller.signal.aborted) return;
            setUpdateState((prev) =>
              prev.kind === 'downloading'
                ? {
                    ...prev,
                    downloadedBytes: progress.downloaded,
                    totalBytes: progress.total,
                    progressPercent: progress.progress,
                  }
                : prev,
            );
          },
          controller.signal,
        );
      } else {
        if (!updateState.update) throw new Error('No update available');
        const downloaded = await downloadDesktopUpdate(
          updateState.update,
          (event) => {
            if (!isCurrent() || controller.signal.aborted) return;
            setUpdateState((prev) => {
              if (prev.kind !== 'downloading') return prev;
              if (event.event === 'Started')
                return { ...prev, totalBytes: event.data.contentLength ?? 0 };
              if (event.event === 'Progress')
                return { ...prev, downloadedBytes: prev.downloadedBytes + event.data.chunkLength };
              return prev;
            });
          },
          controller.signal,
        );
        if (!isCurrent() || controller.signal.aborted) {
          await downloaded.close();
        } else {
          downloadedUpdate.current = downloaded;
        }
      }
      if (!isCurrent()) return;
      if (controller.signal.aborted) throw new DOMException('Cancelled', 'AbortError');
      readyToInstall.current = true;
      setUpdateState((prev) =>
        prev.kind === 'downloading' ? { ...prev, kind: 'downloaded', progressPercent: 100 } : prev,
      );
    } catch (error) {
      if (!isCurrent()) return;
      const cancelled = controller.signal.aborted || isUpdateDownloadCancelled(error);
      setUpdateState((prev) => {
        if (prev.kind !== 'downloading' && prev.kind !== 'cancelling') return prev;
        return cancelled
          ? {
              ...prev,
              kind: 'available',
              error: undefined,
              downloadedBytes: 0,
              totalBytes: 0,
              progressPercent: 0,
            }
          : {
              ...prev,
              kind: 'error',
              error: error instanceof Error ? error.message : String(error),
            };
      });
    } finally {
      if (isCurrent()) activeDownload.current = null;
    }
  }, [updateState, isMobilePlatform]);

  const cancelDownload = useCallback(() => {
    if (!activeDownload.current || activeDownload.current.signal.aborted) return;
    setUpdateState((prev) =>
      prev.kind === 'downloading' ? { ...prev, kind: 'cancelling' } : prev,
    );
    activeDownload.current.abort();
  }, []);

  const installUpdate = useCallback(async () => {
    if (updateState.kind !== 'downloaded' || installing.current) return;
    installing.current = true;
    setUpdateState({ ...updateState, kind: 'installing' });
    try {
      if (isMobilePlatform) {
        // Android：调用系统包安装器
        await androidInstallApk(updateState.version);
        setUpdateState((prev) =>
          prev.kind === 'installing' ? { ...prev, kind: 'downloaded' } : prev,
        );
      } else {
        // 桌面端：安装并重启
        if (!downloadedUpdate.current) throw new Error('No downloaded update available');
        await downloadedUpdate.current.install();
        await relaunch();
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);

      // Android 用户未授予「安装未知应用」权限时，Kotlin 端会打开系统设置页
      // 并 reject 该错误。此时应保持「已下载」状态，让用户返回后再次点击安装，
      // 而不是进入 error 状态导致必须重启应用。
      if (isMobilePlatform && message.includes('NEED_INSTALL_UNKNOWN_APPS_PERMISSION')) {
        setUpdateState((prev) =>
          prev.kind === 'installing' ? { ...prev, kind: 'downloaded' } : prev,
        );
        import('@/stores/uiStore').then(({ useUiStore }) => {
          useUiStore.getState().showToast({
            type: 'warning',
            message: t('settings:need_install_unknown_apps', {
              defaultValue:
                '请在系统设置中为 SoloSoul 开启「安装未知应用」权限，然后重新点击安装。',
            }),
            duration: 8000,
          });
        });
        return;
      }

      setUpdateState((prev) =>
        prev.kind === 'installing'
          ? {
              ...prev,
              kind: 'error' as const,
              error: message,
            }
          : prev,
      );
    } finally {
      installing.current = false;
    }
  }, [updateState, isMobilePlatform, t]);

  /** 隐藏横幅（跳过按钮需额外写 ST_SKIPPED_VERSION，由调用方处理）。 */
  const dismissUpdate = useCallback(() => {
    if (activeDownload.current || installing.current) return;
    checkGeneration.current += 1;
    readyToInstall.current = false;
    void downloadedUpdate.current
      ?.close()
      .catch((error) => logger.warn('[updater] dismiss:', error));
    downloadedUpdate.current = null;
    void checkedUpdate.current?.close().catch((error) => logger.warn('[updater] dismiss:', error));
    checkedUpdate.current = null;
    setUpdateState({ kind: 'hidden' });
  }, []);

  return { updateState, startDownload, cancelDownload, installUpdate, dismissUpdate };
}
