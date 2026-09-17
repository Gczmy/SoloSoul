import { useState, useEffect, useCallback, useRef } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import {
  desktopCheckForUpdate,
  downloadAndInstallUpdate,
  androidCheckForUpdate,
  androidInstallApk,
  ensureApkDownloaded,
  isUpdateDownloadCancelled,
  type UpdateProgress,
  type ApkDownloadProgress,
  type UpdateTransferInfo,
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
  /** P012: 校验和不可用原因，展示为警告 */
  checksumWarning?: string;
  mandatory?: boolean;
}

/** AboutPage 的更新检查与双平台下载/安装状态机。 */
export function useUpdateChecker() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [versionInfo, setVersionInfo] = useState<VersionInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);

  // 更新下载/安装状态
  const [updatePhase, setUpdatePhase] = useState<
    'idle' | 'downloading' | 'cancelling' | 'installing'
  >('idle');
  const mountedRef = useRef(true);
  const checkRevisionRef = useRef(0);
  const operationRef = useRef<{
    controller: AbortController;
    installing: boolean;
  } | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<
    UpdateProgress | ApkDownloadProgress | null
  >(null);
  const [transfer, setTransfer] = useState<UpdateTransferInfo>();
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState(0);
  const [downloadError, setDownloadError] = useState<string | null>(null);

  const runCheck = useCallback(async () => {
    if (operationRef.current) return;
    const revision = ++checkRevisionRef.current;
    const isCurrent = () => mountedRef.current && checkRevisionRef.current === revision;
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
                  checksumWarning: result.info.checksumWarning || undefined,
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
      if (!isCurrent()) return;
      setInfo(app);
      setVersionInfo({ ...ver, currentVersion: app.version });
    } catch (err) {
      // get_app_info 失败：保留现有信息，仅结束加载态。
      // P227: 更新检查静默失败可接受（非关键路径），但需留痕。
      if (isCurrent()) logger.warn('[useUpdateChecker] Initial check failed:', err);
    } finally {
      if (isCurrent()) {
        setChecking(false);
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    void runCheck();
    return () => {
      mountedRef.current = false;
      checkRevisionRef.current += 1;
      const operation = operationRef.current;
      // 文件替换/系统安装器启动后不再中断；未结束的下载由原生层清理。
      if (operation && !operation.installing) operation.controller.abort();
    };
  }, [runCheck]);

  const handleUpdate = useCallback(async () => {
    if (!mountedRef.current || operationRef.current) return;
    const operation = { controller: new AbortController(), installing: false };
    operationRef.current = operation;
    checkRevisionRef.current += 1;
    const isCurrent = () => mountedRef.current && operationRef.current === operation;
    const canProgress = () => isCurrent() && !operation.controller.signal.aborted;
    const startInstalling = () => {
      if (!canProgress()) return;
      operation.installing = true;
      setUpdatePhase('installing');
    };
    setChecking(false);
    setUpdatePhase('downloading');
    setTransfer({ phase: 'probing' });
    let receivedTransfer = false;
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
        if (versionInfo?.downloadUrl) {
          // P010: 统一封装——检查已下载、事件驱动下载、清理监听
          // P002: URL/校验和由 Rust 端按 version 重新拉取并验签，前端不再回传
          await ensureApkDownloaded(
            targetVersion,
            (progress) => {
              if (!canProgress()) return;
              setDownloadProgress(progress);
              setDownloadedBytes(progress.downloaded);
              setTotalBytes(progress.total);
              setTransfer({
                phase: progress.phase ?? 'downloading',
                source: progress.source,
                bytesPerSecond: progress.bytesPerSecond,
              });
            },
            operation.controller.signal,
          );
        }
        if (!canProgress()) return;
        startInstalling();
        // 安装已下载的 APK
        await androidInstallApk(targetVersion);
      } else {
        // 桌面端更新流程
        await downloadAndInstallUpdate(
          (progress) => {
            if (!canProgress()) return;
            setDownloadProgress(progress);
            if (progress.event === 'Transfer') {
              receivedTransfer = true;
              const { downloaded, total, ...details } = progress.data;
              setDownloadedBytes(downloaded);
              setTotalBytes(total);
              setTransfer(details);
            } else if (progress.event === 'Started' && !receivedTransfer) {
              setTransfer({ phase: 'downloading' });
              setTotalBytes(progress.data.contentLength ?? 0);
            } else if (progress.event === 'Progress' && !receivedTransfer) {
              setDownloadedBytes((prev) => prev + (progress.data.chunkLength ?? 0));
            }
          },
          operation.controller.signal,
          startInstalling,
        );
      }
    } catch (err) {
      if (isCurrent() && !operation.controller.signal.aborted && !isUpdateDownloadCancelled(err)) {
        setDownloadError(err instanceof Error ? err.message : String(err));
      }
    } finally {
      // 只有原生下载真正结束后才允许重试，避免取消与重试共享同一临时文件。
      if (operationRef.current === operation) {
        operationRef.current = null;
        if (mountedRef.current) {
          setUpdatePhase('idle');
          setTransfer(undefined);
          setDownloadProgress(null);
          setDownloadedBytes(0);
          setTotalBytes(0);
        }
      }
    }
  }, [versionInfo]);

  const cancelDownload = useCallback(() => {
    const operation = operationRef.current;
    if (!operation || operation.installing || operation.controller.signal.aborted) return;
    setUpdatePhase('cancelling');
    operation.controller.abort();
  }, []);

  const progressPercent =
    totalBytes > 0 ? Math.min(Math.round((downloadedBytes / totalBytes) * 100), 100) : 0;

  const isMandatory = versionInfo?.mandatory === true;

  return {
    info,
    versionInfo,
    loading,
    checking,
    downloading: updatePhase !== 'idle',
    cancelling: updatePhase === 'cancelling',
    installing: updatePhase === 'installing',
    downloadProgress,
    transfer,
    downloadedBytes,
    totalBytes,
    downloadError,
    progressPercent,
    isMandatory,
    runCheck,
    handleUpdate,
    cancelDownload,
  };
}
