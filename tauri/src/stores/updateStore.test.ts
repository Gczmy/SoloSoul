import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Platform } from '@tauri-apps/plugin-os';
import type { Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { useUpdateStore, type AppUpdateState } from './updateStore';
import { getPlatform } from '@/lib/platform';
import {
  androidCachedUpdate,
  androidCheckForUpdate,
  androidInstallApk,
  checkForUpdate,
  downloadDesktopUpdate,
  ensureApkDownloaded,
  type DownloadedDesktopUpdate,
} from '@/lib/updater';

vi.mock('@/lib/platform', () => ({ getPlatform: vi.fn() }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));
vi.mock('@/lib/updater', () => ({
  checkForUpdate: vi.fn(),
  downloadDesktopUpdate: vi.fn(),
  androidInstallApk: vi.fn(),
  androidCachedUpdate: vi.fn(),
  androidCheckForUpdate: vi.fn(),
  ensureApkDownloaded: vi.fn(),
  isUpdateDownloadCancelled: (e: Error) => e.name === 'AbortError',
}));

const info = {
  currentVersion: '2.13.1',
  latestVersion: '2.13.2',
  downloadUrl: 'https://example.com/app.apk',
  checksum: 'abc',
  checksumWarning: null,
  mandatory: false,
  releaseNotes: null,
  publishedAt: null,
  apkSize: 108,
  cachedDownload: { downloaded: 40, total: 108, done: false },
};

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function desktopResources() {
  const update = {
    rid: 17,
    version: info.latestVersion,
    body: 'Desktop release',
    close: vi.fn().mockResolvedValue(undefined),
  } as unknown as Update;
  const install = vi.fn().mockResolvedValue(undefined);
  const close = vi.fn().mockResolvedValue(undefined);
  const downloaded = { rid: 91, install, close } as unknown as DownloadedDesktopUpdate;
  return { update, downloaded, install, close };
}

function candidate(kind: 'available' | 'downloaded', update: Update | null = null): AppUpdateState {
  return {
    kind,
    update,
    androidInfo: info,
    version: info.latestVersion,
    releaseNotes: null,
    downloadedBytes: kind === 'downloaded' ? 108 : 40,
    totalBytes: 108,
    progressPercent: kind === 'downloaded' ? 100 : 37,
    mandatory: false,
  };
}

function expectNoUpdateWork() {
  for (const operation of [
    androidCachedUpdate,
    androidCheckForUpdate,
    androidInstallApk,
    ensureApkDownloaded,
    checkForUpdate,
    downloadDesktopUpdate,
    relaunch,
  ]) {
    expect(operation).not.toHaveBeenCalled();
  }
}

beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  useUpdateStore.setState(useUpdateStore.getInitialState(), true);
  vi.mocked(getPlatform).mockResolvedValue('android');
  vi.mocked(androidCachedUpdate).mockResolvedValue(info);
  vi.mocked(androidCheckForUpdate).mockResolvedValue({ kind: 'available', info });
  vi.mocked(ensureApkDownloaded).mockResolvedValue(true);
  vi.mocked(androidInstallApk).mockResolvedValue(undefined);
  vi.mocked(relaunch).mockResolvedValue(undefined);
});

describe('global update cache and progress', () => {
  it('restores actual persisted bytes immediately while network metadata is still pending', async () => {
    const network = deferred<{ kind: 'error'; message: string }>();
    vi.mocked(androidCheckForUpdate).mockReturnValue(network.promise);
    const first = useUpdateStore.getState().check();
    await vi.waitFor(() => expect(androidCheckForUpdate).toHaveBeenCalledOnce());
    expect(useUpdateStore.getState().updateState).toMatchObject({
      kind: 'available',
      downloadedBytes: 40,
      totalBytes: 108,
    });
    const second = useUpdateStore.getState().check(true);
    expect(second).toBe(first);
    network.resolve({ kind: 'error', message: 'offline' });
    await first;
    expect(useUpdateStore.getState().updateState).toMatchObject({
      kind: 'available',
      downloadedBytes: 40,
    });
    expect(useUpdateStore.getState().checkError).toBe('offline');
  });

  it('uses absolute bytes across retry/source switches, caps invalid excess, and retains progress until resumed', async () => {
    await useUpdateStore.getState().check();
    const download = deferred<boolean>();
    vi.mocked(ensureApkDownloaded).mockReturnValue(download.promise);
    const task = useUpdateStore.getState().startDownload();
    expect(useUpdateStore.getState().updateState).toMatchObject({ downloadedBytes: 40 });
    await vi.waitFor(() => expect(ensureApkDownloaded).toHaveBeenCalledOnce());
    const emit = vi.mocked(ensureApkDownloaded).mock.calls[0][1]!;
    for (const downloaded of [70, 40, 75, 140]) {
      emit({ downloaded, total: 108, progress: 999, done: false, error: null });
      expect(useUpdateStore.getState().updateState).toMatchObject({
        downloadedBytes: Math.min(downloaded, 108),
      });
    }
    expect(useUpdateStore.getState().updateState).toMatchObject({ progressPercent: 99 });
    download.resolve(true);
    await task;
    expect(useUpdateStore.getState().updateState).toMatchObject({
      kind: 'downloaded',
      progressPercent: 100,
    });
  });
});

describe('RF202 platform update boundaries', () => {
  it('rf202_initial_platform_is_unresolved_not_unsupported', () => {
    expect(useUpdateStore.getState().unsupportedReason).toBeNull();
    expect(useUpdateStore.getState().lastChecked).toBe(0);
    expectNoUpdateWork();
  });

  it.each(['cached-check', 'version-check', 'download', 'install'] as const)(
    'rf202_ios_%s_hides_updates_without_apk_or_desktop_work',
    async (stage) => {
      vi.mocked(getPlatform).mockResolvedValue('ios');
      const resources = desktopResources();
      useUpdateStore.setState({
        lastChecked: 17,
        checkError: 'old network error',
        updateState:
          stage === 'cached-check'
            ? { kind: 'hidden' }
            : candidate(stage === 'install' ? 'downloaded' : 'available', resources.update),
        downloaded: stage === 'install' ? resources.downloaded : null,
      });
      if (stage === 'download') await useUpdateStore.getState().startDownload();
      else if (stage === 'install') await useUpdateStore.getState().installUpdate();
      else await useUpdateStore.getState().check(true);
      expectNoUpdateWork();
      expect(resources.install).not.toHaveBeenCalled();
      expect(useUpdateStore.getState()).toMatchObject({
        unsupportedReason: 'ios',
        updateState: { kind: 'hidden' },
        checking: false,
        lastChecked: 17,
        controller: null,
        checkPromise: null,
        downloaded: null,
      });
      expect(useUpdateStore.getState().checkError).toBeUndefined();
    },
  );

  it.each(['android', 'ios', 'macos', 'windows'] as const)(
    'rf202_%s_check_reentry_shares_promise_while_platform_is_pending',
    async (platform) => {
      const detection = deferred<Platform>();
      vi.mocked(getPlatform).mockReturnValue(detection.promise);
      vi.mocked(androidCachedUpdate).mockResolvedValue(null);
      vi.mocked(androidCheckForUpdate).mockResolvedValue({ kind: 'up-to-date' });
      vi.mocked(checkForUpdate).mockResolvedValue({ kind: 'up-to-date' });
      const first = useUpdateStore.getState().check();
      const second = useUpdateStore.getState().check(true);
      expect(second).toBe(first);
      expect(getPlatform).toHaveBeenCalledOnce();
      expectNoUpdateWork();
      expect(useUpdateStore.getState().checking).toBe(true);
      detection.resolve(platform);
      await first;
      expect(useUpdateStore.getState()).toMatchObject({ checking: false, checkPromise: null });
      expect(useUpdateStore.getState().checkError).toBeUndefined();
      if (platform === 'ios') {
        expectNoUpdateWork();
        expect(useUpdateStore.getState()).toMatchObject({
          unsupportedReason: 'ios',
          lastChecked: 0,
        });
      } else {
        expect(useUpdateStore.getState().lastChecked).toBeGreaterThan(0);
        expect(useUpdateStore.getState().unsupportedReason).toBeNull();
        expect(androidCachedUpdate).toHaveBeenCalledTimes(platform === 'android' ? 1 : 0);
        expect(androidCheckForUpdate).toHaveBeenCalledTimes(platform === 'android' ? 1 : 0);
        expect(checkForUpdate).toHaveBeenCalledTimes(platform === 'android' ? 0 : 1);
      }
    },
  );

  it.each(['android', 'macos', 'windows'] as const)(
    'rf202_%s_download_and_install_preserve_platform_route_and_reserve_before_await',
    async (platform) => {
      vi.mocked(getPlatform).mockResolvedValue(platform);
      const resources = desktopResources();
      vi.mocked(checkForUpdate).mockResolvedValue({
        kind: 'available',
        info: { version: info.latestVersion, body: 'Desktop release' },
        update: resources.update,
      });
      await useUpdateStore.getState().check(true);
      expect(useUpdateStore.getState().updateState).toMatchObject({
        kind: 'available',
        version: info.latestVersion,
      });
      expect(androidCachedUpdate).toHaveBeenCalledTimes(platform === 'android' ? 1 : 0);
      expect(androidCheckForUpdate).toHaveBeenCalledTimes(platform === 'android' ? 1 : 0);
      expect(checkForUpdate).toHaveBeenCalledTimes(platform === 'android' ? 0 : 1);

      const detection = deferred<Platform>();
      vi.mocked(getPlatform).mockReturnValueOnce(detection.promise);
      const apkDownload = deferred<boolean>();
      const desktopDownload = deferred<DownloadedDesktopUpdate>();
      vi.mocked(ensureApkDownloaded).mockReturnValue(apkDownload.promise);
      vi.mocked(downloadDesktopUpdate).mockReturnValue(desktopDownload.promise);
      const firstDownload = useUpdateStore.getState().startDownload();
      const secondDownload = useUpdateStore.getState().startDownload();
      expect(getPlatform).toHaveBeenCalledTimes(2);
      expect(useUpdateStore.getState().controller).not.toBeNull();
      expect(ensureApkDownloaded).not.toHaveBeenCalled();
      expect(downloadDesktopUpdate).not.toHaveBeenCalled();
      detection.resolve(platform);
      const downloadOperation =
        platform === 'android' ? ensureApkDownloaded : downloadDesktopUpdate;
      await vi.waitFor(() => expect(downloadOperation).toHaveBeenCalledOnce());
      await useUpdateStore.getState().startDownload();
      expect(downloadOperation).toHaveBeenCalledOnce();
      if (platform === 'android') {
        expect(downloadDesktopUpdate).not.toHaveBeenCalled();
        expect(ensureApkDownloaded).toHaveBeenCalledWith(
          info.latestVersion,
          expect.any(Function),
          expect.any(AbortSignal),
        );
        apkDownload.resolve(true);
      } else {
        expect(ensureApkDownloaded).not.toHaveBeenCalled();
        expect(downloadDesktopUpdate).toHaveBeenCalledWith(
          resources.update,
          expect.any(Function),
          expect.any(AbortSignal),
        );
        desktopDownload.resolve(resources.downloaded);
      }
      await Promise.all([firstDownload, secondDownload]);
      expect(useUpdateStore.getState()).toMatchObject({
        updateState: { kind: 'downloaded', progressPercent: 100 },
        controller: null,
      });

      const installDetection = deferred<Platform>();
      vi.mocked(getPlatform).mockReturnValueOnce(installDetection.promise);
      const installation = deferred<void>();
      vi.mocked(androidInstallApk).mockReturnValue(installation.promise);
      resources.install.mockReturnValue(installation.promise);
      const firstInstall = useUpdateStore.getState().installUpdate();
      const secondInstall = useUpdateStore.getState().installUpdate();
      expect(getPlatform).toHaveBeenCalledTimes(3);
      expect(useUpdateStore.getState().updateState.kind).toBe('installing');
      expect(androidInstallApk).not.toHaveBeenCalled();
      expect(resources.install).not.toHaveBeenCalled();
      installDetection.resolve(platform);
      const installOperation = platform === 'android' ? androidInstallApk : resources.install;
      await vi.waitFor(() => expect(installOperation).toHaveBeenCalledOnce());
      await useUpdateStore.getState().installUpdate();
      expect(installOperation).toHaveBeenCalledOnce();
      expect(relaunch).not.toHaveBeenCalled();
      installation.resolve(undefined);
      await Promise.all([firstInstall, secondInstall]);
      if (platform === 'android') {
        expect(androidInstallApk).toHaveBeenCalledExactlyOnceWith(info.latestVersion);
        expect(resources.install).not.toHaveBeenCalled();
        expect(relaunch).not.toHaveBeenCalled();
        expect(useUpdateStore.getState().updateState.kind).toBe('downloaded');
      } else {
        expect(androidInstallApk).not.toHaveBeenCalled();
        expect(relaunch).toHaveBeenCalledOnce();
        expect(useUpdateStore.getState().downloaded).toBe(resources.downloaded);
      }
      expect(resources.close).not.toHaveBeenCalled();
    },
  );

  it.each(['android', 'macos', 'windows'] as const)(
    'rf202_%s_cancel_during_platform_detection_does_not_start_download',
    async (platform) => {
      const detection = deferred<Platform>();
      vi.mocked(getPlatform).mockReturnValue(detection.promise);
      useUpdateStore.setState({ updateState: candidate('available', desktopResources().update) });
      const task = useUpdateStore.getState().startDownload();
      useUpdateStore.getState().cancelDownload();
      expect(useUpdateStore.getState().updateState.kind).toBe('cancelling');
      detection.resolve(platform);
      await task;
      expectNoUpdateWork();
      expect(useUpdateStore.getState()).toMatchObject({
        controller: null,
        updateState: { kind: 'available' },
      });
      const state = useUpdateStore.getState().updateState;
      expect(state.kind !== 'hidden' && state.error).toBeUndefined();
    },
  );

  it('rf202_dismiss_during_platform_detection_invalidates_check_without_any_update_work', async () => {
    const detection = deferred<Platform>();
    vi.mocked(getPlatform).mockReturnValue(detection.promise);
    const task = useUpdateStore.getState().check();
    useUpdateStore.getState().dismissUpdate();
    detection.resolve('android');
    await task;
    expectNoUpdateWork();
    expect(useUpdateStore.getState()).toMatchObject({
      updateState: { kind: 'hidden' },
      lastChecked: 0,
      checkPromise: null,
      checking: false,
    });
  });
});
