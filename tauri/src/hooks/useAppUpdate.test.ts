import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { useAppUpdate } from './useAppUpdate';
import { useAuthStore } from '@/stores/authStore';
import type { ApkDownloadProgress, UpdateProgress } from '@/lib/updater';

const mocks = vi.hoisted(() => ({
  mobile: true,
  androidCheck: vi.fn(),
  desktopCheck: vi.fn(),
  androidDownload: vi.fn(),
  desktopDownload: vi.fn(),
  androidInstall: vi.fn(),
  relaunch: vi.fn(),
}));
vi.mock('@/lib/platform', () => ({ isMobilePlatformSync: () => mocks.mobile }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: mocks.relaunch }));
vi.mock('@/lib/updater', () => ({
  androidCheckForUpdate: mocks.androidCheck,
  checkForUpdate: mocks.desktopCheck,
  ensureApkDownloaded: mocks.androidDownload,
  downloadDesktopUpdate: mocks.desktopDownload,
  androidInstallApk: mocks.androidInstall,
  isUpdateDownloadCancelled: (error: unknown) =>
    error instanceof Error && error.name === 'AbortError',
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}
const androidInfo = {
  latestVersion: '2.13.1',
  currentVersion: '2.13.0',
  downloadUrl: 'https://example.com/app.apk',
  checksum: '',
  checksumWarning: null,
  mandatory: false,
  releaseNotes: 'notes',
  publishedAt: null,
  apkSize: 100,
};
function available(mobile = true, mandatory = false) {
  mocks.mobile = mobile;
  const update = { rid: 12, close: vi.fn().mockResolvedValue(undefined) };
  mocks.androidCheck.mockResolvedValue({ kind: 'available', info: { ...androidInfo, mandatory } });
  mocks.desktopCheck.mockResolvedValue({
    kind: 'available',
    update,
    info: { version: '2.13.1', body: 'notes' },
  });
  return update;
}

beforeEach(() => {
  vi.resetAllMocks();
  mocks.mobile = true;
  localStorage.clear();
  useAuthStore.setState({ isAuthenticated: false });
  mocks.androidCheck.mockResolvedValue({ kind: 'up-to-date' });
  mocks.desktopCheck.mockResolvedValue({ kind: 'up-to-date' });
});

describe('useAppUpdate', () => {
  it('未解锁时检查更新', async () => {
    renderHook(() => useAppUpdate());
    await waitFor(() => expect(mocks.androidCheck).toHaveBeenCalledTimes(1));
  });

  it('解锁后重查，显示可用更新', async () => {
    available();
    const { result } = renderHook(() => useAppUpdate());
    await waitFor(() => expect(result.current.updateState.kind).toBe('available'));
    act(() => useAuthStore.setState({ isAuthenticated: true }));
    await waitFor(() => expect(mocks.androidCheck).toHaveBeenCalledTimes(2));
    expect(result.current.updateState.kind).toBe('available');
  });

  it.each([true, false])(
    '取消等待原生清理完成，保留更新选项且不安装（mobile=%s）',
    async (mobile) => {
      available(mobile);
      const pending = deferred<never>();
      const download = mobile ? mocks.androidDownload : mocks.desktopDownload;
      download.mockReturnValue(pending.promise);
      const { result } = renderHook(() => useAppUpdate());
      await waitFor(() => expect(result.current.updateState.kind).toBe('available'));
      let task!: Promise<void>;
      act(() => {
        task = result.current.startDownload();
      });
      await waitFor(() => expect(download).toHaveBeenCalledTimes(1));
      const signal = download.mock.calls[0][2] as AbortSignal;
      act(() => result.current.cancelDownload());
      expect(signal.aborted).toBe(true);
      expect(result.current.updateState.kind).toBe('cancelling');
      act(() => {
        void result.current.startDownload();
        result.current.dismissUpdate();
      });
      expect(download).toHaveBeenCalledTimes(1);
      expect(result.current.updateState.kind).toBe('cancelling');
      await act(async () => {
        pending.reject(new DOMException('Cancelled', 'AbortError'));
        await task;
      });
      expect(result.current.updateState).toMatchObject({
        kind: 'available',
        version: '2.13.1',
        downloadedBytes: 0,
        totalBytes: 0,
        error: undefined,
      });
      expect(mocks.androidInstall).not.toHaveBeenCalled();
      expect(mocks.relaunch).not.toHaveBeenCalled();
    },
  );

  it('重试使用新signal；旧下载迟到进度不影响新下载，强更仍保留', async () => {
    available(true, true);
    const first = deferred<boolean>();
    const second = deferred<boolean>();
    mocks.androidDownload.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const { result } = renderHook(() => useAppUpdate());
    await waitFor(() => expect(result.current.updateState.kind).toBe('available'));
    let task!: Promise<void>;
    act(() => {
      task = result.current.startDownload();
      void result.current.startDownload();
    });
    await waitFor(() => expect(mocks.androidDownload).toHaveBeenCalledTimes(1));
    const oldProgress = mocks.androidDownload.mock.calls[0][1] as (p: ApkDownloadProgress) => void;
    act(() => result.current.cancelDownload());
    await act(async () => {
      first.resolve(true);
      await task;
    });
    expect(result.current.updateState).toMatchObject({ kind: 'available', mandatory: true });
    act(() => {
      task = result.current.startDownload();
    });
    await waitFor(() => expect(mocks.androidDownload).toHaveBeenCalledTimes(2));
    expect(mocks.androidDownload.mock.calls[1][2]).not.toBe(mocks.androidDownload.mock.calls[0][2]);
    act(() => oldProgress({ downloaded: 90, total: 100, progress: 90, done: true, error: null }));
    expect(result.current.updateState).toMatchObject({ kind: 'downloading', downloadedBytes: 0 });
    await act(async () => {
      second.resolve(true);
      await task;
    });
    expect(result.current.updateState).toMatchObject({ kind: 'downloaded', mandatory: true });
  });

  it('桌面 Transfer 使用绝对进度，换源不重复累计，取消重试恢复缓存进度', async () => {
    available(false);
    const first = deferred<never>();
    const second = deferred<{
      close: ReturnType<typeof vi.fn>;
      install: ReturnType<typeof vi.fn>;
    }>();
    mocks.desktopDownload.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const { result } = renderHook(() => useAppUpdate());
    await waitFor(() => expect(result.current.updateState.kind).toBe('available'));
    let task!: Promise<void>;
    act(() => {
      task = result.current.startDownload();
    });
    expect(result.current.updateState).toMatchObject({ transfer: { phase: 'probing' } });
    await waitFor(() => expect(mocks.desktopDownload).toHaveBeenCalledOnce());
    const progress = mocks.desktopDownload.mock.calls[0][1] as (p: UpdateProgress) => void;
    act(() => {
      progress({ event: 'Started', data: { contentLength: 100 } });
      progress({
        event: 'Transfer',
        data: {
          downloaded: 40,
          total: 100,
          source: 'github.com',
          bytesPerSecond: 2048,
          phase: 'downloading',
        },
      });
      progress({ event: 'Progress', data: { chunkLength: 40 } });
    });
    expect(result.current.updateState).toMatchObject({
      downloadedBytes: 40,
      progressPercent: 40,
      transfer: { source: 'github.com', bytesPerSecond: 2048 },
    });
    act(() => {
      progress({
        event: 'Transfer',
        data: {
          downloaded: 50,
          total: 100,
          source: 'mirror.example',
          bytesPerSecond: 0,
          phase: 'switching',
        },
      });
    });
    expect(result.current.updateState).toMatchObject({
      downloadedBytes: 50,
      transfer: { phase: 'switching' },
    });
    act(() => result.current.cancelDownload());
    await act(async () => {
      first.reject(new DOMException('Cancelled', 'AbortError'));
      await task;
    });
    expect(result.current.updateState).toMatchObject({ kind: 'available', transfer: undefined });
    act(() => {
      task = result.current.startDownload();
    });
    await waitFor(() => expect(mocks.desktopDownload).toHaveBeenCalledTimes(2));
    const resumed = mocks.desktopDownload.mock.calls[1][1] as (p: UpdateProgress) => void;
    act(() => {
      resumed({
        event: 'Transfer',
        data: {
          downloaded: 50,
          total: 100,
          source: 'mirror.example',
          bytesPerSecond: 0,
          phase: 'probing',
        },
      });
      progress({
        event: 'Transfer',
        data: {
          downloaded: 99,
          total: 100,
          source: 'old.example',
          bytesPerSecond: 999,
          phase: 'downloading',
        },
      });
    });
    expect(result.current.updateState).toMatchObject({
      downloadedBytes: 50,
      transfer: { source: 'mirror.example' },
    });
    await act(async () => {
      resumed({
        event: 'Transfer',
        data: {
          downloaded: 100,
          total: 100,
          source: 'mirror.example',
          bytesPerSecond: 4096,
          phase: 'downloading',
        },
      });
      second.resolve({ close: vi.fn().mockResolvedValue(undefined), install: vi.fn() });
      await task;
    });
    expect(result.current.updateState).toMatchObject({ kind: 'downloaded', downloadedBytes: 100 });
  });

  it('取消赢过桌面下载完成时释放产物，回到选项', async () => {
    available(false);
    const pending = deferred<{
      close: ReturnType<typeof vi.fn>;
      install: ReturnType<typeof vi.fn>;
    }>();
    const downloaded = { close: vi.fn().mockResolvedValue(undefined), install: vi.fn() };
    mocks.desktopDownload.mockReturnValue(pending.promise);
    const { result } = renderHook(() => useAppUpdate());
    await waitFor(() => expect(result.current.updateState.kind).toBe('available'));
    let task!: Promise<void>;
    act(() => {
      task = result.current.startDownload();
    });
    await waitFor(() => expect(mocks.desktopDownload).toHaveBeenCalledOnce());
    act(() => result.current.cancelDownload());
    await act(async () => {
      pending.resolve(downloaded);
      await task;
    });
    expect(downloaded.close).toHaveBeenCalledOnce();
    expect(downloaded.install).not.toHaveBeenCalled();
    expect(result.current.updateState.kind).toBe('available');
  });

  it('桌面下载完成后只有显式安装才执行安装，安装期间禁止重复操作', async () => {
    available(false);
    const install = deferred<void>();
    const downloaded = {
      close: vi.fn().mockResolvedValue(undefined),
      install: vi.fn().mockReturnValue(install.promise),
    };
    mocks.desktopDownload.mockImplementation(
      async (_update, progress: (event: UpdateProgress) => void) => {
        progress({ event: 'Started', data: { contentLength: 100 } });
        progress({ event: 'Progress', data: { chunkLength: 100 } });
        return downloaded;
      },
    );
    const { result } = renderHook(() => useAppUpdate());
    await waitFor(() => expect(result.current.updateState.kind).toBe('available'));
    await act(async () => {
      await result.current.startDownload();
    });
    expect(result.current.updateState).toMatchObject({ kind: 'downloaded', downloadedBytes: 100 });
    expect(downloaded.install).not.toHaveBeenCalled();
    let task!: Promise<void>;
    act(() => {
      task = result.current.installUpdate();
      void result.current.installUpdate();
    });
    expect(result.current.updateState.kind).toBe('installing');
    act(() => {
      result.current.cancelDownload();
      result.current.dismissUpdate();
    });
    expect(result.current.updateState.kind).toBe('installing');
    expect(downloaded.install).toHaveBeenCalledOnce();
    await act(async () => {
      install.resolve();
      await task;
    });
    expect(mocks.relaunch).toHaveBeenCalledOnce();
  });

  it('解锁重查的迟到结果不能覆盖下载状态', async () => {
    available();
    const check = deferred<unknown>();
    const download = deferred<boolean>();
    mocks.androidCheck
      .mockResolvedValueOnce({ kind: 'available', info: androidInfo })
      .mockReturnValueOnce(check.promise);
    mocks.androidDownload.mockReturnValue(download.promise);
    const { result } = renderHook(() => useAppUpdate());
    await waitFor(() => expect(result.current.updateState.kind).toBe('available'));
    act(() => useAuthStore.setState({ isAuthenticated: true }));
    await waitFor(() => expect(mocks.androidCheck).toHaveBeenCalledTimes(2));
    let task!: Promise<void>;
    act(() => {
      task = result.current.startDownload();
    });
    await act(async () => {
      check.resolve({ kind: 'available', info: { ...androidInfo, latestVersion: '9.0.0' } });
    });
    expect(result.current.updateState).toMatchObject({ kind: 'downloading', version: '2.13.1' });
    await act(async () => {
      download.resolve(true);
      await task;
    });
  });

  it('卸载中止正在下载的任务并忽略其完成', async () => {
    available();
    const pending = deferred<boolean>();
    mocks.androidDownload.mockReturnValue(pending.promise);
    const { result, unmount } = renderHook(() => useAppUpdate());
    await waitFor(() => expect(result.current.updateState.kind).toBe('available'));
    let task!: Promise<void>;
    act(() => {
      task = result.current.startDownload();
    });
    await waitFor(() => expect(mocks.androidDownload).toHaveBeenCalledOnce());
    const signal = mocks.androidDownload.mock.calls[0][2] as AbortSignal;
    unmount();
    expect(signal.aborted).toBe(true);
    await act(async () => {
      pending.resolve(true);
      await task;
    });
    expect(mocks.androidInstall).not.toHaveBeenCalled();
  });
});
