import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useUpdateChecker } from './useUpdateChecker';
import {
  androidCheckForUpdate,
  androidInstallApk,
  checkForUpdate,
  downloadDesktopUpdate,
  ensureApkDownloaded,
} from '@/lib/updater';
import { useUpdateStore } from '@/stores/updateStore';
import { useAppUpdate } from './useAppUpdate';
import { isMobilePlatformSync } from '@/lib/platform';

vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));
vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: vi.fn(async () => ({
    appName: 'SoloSoul',
    version: '2.13.0',
    os: 'android',
    arch: 'aarch64',
  })),
}));
vi.mock('@/lib/platform', () => ({ isMobilePlatformSync: vi.fn(() => true) }));
vi.mock('@/lib/updater', () => ({
  androidCheckForUpdate: vi.fn(),
  androidCachedUpdate: vi.fn().mockResolvedValue(null),
  checkForUpdate: vi.fn(),
  androidInstallApk: vi.fn(),
  ensureApkDownloaded: vi.fn(),
  downloadDesktopUpdate: vi.fn(),
  isUpdateDownloadCancelled: (error: unknown) =>
    error instanceof Error && error.name === 'AbortError',
}));

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

async function ready() {
  const hook = renderHook(useUpdateChecker);
  await waitFor(() => expect(hook.result.current.versionInfo?.state).toBe('available'));
  await waitFor(() => expect(hook.result.current.loading).toBe(false));
  return hook;
}

describe('useUpdateChecker cancellation', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    useUpdateStore.setState(useUpdateStore.getInitialState(), true);
    localStorage.clear();
    vi.mocked(isMobilePlatformSync).mockReturnValue(true);
    vi.mocked(androidCheckForUpdate).mockResolvedValue({
      kind: 'available',
      info: {
        currentVersion: '2.13.0',
        latestVersion: '2.13.1',
        downloadUrl: 'https://example.com/update.apk',
        checksum: 'a'.repeat(64),
        apkSize: 100,
        mandatory: true,
        releaseNotes: 'Security update',
        publishedAt: null,
        checksumWarning: null,
      },
    });
    vi.mocked(checkForUpdate).mockResolvedValue({
      kind: 'available',
      update: { version: '2.13.1', close: vi.fn().mockResolvedValue(undefined) } as never,
      info: { version: '2.13.1', body: 'notes' },
    });
    vi.mocked(androidInstallApk).mockResolvedValue(undefined);
  });

  it('waits for Android cancellation to settle, ignores late progress, and preserves mandatory update', async () => {
    const first = deferred<boolean>();
    const second = deferred<boolean>();
    vi.mocked(ensureApkDownloaded)
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const { result } = await ready();
    let firstRun!: Promise<void>;
    act(() => {
      firstRun = result.current.handleUpdate();
    });
    const [, firstProgress, firstSignal] = vi.mocked(ensureApkDownloaded).mock.calls[0];
    act(() =>
      firstProgress?.({
        downloaded: 20,
        total: 100,
        progress: 20,
        done: false,
        error: null,
        phase: 'switching',
        source: 'mirror.example',
        bytesPerSecond: 0,
      }),
    );
    expect(result.current.downloadedBytes).toBe(20);
    expect(result.current.transfer).toEqual({
      phase: 'switching',
      source: 'mirror.example',
      bytesPerSecond: 0,
    });
    act(() => result.current.cancelDownload());
    expect(firstSignal?.aborted).toBe(true);
    expect(result.current.cancelling).toBe(true);
    expect(result.current.downloading).toBe(true);
    act(() => {
      firstProgress?.({ downloaded: 100, total: 100, progress: 100, done: true, error: null });
      void result.current.handleUpdate();
    });
    expect(ensureApkDownloaded).toHaveBeenCalledTimes(1);
    expect(result.current.downloadedBytes).toBe(20);
    await act(async () => {
      // 即便下载在取消同时成功，已取消的操作也不得继续启动安装器。
      first.resolve(true);
      await firstRun;
    });
    expect(androidInstallApk).not.toHaveBeenCalled();
    expect(result.current.downloading).toBe(false);
    expect(result.current.transfer).toBeUndefined();
    expect(result.current.downloadError).toBeNull();
    expect(result.current.isMandatory).toBe(true);
    expect(result.current.versionInfo?.state).toBe('available');

    let secondRun!: Promise<void>;
    act(() => {
      secondRun = result.current.handleUpdate();
      firstProgress?.({ downloaded: 100, total: 100, progress: 100, done: true, error: null });
    });
    expect(result.current.downloadedBytes).toBe(20);
    expect(ensureApkDownloaded).toHaveBeenCalledTimes(2);
    await act(async () => {
      second.resolve(true);
      await secondRun;
    });
    expect(androidInstallApk).toHaveBeenCalledExactlyOnceWith('2.13.1');
  });

  it('keeps cancellation pending until the desktop download rejects and allows retry afterward', async () => {
    vi.mocked(isMobilePlatformSync).mockReturnValue(false);
    const download = deferred<Awaited<ReturnType<typeof downloadDesktopUpdate>>>();
    vi.mocked(downloadDesktopUpdate).mockReturnValue(download.promise);
    const { result } = await ready();
    let run!: Promise<void>;
    act(() => {
      run = result.current.handleUpdate();
    });
    const [, progress, signal] = vi.mocked(downloadDesktopUpdate).mock.calls[0];
    act(() => {
      progress?.({ event: 'Started', data: { contentLength: 100 } });
      progress?.({
        event: 'Transfer',
        data: {
          downloaded: 25,
          total: 100,
          source: 'mirror.example',
          bytesPerSecond: 2048,
          phase: 'downloading',
        },
      });
      progress?.({ event: 'Progress', data: { chunkLength: 25 } });
      result.current.cancelDownload();
      progress?.({ event: 'Progress', data: { chunkLength: 75 } });
    });
    expect(result.current.downloadedBytes).toBe(25);
    expect(result.current.transfer).toEqual({
      source: 'mirror.example',
      bytesPerSecond: 2048,
      phase: 'downloading',
    });
    expect(result.current.installing).toBe(false);
    expect(result.current.cancelling).toBe(true);
    expect(signal?.aborted).toBe(true);
    await act(async () => {
      download.reject(new DOMException('Cancelled', 'AbortError'));
      await run;
    });
    expect(result.current.downloading).toBe(false);
    expect(result.current.transfer).toBeUndefined();
    expect(result.current.downloadError).toBeNull();
  });

  it('does not cancel desktop installation once file replacement starts', async () => {
    vi.mocked(isMobilePlatformSync).mockReturnValue(false);
    const install = deferred<void>();
    vi.mocked(downloadDesktopUpdate).mockResolvedValue({
      install: vi.fn(() => install.promise),
      close: vi.fn(),
    } as never);
    const { result, unmount } = await ready();
    let run!: Promise<void>;
    await act(async () => {
      run = result.current.handleUpdate();
    });
    const [, , signal] = vi.mocked(downloadDesktopUpdate).mock.calls[0];
    act(() => result.current.cancelDownload());
    expect(result.current.installing).toBe(true);
    expect(result.current.cancelling).toBe(false);
    expect(signal?.aborted).toBe(false);
    unmount();
    install.resolve(undefined);
    await run;
  });

  it('does not cancel an Android installer that has already started', async () => {
    vi.mocked(ensureApkDownloaded).mockResolvedValue(false);
    const install = deferred<void>();
    vi.mocked(androidInstallApk).mockReturnValue(install.promise);
    const { result } = await ready();
    let run!: Promise<void>;
    await act(async () => {
      run = result.current.handleUpdate();
    });
    expect(result.current.installing).toBe(true);
    act(() => result.current.cancelDownload());
    expect(vi.mocked(ensureApkDownloaded).mock.calls[0][2]?.aborted).toBe(false);
    await act(async () => {
      install.resolve(undefined);
      await run;
    });
    expect(result.current.downloading).toBe(false);
  });

  it('continues a single shared task after leaving About and opening the banner', async () => {
    const download = deferred<boolean>();
    vi.mocked(ensureApkDownloaded).mockReturnValue(download.promise);
    const { result, unmount } = await ready();
    let run!: Promise<void>;
    act(() => {
      run = result.current.handleUpdate();
    });
    const [, , signal] = vi.mocked(ensureApkDownloaded).mock.calls[0];
    unmount();
    expect(signal?.aborted).toBe(false);
    const banner = renderHook(useAppUpdate);
    expect(banner.result.current.updateState.kind).toBe('downloading');
    await act(async () => banner.result.current.startDownload());
    expect(ensureApkDownloaded).toHaveBeenCalledOnce();
    await act(async () => {
      download.resolve(true);
      await run;
    });
    expect(androidInstallApk).toHaveBeenCalledOnce();
  });
});
