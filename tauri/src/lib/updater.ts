import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { logger } from '@/lib/logger';

export interface UpdateInfo {
  version: string;
  body?: string;
  date?: string;
}

export type UpdateProgress = DownloadEvent;

export type UpdateCheckResult =
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
export async function checkForUpdate(): Promise<UpdateCheckResult> {
  try {
    const update = await check();
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

/**
 * 下载并安装更新，同时通过回调报告进度事件。
 * 安装完成后自动重启应用。
 */
export async function downloadAndInstallUpdate(
  onProgress?: (progress: UpdateProgress) => void,
): Promise<void> {
  const update = await check();
  if (!update) {
    throw new Error('No update available');
  }

  await update.downloadAndInstall((event) => {
    onProgress?.(event);
  });

  await relaunch();
}

// ── Desktop check (updater plugin + GitHub Release notes) ─────

export interface DesktopUpdateInfo {
  latestVersion: string;
  currentVersion: string;
  /** 是否为强制更新（Release body 包含 [MANDATORY] 标记） */
  mandatory: boolean;
  releaseNotes: string | null;
  publishedAt: string | null;
}

export type DesktopUpdateCheckResult =
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

export type AndroidUpdateCheckResult =
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
 * 下载 Android APK 并监听进度事件。
 * 如果提供了 `checksum`（非空），下载完成后自动验证 SHA-256。
 *
 * 返回一个取消监听函数。
 */
export async function androidDownloadApk(
  version: string,
  downloadUrl: string,
  checksum: string,
  onProgress?: (progress: ApkDownloadProgress) => void,
): Promise<UnlistenFn> {
  const unlisten = await listen<ApkDownloadProgress>('apk-download-progress', (event) => {
    onProgress?.(event.payload);
  });

  // 在后台启动下载（不 await，让事件驱动进度）
  // expectedChecksum: 传入空字符串或有效 hex；Rust 端根据非空决定是否校验
  invoke<void>('android_download_apk', {
    version,
    downloadUrl: downloadUrl,
    expectedChecksum: checksum || null,
  }).catch((err) => {
    logger.error('[updater] android download failed:', err);
    onProgress?.({
      progress: 0,
      downloaded: 0,
      total: 0,
      done: true,
      error: String(err),
    });
  });

  return unlisten;
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
export async function androidIsApkDownloaded(version: string): Promise<boolean> {
  return invoke<boolean>('android_is_apk_downloaded', { version });
}

export type { Update };
