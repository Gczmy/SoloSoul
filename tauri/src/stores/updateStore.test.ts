import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useUpdateStore } from './updateStore';
import { androidCachedUpdate, androidCheckForUpdate, ensureApkDownloaded } from '@/lib/updater';

vi.mock('@/lib/platform', () => ({ isMobilePlatformSync: () => true }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));
vi.mock('@/lib/updater', () => ({
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
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  useUpdateStore.setState(useUpdateStore.getInitialState(), true);
  vi.mocked(androidCachedUpdate).mockResolvedValue(info);
});
describe('global update cache and progress', () => {
  it('restores actual persisted bytes immediately while network metadata is still pending', async () => {
    let finish!: (v: { kind: 'error'; message: string }) => void;
    vi.mocked(androidCheckForUpdate).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    const first = useUpdateStore.getState().check();
    await Promise.resolve();
    expect(useUpdateStore.getState().updateState).toMatchObject({
      kind: 'available',
      downloadedBytes: 40,
      totalBytes: 108,
    });
    const second = useUpdateStore.getState().check(true);
    expect(second).toBe(first);
    finish({ kind: 'error', message: 'offline' });
    await first;
    expect(useUpdateStore.getState().updateState).toMatchObject({
      kind: 'available',
      downloadedBytes: 40,
    });
    expect(useUpdateStore.getState().checkError).toBe('offline');
  });
  it('uses absolute bytes across retry/source switches, caps invalid excess, and retains progress until resumed', async () => {
    vi.mocked(androidCheckForUpdate).mockResolvedValue({ kind: 'available', info });
    await useUpdateStore.getState().check();
    let finish!: (v: boolean) => void;
    vi.mocked(ensureApkDownloaded).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    const task = useUpdateStore.getState().startDownload();
    expect(useUpdateStore.getState().updateState).toMatchObject({ downloadedBytes: 40 });
    const emit = vi.mocked(ensureApkDownloaded).mock.calls[0][1]!;
    for (const downloaded of [70, 40, 75, 140]) {
      emit({ downloaded, total: 108, progress: 999, done: false, error: null });
      expect(useUpdateStore.getState().updateState).toMatchObject({
        downloadedBytes: Math.min(downloaded, 108),
      });
    }
    expect(useUpdateStore.getState().updateState).toMatchObject({ progressPercent: 99 });
    finish(true);
    await task;
    expect(useUpdateStore.getState().updateState).toMatchObject({
      kind: 'downloaded',
      progressPercent: 100,
    });
  });
});
