import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { relaunch } from '@tauri-apps/plugin-process';
import type { Update } from '@tauri-apps/plugin-updater';
import {
  checkForUpdate,
  androidCheckForUpdate,
  androidInstallApk,
  ensureApkDownloaded,
  type AndroidUpdateInfo,
} from '@/lib/updater';
import { isMobilePlatformSync } from '@/lib/platform';
import { ST_SKIPPED_VERSION } from '@/lib/constants';

export type AppUpdateState =
  | { kind: 'hidden' }
  | {
      kind: 'available' | 'downloading' | 'downloaded' | 'error';
      update: Update | null;
      androidInfo: AndroidUpdateInfo | null;
      version: string;
      /** 最新版本 release notes（桌面端 update.body / Android androidInfo.releaseNotes），可能为空 */
      releaseNotes: string | null;
      downloadedBytes: number;
      totalBytes: number;
      progressPercent: number;
      mandatory: boolean;
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

  const [updateState, setUpdateState] = useState<AppUpdateState>({ kind: 'hidden' });

  // 启动时检查更新并显示非侵入式横幅（桌面端 + Android）
  useEffect(() => {
    if (isMobilePlatform) {
      androidCheckForUpdate().then((result) => {
        if (result.kind !== 'available') return;
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
        });
      });
    } else {
      checkForUpdate().then((result) => {
        if (result.kind !== 'available') return;
        const skipped = localStorage.getItem(ST_SKIPPED_VERSION);
        if (skipped === result.info.version) return;
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
        });
      });
    }
  }, [isMobilePlatform]);

  const startDownload = useCallback(async () => {
    if (updateState.kind !== 'available' && updateState.kind !== 'error') return;
    setUpdateState((prev) =>
      prev.kind === 'available' || prev.kind === 'error'
        ? {
            ...prev,
            kind: 'downloading' as const,
            downloadedBytes: 0,
            totalBytes: 0,
            progressPercent: 0,
          }
        : prev,
    );
    try {
      if (isMobilePlatform) {
        // Android：通过 GitHub Release 下载 APK，事件驱动进度
        const info = updateState.androidInfo;
        if (!info || !info.downloadUrl) {
          throw new Error('No download URL available');
        }
        // P010: 统一封装——检查已下载、事件驱动下载、清理监听；
        // 返回 true 表示实际下载完成，false 表示 APK 已存在直接进入安装阶段。
        // P002: URL/校验和由 Rust 端按 version 重新拉取并验签，前端不再回传。
        await ensureApkDownloaded(updateState.version, (progress) => {
          setUpdateState((prev) => {
            if (prev.kind !== 'downloading') return prev;
            return {
              ...prev,
              downloadedBytes: progress.downloaded,
              totalBytes: progress.total,
              progressPercent: progress.progress,
            };
          });
        });
        setUpdateState((prev) =>
          prev.kind === 'downloading'
            ? { ...prev, kind: 'downloaded' as const, progressPercent: 100 }
            : prev,
        );
      } else {
        // 桌面端：使用 Tauri plugin-updater 下载
        const update = updateState.update;
        if (!update) throw new Error('No update available');
        await update.download((event) => {
          setUpdateState((prev) => {
            if (prev.kind !== 'downloading') return prev;
            if (event.event === 'Started') {
              return { ...prev, totalBytes: event.data.contentLength ?? 0 };
            }
            if (event.event === 'Progress') {
              return { ...prev, downloadedBytes: prev.downloadedBytes + event.data.chunkLength };
            }
            if (event.event === 'Finished') {
              return prev;
            }
            return prev;
          });
        });
        setUpdateState((prev) =>
          prev.kind === 'downloading' ? { ...prev, kind: 'downloaded' as const } : prev,
        );
      }
    } catch (err) {
      setUpdateState((prev) => {
        if (prev.kind !== 'downloading') return prev;
        return {
          ...prev,
          kind: 'error' as const,
          error: err instanceof Error ? err.message : String(err),
        };
      });
    }
  }, [updateState, isMobilePlatform]);

  const installUpdate = useCallback(async () => {
    if (updateState.kind !== 'downloaded') return;
    try {
      if (isMobilePlatform) {
        // Android：调用系统包安装器
        await androidInstallApk(updateState.version);
      } else {
        // 桌面端：安装并重启
        if (!updateState.update) throw new Error('No update available');
        await updateState.update.install();
        await relaunch();
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);

      // Android 用户未授予「安装未知应用」权限时，Kotlin 端会打开系统设置页
      // 并 reject 该错误。此时应保持「已下载」状态，让用户返回后再次点击安装，
      // 而不是进入 error 状态导致必须重启应用。
      if (isMobilePlatform && message.includes('NEED_INSTALL_UNKNOWN_APPS_PERMISSION')) {
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
        prev.kind === 'downloaded'
          ? {
              ...prev,
              kind: 'error' as const,
              error: message,
            }
          : prev,
      );
    }
  }, [updateState, isMobilePlatform, t]);

  /** 隐藏横幅（跳过按钮需额外写 ST_SKIPPED_VERSION，由调用方处理）。 */
  const dismissUpdate = useCallback(() => {
    setUpdateState({ kind: 'hidden' });
  }, []);

  return { updateState, startDownload, installUpdate, dismissUpdate };
}
