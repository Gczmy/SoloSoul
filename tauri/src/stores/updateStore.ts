import { create } from 'zustand';
import { relaunch } from '@tauri-apps/plugin-process';
import type { Update } from '@tauri-apps/plugin-updater';
import {
  checkForUpdate,
  androidCheckForUpdate,
  androidCachedUpdate,
  androidInstallApk,
  ensureApkDownloaded,
  downloadDesktopUpdate,
  isUpdateDownloadCancelled,
  type DownloadedDesktopUpdate,
  type AndroidUpdateInfo,
  type UpdateTransferInfo,
} from '@/lib/updater';
import {
  getPlatformCapabilities,
  getPlatformCapabilitiesSync,
  type PlatformCapabilities,
} from '@/lib/platformCapabilities';
import { ST_SKIPPED_VERSION } from '@/lib/constants';
import i18n from '@/lib/i18n';
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
      transfer?: UpdateTransferInfo;
      mandatory: boolean;
      /** P012: APK 校验和不可用原因（Android），供横幅展示可感知警告 */
      checksumWarning?: string | null;
      error?: string;
    };

interface UpdateStore {
  updateState: AppUpdateState;
  checking: boolean;
  checkError?: string;
  unsupportedReason: string | null;
  lastChecked: number;
  checkToken: object | null;
  checkPromise: Promise<void> | null;
  controller: AbortController | null;
  downloaded: DownloadedDesktopUpdate | null;
  check: (manual?: boolean) => Promise<void>;
  startDownload: () => Promise<void>;
  cancelDownload: () => void;
  installUpdate: () => Promise<void>;
  dismissUpdate: () => void;
}

/** 能力不可用不当作网络成功/最新版本；保留 iOS 原有展示语义。 */
function blockedUpdateState(capabilities: PlatformCapabilities): Partial<UpdateStore> | null {
  if (capabilities.update.status === 'supported' && capabilities.updateMethod !== 'none')
    return null;
  return {
    unsupportedReason:
      capabilities.update.status === 'unsupported'
        ? capabilities.update.reason === 'ios_in_app_update_not_implemented'
          ? 'ios'
          : capabilities.update.reason
        : null,
    checkError:
      capabilities.update.status === 'unavailable'
        ? i18n.t('settings:update_capability_unavailable')
        : undefined,
    checking: false,
    downloaded: null,
    updateState: { kind: 'hidden' },
  };
}

const routeUnavailableState = () =>
  blockedUpdateState(getPlatformCapabilitiesSync()) ?? {
    unsupportedReason: null,
    checkError: i18n.t('settings:update_capability_unavailable'),
    checking: false,
    downloaded: null,
    updateState: { kind: 'hidden' as const },
  };

/** 切到禁用状态后释放已持有的原生资源，避免丢弃 JS 引用留下孤立 rid。 */
function applyBlockedUpdateState(
  previous: UpdateStore,
  setState: (patch: Partial<UpdateStore>) => void,
  blocked: Partial<UpdateStore>,
): void {
  setState(blocked);
  if (previous.updateState.kind !== 'hidden' && previous.updateState.update) {
    void previous.updateState.update
      .close()
      .catch((error) => logger.warn('[updater] blocked update close:', error));
  }
  void previous.downloaded
    ?.close()
    .catch((error) => logger.warn('[updater] blocked download close:', error));
}

function isUnsupportedApkUpdateError(error: unknown): boolean {
  return error instanceof Error && 'code' in error && error.code === 'APK_UPDATE_UNSUPPORTED';
}

/** 全应用唯一任务，独立于 Vault 会话及页面生命周期；离开页面不取消网络请求。 */
export const useUpdateStore = create<UpdateStore>((set, get) => ({
  updateState: { kind: 'hidden' },
  checking: false,
  unsupportedReason: null,
  lastChecked: 0,
  checkToken: null,
  checkPromise: null,
  controller: null,
  downloaded: null,
  check: (manual = false) => {
    const current = get();
    if (current.controller || ['downloaded', 'installing'].includes(current.updateState.kind))
      return Promise.resolve();
    if (current.checkPromise) return current.checkPromise;
    const token = {};
    set({ checking: true, checkError: undefined, checkToken: token });
    const task = (async () => {
      const capabilities = await getPlatformCapabilities(manual);
      if (get().checkToken !== token || get().controller) return;
      const blocked = blockedUpdateState(capabilities);
      if (blocked) {
        applyBlockedUpdateState(get(), set, blocked);
        return;
      }
      set({ unsupportedReason: null });
      if (capabilities.updateMethod === 'android_apk' && get().updateState.kind === 'hidden') {
        try {
          const cached = await androidCachedUpdate();
          if (
            cached &&
            get().checkToken === token &&
            !get().controller &&
            (manual || localStorage.getItem(ST_SKIPPED_VERSION) !== cached.latestVersion)
          ) {
            const p = cached.cachedDownload;
            set({
              updateState: {
                kind: 'available',
                update: null,
                androidInfo: cached,
                version: cached.latestVersion,
                releaseNotes: cached.releaseNotes,
                mandatory: false,
                checksumWarning: cached.checksumWarning,
                downloadedBytes: p?.downloaded ?? 0,
                totalBytes: p?.total ?? cached.apkSize ?? 0,
                progressPercent: p?.total ? Math.min(99, (p.downloaded / p.total) * 100) : 0,
              },
            });
          }
        } catch (error) {
          if (get().checkToken !== token || get().controller) return;
          if (isUnsupportedApkUpdateError(error)) {
            applyBlockedUpdateState(get(), set, routeUnavailableState());
            return;
          }
          logger.warn('[updater] cached metadata:', error);
        }
      }
      if (get().checkToken !== token || get().controller) return;
      const result =
        capabilities.updateMethod === 'android_apk'
          ? await androidCheckForUpdate()
          : await checkForUpdate();
      if (get().checkToken !== token || get().controller) {
        if ('update' in result) await result.update.close();
        return;
      }
      if (result.kind === 'unsupported') {
        applyBlockedUpdateState(get(), set, routeUnavailableState());
        return;
      }
      if (result.kind === 'error') {
        set({ checkError: result.message || i18n.t('settings:update_check_failed') });
        return;
      }
      set({ lastChecked: Date.now() });
      if (result.kind !== 'available') return;
      const androidInfo = 'latestVersion' in result.info ? result.info : null;
      const update = 'update' in result ? result.update : null;
      const mandatory =
        androidInfo?.mandatory ??
        ('body' in result.info && !!result.info.body?.includes('[MANDATORY]'));
      const version =
        androidInfo?.latestVersion ??
        update!.version ??
        ('version' in result.info ? result.info.version : '');
      if (!manual && !mandatory && localStorage.getItem(ST_SKIPPED_VERSION) === version) {
        await update?.close();
        return;
      }
      const prev = get().updateState;
      const cached = androidInfo?.cachedDownload;
      set({
        updateState: {
          kind: cached?.done ? 'downloaded' : 'available',
          update,
          androidInfo,
          version,
          releaseNotes:
            androidInfo?.releaseNotes ??
            ('body' in result.info ? (result.info.body ?? null) : null),
          downloadedBytes: cached?.downloaded ?? 0,
          totalBytes: cached?.total ?? androidInfo?.apkSize ?? 0,
          progressPercent: cached?.total
            ? Math.min(100, (cached.downloaded / cached.total) * 100)
            : 0,
          mandatory,
          checksumWarning: androidInfo?.checksumWarning,
        },
      });
      // 先原子切换可下载资源，再释放旧资源，避免关闭期间点击更新拿到已释放的 rid。
      if (prev.kind !== 'hidden' && prev.update && prev.update !== update)
        void prev.update.close().catch((error) => logger.warn('[updater] replace check:', error));
    })()
      .catch((error) => {
        if (get().checkToken === token) {
          if (isUnsupportedApkUpdateError(error))
            applyBlockedUpdateState(get(), set, routeUnavailableState());
          else set({ checkError: String(error) });
        }
      })
      .finally(() => {
        if (get().checkToken === token) set({ checking: false, checkPromise: null });
      });
    set({ checkPromise: task });
    return task;
  },
  startDownload: async () => {
    const state = get().updateState;
    if (get().controller || !['available', 'error'].includes(state.kind) || state.kind === 'hidden')
      return;
    const controller = new AbortController();
    set({
      controller,
      checkToken: null,
      checkPromise: null,
      checking: false,
      updateState: {
        ...state,
        kind: 'downloading',
        error: undefined,
        transfer: { phase: 'probing' },
      },
    });
    const current = () => get().controller === controller;
    const progress = (downloaded: number, total: number, transfer?: UpdateTransferInfo) => {
      const prev = get().updateState;
      if (!current() || controller.signal.aborted || prev.kind !== 'downloading') return;
      // Channel 发送的是当前文件的绝对字节数，重试/换源时不能累计重复流量。
      const size = Number.isFinite(total) ? Math.max(0, total) : 0;
      const bytes = Math.max(0, Number.isFinite(downloaded) ? downloaded : 0);
      const validBytes = size ? Math.min(bytes, size) : bytes;
      set({
        updateState: {
          ...prev,
          downloadedBytes: validBytes,
          totalBytes: size,
          progressPercent: size ? Math.min(99, (validBytes / size) * 100) : 0,
          transfer,
        },
      });
    };
    try {
      const capabilities = await getPlatformCapabilities();
      if (!current()) return;
      if (controller.signal.aborted) throw new DOMException('Cancelled', 'AbortError');
      const blocked = blockedUpdateState(capabilities);
      if (blocked) {
        applyBlockedUpdateState(get(), set, blocked);
        return;
      }
      set({ unsupportedReason: null });
      if (capabilities.updateMethod === 'android_apk') {
        if (!state.androidInfo?.downloadUrl) throw new Error('No download URL available');
        await ensureApkDownloaded(
          state.version,
          (p) =>
            progress(p.downloaded, p.total, {
              phase: p.phase ?? 'downloading',
              source: p.source,
              bytesPerSecond: p.bytesPerSecond,
            }),
          controller.signal,
        );
      } else {
        if (!state.update) throw new Error('No update available');
        let transferred = false;
        const downloaded = await downloadDesktopUpdate(
          state.update,
          (event) => {
            if (event.event === 'Transfer') {
              transferred = true;
              const { downloaded, total, ...transfer } = event.data;
              progress(downloaded, total, transfer);
            } else if (!transferred && current()) {
              const prev = get().updateState;
              if (prev.kind === 'hidden') return;
              if (event.event === 'Started')
                progress(0, event.data.contentLength ?? 0, { phase: 'downloading' });
              if (event.event === 'Progress')
                progress(prev.downloadedBytes + event.data.chunkLength, prev.totalBytes, {
                  phase: 'downloading',
                });
            }
          },
          controller.signal,
        );
        if (!current() || controller.signal.aborted) await downloaded.close();
        else set({ downloaded });
      }
      if (!current()) return;
      if (controller.signal.aborted) throw new DOMException('Cancelled', 'AbortError');
      const prev = get().updateState;
      if (prev.kind === 'downloading')
        set({
          updateState: { ...prev, kind: 'downloaded', progressPercent: 100, transfer: undefined },
        });
    } catch (error) {
      if (!current()) return;
      if (!controller.signal.aborted && isUnsupportedApkUpdateError(error)) {
        applyBlockedUpdateState(get(), set, routeUnavailableState());
        return;
      }
      const prev = get().updateState;
      if (prev.kind === 'hidden') return;
      const cancelled = controller.signal.aborted || isUpdateDownloadCancelled(error);
      set({
        updateState: {
          ...prev,
          kind: cancelled ? 'available' : 'error',
          transfer: undefined,
          error: cancelled ? undefined : String(error),
        },
      });
    } finally {
      if (current()) set({ controller: null });
    }
  },
  cancelDownload: () => {
    const { controller, updateState } = get();
    if (!controller || controller.signal.aborted || updateState.kind !== 'downloading') return;
    set({ updateState: { ...updateState, kind: 'cancelling' } });
    controller.abort();
  },
  installUpdate: async () => {
    const state = get().updateState;
    if (state.kind !== 'downloaded') return;
    // 平台解析前同步占位；对象身份保护等待期间被替换的安装任务。
    const installing = { ...state, kind: 'installing' as const };
    set({ updateState: installing });
    const current = () => get().updateState === installing;
    try {
      const capabilities = await getPlatformCapabilities();
      if (!current()) return;
      const blocked = blockedUpdateState(capabilities);
      if (blocked) {
        applyBlockedUpdateState(get(), set, blocked);
        return;
      }
      set({ unsupportedReason: null });
      if (capabilities.updateMethod === 'android_apk') {
        await androidInstallApk(state.version);
        if (current()) set({ updateState: state });
      } else {
        const downloaded = get().downloaded;
        if (!downloaded) throw new Error('No downloaded update available');
        await downloaded.install();
        if (current()) await relaunch();
      }
    } catch (error) {
      if (!current()) return;
      if (isUnsupportedApkUpdateError(error)) {
        applyBlockedUpdateState(get(), set, routeUnavailableState());
        return;
      }
      // 已验证安装包保留；权限引导或安装器失败后可以重试安装。
      const failed = { ...state, error: String(error) };
      set({ updateState: failed });
      logger.warn('[updater] install:', error);
      const { useUiStore } = await import('@/stores/uiStore');
      if (get().updateState !== failed) return;
      useUiStore.getState().showToast({
        type: 'error',
        message: String(error).includes('NEED_INSTALL_UNKNOWN_APPS_PERMISSION')
          ? i18n.t('settings:need_install_unknown_apps')
          : String(error),
      });
    }
  },

  dismissUpdate: () => {
    const { updateState, controller } = get();
    if (controller || updateState.kind === 'installing') return;
    if (updateState.kind !== 'hidden')
      void updateState.update?.close().catch((error) => logger.warn('[updater] close:', error));
    void get()
      .downloaded?.close()
      .catch((error) => logger.warn('[updater] close:', error));
    set({
      updateState: { kind: 'hidden' },
      downloaded: null,
      checkToken: null,
      checkPromise: null,
      checking: false,
    });
  },
}));
