import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
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

/**
 * 下载并安装更新，同时通过回调报告进度事件。
 * 安装完成后自动重启应用。
 */
export async function downloadAndInstallUpdate(
  onProgress?: (progress: UpdateProgress) => void,
): Promise<void> {
  const update = await check({ timeout: UPDATE_REQUEST_TIMEOUT_MS });
  if (!update) {
    throw new Error('No update available');
  }

  await update.downloadAndInstall((event) => {
    onProgress?.(event);
  });

  await relaunch();
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
 * 下载 Android APK 并监听进度事件。
 *
 * P002: 下载 URL 与校验和由 Rust 端按 version 重新拉取 GitHub Release 元数据
 * 并重新验签（不信任前端回传，杜绝 XSS 诱导下载任意 APK），因此前端仅传 version。
 *
 * 返回一个取消监听函数。
 */
export async function androidDownloadApk(
  version: string,
  onProgress?: (progress: ApkDownloadProgress) => void,
): Promise<UnlistenFn> {
  const unlisten = await listen<ApkDownloadProgress>('apk-download-progress', (event) => {
    onProgress?.(event.payload);
  });

  // 在后台启动下载（不 await，让事件驱动进度）
  invoke<void>('android_download_apk', {
    version,
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
 * P010: Android APK 下载流程统一封装——两个 hook（useUpdateChecker / useAppUpdate）
 * 曾各自重复实现「检查已下载 → 启动事件驱动下载 → 等待 done → 清理事件监听」约 80 行。
 *
 * 返回 true 表示本次完成了实际下载（调用方无需再查是否已下载）；
 * 若 APK 已存在则直接返回 false（调用方据此跳转到安装阶段）。
 *
 * P002: URL/校验和参数已移除——Rust 端按 version 重新拉取元数据并验签。
 *
 * @param onProgress 下载进度回调（含 done/error 终态）。
 */
export async function ensureApkDownloaded(
  version: string,
  onProgress?: (progress: ApkDownloadProgress) => void,
): Promise<boolean> {
  const alreadyDownloaded = await androidIsApkDownloaded(version);
  if (alreadyDownloaded) {
    return false;
  }
  // 启动下载（事件驱动进度），等待 done 终态；unlisten 用于完成后移除事件监听防止泄漏
  let unlistenFn: UnlistenFn | undefined;
  let settled = false;
  try {
    await new Promise<void>((resolve, reject) => {
      androidDownloadApk(version, (progress) => {
        onProgress?.(progress);
        if (progress.done && !settled) {
          settled = true;
          if (progress.error) {
            reject(new Error(progress.error));
          } else {
            resolve();
          }
        }
      })
        .then((fn) => {
          unlistenFn = fn;
        })
        .catch((err) => {
          if (!settled) {
            settled = true;
            reject(err);
          }
        });
    });
  } finally {
    // 无论成功或失败，都移除 Tauri 事件监听器，防止累积泄漏
    unlistenFn?.();
  }
  return true;
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

