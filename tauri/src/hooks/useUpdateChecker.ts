import { useState, useEffect, useCallback } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import {
  desktopCheckForUpdate,
  downloadAndInstallUpdate,
  androidCheckForUpdate,
  androidDownloadApk,
  androidInstallApk,
  androidIsApkDownloaded,
  type UpdateProgress,
  type ApkDownloadProgress,
} from '@/lib/updater';
import { isMobilePlatformSync } from '@/lib/platform';
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
  mandatory?: boolean;
}

/** AboutPage 的更新检查与双平台下载/安装状态机。 */
export function useUpdateChecker() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [versionInfo, setVersionInfo] = useState<VersionInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);

  // 更新下载/安装状态
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<
    UpdateProgress | ApkDownloadProgress | null
  >(null);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState(0);
  const [downloadError, setDownloadError] = useState<string | null>(null);

  const runCheck = useCallback(async () => {
    setChecking(true);
    const isMobilePlatform = isMobilePlatformSync();
    try {
      const [app, ver] = await Promise.all([
        invoke<AppInfo>('get_app_info'),
        isMobilePlatform
          ? androidCheckForUpdate().then((result) => {
              if (result.kind === 'available') {
                return {
                  currentVersion: '',
                  latestVersion: result.info.latestVersion,
                  downloadUrl: result.info.downloadUrl,
                  checksum: result.info.checksum,
                  mandatory: result.info.mandatory,
                  state: 'available' as const,
                  body: result.info.releaseNotes || undefined,
                };
              }
              if (result.kind === 'error') {
                return {
                  currentVersion: '',
                  latestVersion: null,
                  downloadUrl: null,
                  state: 'error' as const,
                  error: result.message,
                };
              }
              return {
                currentVersion: '',
                latestVersion: null,
                downloadUrl: null,
                state: 'up-to-date' as const,
              };
            })
          : desktopCheckForUpdate().then((result) => {
              if (result.kind === 'available') {
                return {
                  currentVersion: '',
                  latestVersion: result.info.latestVersion,
                  downloadUrl: null,
                  checksum: undefined,
                  mandatory: result.info.mandatory,
                  state: 'available' as const,
                  body: result.info.releaseNotes || undefined,
                };
              }
              if (result.kind === 'error') {
                return {
                  currentVersion: '',
                  latestVersion: null,
                  downloadUrl: null,
                  state: 'error' as const,
                  error: result.message,
                };
              }
              return {
                currentVersion: '',
                latestVersion: null,
                downloadUrl: null,
                state: 'up-to-date' as const,
              };
            }),
      ]);
      setInfo(app);
      setVersionInfo({ ...ver, currentVersion: app.version });
    } catch (err) {
      // get_app_info 失败：保留现有信息，仅结束加载态。
      // P227: 更新检查静默失败可接受（非关键路径），但需留痕。
      logger.warn('[useUpdateChecker] Initial check failed:', err);
    } finally {
      setChecking(false);
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    runCheck();
  }, [runCheck]);

  const handleUpdate = useCallback(async () => {
    setDownloading(true);
    setDownloadError(null);
    setDownloadProgress(null);
    setDownloadedBytes(0);
    setTotalBytes(0);
    try {
      if (isMobilePlatformSync()) {
        // Android 更新流程：下载 APK（如果尚未下载）→ 自动安装
        const targetVersion = versionInfo?.latestVersion;
        if (!targetVersion) {
          throw new Error('No target version available');
        }
        const isDownloaded = await androidIsApkDownloaded(targetVersion);
        if (!isDownloaded && versionInfo?.downloadUrl) {
          // 启动下载（监听事件驱动进度），等待下载完成
          await new Promise<void>((resolve, reject) => {
            androidDownloadApk(
              targetVersion,
              versionInfo.downloadUrl!,
              versionInfo.checksum || '',
              (progress) => {
                setDownloadProgress(progress);
                setDownloadedBytes(progress.downloaded);
                setTotalBytes(progress.total);
                if (progress.done) {
                  if (progress.error) {
                    reject(new Error(progress.error));
                  } else {
                    resolve();
                  }
                }
              },
            ).catch(reject);
          });
        }
        // 安装已下载的 APK
        await androidInstallApk(targetVersion);
        setDownloading(false);
      } else {
        // 桌面端更新流程
        await downloadAndInstallUpdate((progress) => {
          setDownloadProgress(progress);
          if (progress.event === 'Started') {
            setTotalBytes(progress.data.contentLength ?? 0);
          } else if (progress.event === 'Progress') {
            setDownloadedBytes((prev) => prev + (progress.data.chunkLength ?? 0));
          }
        });
      }
    } catch (err) {
      setDownloading(false);
      setDownloadError(err instanceof Error ? err.message : String(err));
    }
  }, [versionInfo]);

  const progressPercent =
    totalBytes > 0 ? Math.min(Math.round((downloadedBytes / totalBytes) * 100), 100) : 0;

  const isMandatory = versionInfo?.mandatory === true;

  return {
    info,
    versionInfo,
    loading,
    checking,
    downloading,
    downloadProgress,
    downloadedBytes,
    totalBytes,
    downloadError,
    progressPercent,
    isMandatory,
    runCheck,
    handleUpdate,
  };
}
