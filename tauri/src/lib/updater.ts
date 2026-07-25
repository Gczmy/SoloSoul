import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
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

export type { Update };
