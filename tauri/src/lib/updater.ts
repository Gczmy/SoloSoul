import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { Channel, Resource } from '@tauri-apps/api/core';
import { logger } from '@/lib/logger';

interface UpdateInfo {
  version: string;
  body?: string;
  date?: string;
}

export type UpdateProgress = DownloadEvent;

type UpdateCheckResult =
  | { kind: 'available'; info: UpdateInfo; update: Update }
  | { kind: 'up-to-date' }
  | { kind: 'error'; message?: string };

// ── Desktop updater (Tauri plugin-updater) ─────────────────────

/**
 * 检查是否有可用更新。
 * - 'available': 有新版本可更新
 * - 'up-to-date': 当前已是最新版
 * - 'error': 检查失败（如网络异常、端点不可达）
 */
// T003: updater 插件默认无请求超时，直连黑洞（hang 而非 RST）时会卡住代理回退；
// 显式传 15s 超时（毫秒），超时后插件自动尝试下一个 endpoint。
const UPDATE_REQUEST_TIMEOUT_MS = 15_000;

export async function checkForUpdate(): Promise<UpdateCheckResult> {
  try {
    const update = await check({ timeout: UPDATE_REQUEST_TIMEOUT_MS });
    if (!update) {
      return { kind: 'up-to-date' };
    }
    return {
      kind: 'available',
      info: {
        version: update.version,
        body: update.body,
        date: update.date,
      },
      update,
    };
  } catch (error) {
    // 网络异常时静默失败，避免打扰用户
    logger.warn('[updater] check failed:', error);
    return {
      kind: 'error',
      message: error instanceof Error ? error.message : String(error),
    };
  }
}

/** 取消是正常用户操作，不作为下载失败展示。 */
export function isUpdateDownloadCancelled(error: unknown): boolean {
  return (
    (error instanceof Error && error.name === 'AbortError') ||
    String(error).includes('UPDATE_DOWNLOAD_CANCELLED')
  );
}

function cancelledDownload(): Error {
  return new DOMException('Update download cancelled', 'AbortError');
}

/**
 * 先登记可取消操作，再启动下载，保证「立即取消」不会早于后端注册。
 * 取消后等待原生命令退出，避免旧请求仍写文件时已经允许下一次重试。
 */
async function withUpdateDownload<T>(
  signal: AbortSignal | undefined,
  run: (operationId: number) => Promise<T>,
  discard?: (result: T) => Promise<void>,
): Promise<T> {
  if (signal?.aborted) throw cancelledDownload();
  const operation = new Resource(await invoke<number>('create_update_download'));
  let cancellation: Promise<void> | undefined;
  const cancel = () => {
    cancellation ??= invoke<void>('cancel_update_download', { operationId: operation.rid }).catch(
      (error) => logger.warn('[updater] cancel request failed:', error),
    );
  };
  signal?.addEventListener('abort', cancel, { once: true });
  let result: T;
  try {
    if (signal?.aborted) {
      cancel();
      throw cancelledDownload();
    }
    result = await run(operation.rid);
  } catch (error) {
    if (signal?.aborted || isUpdateDownloadCancelled(error)) throw cancelledDownload();
    throw error;
  } finally {
    signal?.removeEventListener('abort', cancel);
    await cancellation;
    await operation
      .close()
      .catch((error) => logger.warn('[updater] release operation failed:', error));
  }
  if (signal?.aborted) {
    await discard?.(result);
    throw cancelledDownload();
  }
  return result;
}

/** Rust 持有已验签数据与其 Update，前端不回传下载地址或签名。 */
export class DownloadedDesktopUpdate extends Resource {
  private released = false;

  async install(): Promise<void> {
    if (this.released) throw new Error('Downloaded update already released');
    await invoke<void>('desktop_install_update', { downloadRid: this.rid });
    this.released = true; // 安装成功时 Rust 已消费资源；失败时保留供重试。
  }

  override async close(): Promise<void> {
    if (this.released) return;
    this.released = true;
    await super.close();
  }
}

export function downloadDesktopUpdate(
  update: Update,
  onProgress?: (progress: UpdateProgress) => void,
  signal?: AbortSignal,
): Promise<DownloadedDesktopUpdate> {
  return withUpdateDownload(
    signal,
    async (operationId) => {
      const onEvent = new Channel<UpdateProgress>();
      onEvent.onmessage = (progress) => {
        if (!signal?.aborted) onProgress?.(progress);
      };
      const rid = await invoke<number>('desktop_download_update', {
        updateRid: update.rid,
        operationId,
        onEvent,
      });
      return new DownloadedDesktopUpdate(rid);
    },
    (downloaded) => downloaded.close(),
  );
}

/** 下载期间可取消；开始安装后交给原生安装器，不再中断文件替换。 */
export async function downloadAndInstallUpdate(
  onProgress?: (progress: UpdateProgress) => void,
  signal?: AbortSignal,
  onInstalling?: () => void,
): Promise<void> {
  let update: Update | null = null;
  let downloaded: DownloadedDesktopUpdate | undefined;
  try {
    if (signal?.aborted) throw cancelledDownload();
    update = await check({ timeout: UPDATE_REQUEST_TIMEOUT_MS });
    if (signal?.aborted) throw cancelledDownload();
    if (!update) throw new Error('No update available');
    downloaded = await downloadDesktopUpdate(update, onProgress, signal);
    if (signal?.aborted) throw cancelledDownload();
    onInstalling?.();
    await downloaded.install();
    await relaunch();
  } finally {
    await downloaded
      ?.close()
      .catch((error) => logger.warn('[updater] release download failed:', error));
    await update?.close().catch((error) => logger.warn('[updater] release update failed:', error));
  }
}

// ── Desktop check (updater plugin + GitHub Release notes) ─────

interface DesktopUpdateInfo {
  latestVersion: string;
  currentVersion: string;
  /** 是否为强制更新（Release body 包含 [MANDATORY] 标记） */
  mandatory: boolean;
  releaseNotes: string | null;
  publishedAt: string | null;
}

type DesktopUpdateCheckResult =
  | { kind: 'available'; info: DesktopUpdateInfo }
  | { kind: 'up-to-date' }
  | { kind: 'error'; message?: string };

/**
 * 检查桌面端更新（版本检测走 Tauri updater 插件，release notes 通过 GitHub API 补全）。
 * - 'available': 有新版本可更新
 * - 'up-to-date': 当前已是最新版
 * - 'error': 检查失败（如网络异常、端点不可达）
 */
export async function desktopCheckForUpdate(): Promise<DesktopUpdateCheckResult> {
  try {
    const info = await invoke<DesktopUpdateInfo>('desktop_check_update');
    if (info.latestVersion === info.currentVersion) {
      return { kind: 'up-to-date' };
    }
    return { kind: 'available', info };
  } catch (error) {
    logger.warn('[updater] desktop check failed:', error);
    return {
      kind: 'error',
      message: error instanceof Error ? error.message : String(error),
    };
  }
}

// ── Android self-update (GitHub API + APK download + install) ──

export interface AndroidUpdateInfo {
  latestVersion: string;
  currentVersion: string;
  downloadUrl: string | null;
  /** SHA-256 校验和（hex 编码），空字符串表示不可用 */
  checksum: string;
  /** P012: 校验和不可用原因（签名缺失/验签失败/资产缺失），用于展示可感知警告 */
  checksumWarning: string | null;
  /** 是否为强制更新（Release body 包含 [MANDATORY] 标记） */
  mandatory: boolean;
  releaseNotes: string | null;
  publishedAt: string | null;
  apkSize: number | null;
}

export interface ApkDownloadProgress {
  progress: number;
  downloaded: number;
  total: number;
  done: boolean;
  error: string | null;
}

type AndroidUpdateCheckResult =
  | { kind: 'available'; info: AndroidUpdateInfo }
  | { kind: 'up-to-date' }
  | { kind: 'error'; message?: string };

/**
 * 检查 Android GitHub Release 更新。
 */
export async function androidCheckForUpdate(): Promise<AndroidUpdateCheckResult> {
  try {
    const info = await invoke<AndroidUpdateInfo>('android_check_update');
    if (info.latestVersion === info.currentVersion) {
      return { kind: 'up-to-date' };
    }
    return { kind: 'available', info };
  } catch (error) {
    logger.warn('[updater] android check failed:', error);
    return {
      kind: 'error',
      message: error instanceof Error ? error.message : String(error),
    };
  }
}

/**
 * 共享 APK 下载流程。进度 Channel 只属于本次操作，迟到事件不串到重试。
 * URL/校验和仍由 Rust 按版本号读取并验签；只有原生命令成功退出才算完成。
 */
export async function ensureApkDownloaded(
  version: string,
  onProgress?: (progress: ApkDownloadProgress) => void,
  signal?: AbortSignal,
): Promise<boolean> {
  if (signal?.aborted) throw cancelledDownload();
  const alreadyDownloaded = await androidIsApkDownloaded(version);
  if (signal?.aborted) throw cancelledDownload();
  if (alreadyDownloaded) return false;
  return withUpdateDownload(signal, async (operationId) => {
    const onEvent = new Channel<ApkDownloadProgress>();
    onEvent.onmessage = (progress) => {
      if (!signal?.aborted) onProgress?.(progress);
    };
    await invoke<void>('android_download_apk', { version, operationId, onEvent });
    return true;
  });
}

/**
 * 安装已下载的 Android APK（调用系统包安装器）。
 */
export async function androidInstallApk(version: string): Promise<void> {
  const filePath = await invoke<string>('android_get_apk_path', { version });
  await invoke('android_install_apk', { filePath });
}

/**
 * 检查 APK 是否已下载。
 */
async function androidIsApkDownloaded(version: string): Promise<boolean> {
  return invoke<boolean>('android_is_apk_downloaded', { version });
}
