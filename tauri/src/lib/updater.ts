import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export interface UpdateInfo {
  version: string;
  body?: string;
  date?: string;
}

export type UpdateProgress = DownloadEvent;

/**
 * 检查是否有可用更新。
 * 返回 null 表示当前已是最新版或检查失败。
 */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  try {
    const update = await check();
    if (!update) {
      return null;
    }
    return {
      version: update.version,
      body: update.body,
      date: update.date,
    };
  } catch (error) {
    // 网络异常时静默失败，避免打扰用户
    // eslint-disable-next-line no-console
    console.warn('[updater] check failed:', error);
    return null;
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

