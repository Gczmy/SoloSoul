import { describe, it, expect, vi, beforeEach } from 'vitest';

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));

// 底层依赖 mock：ipcClient.invokeCommand + tauri event.listen，
// ensureApkDownloaded 经 androidIsApkDownloaded/androidDownloadApk 走真实链路。
const ipcMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() => vi.fn());

vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: ipcMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

vi.mock('@/lib/logger', () => ({
  logger: { warn: vi.fn(), error: vi.fn() },
}));

import { ensureApkDownloaded } from './updater';
import type { ApkDownloadProgress } from './updater';

beforeEach(() => {
  vi.clearAllMocks();
});

/** 模拟 listen 注册后由外部触发一次进度事件（androidDownloadApk 取 event.payload）。 */
function simulateProgress(
  listener: (event: { payload: ApkDownloadProgress }) => void,
  payload: ApkDownloadProgress,
) {
  listener({ payload });
}

describe('ensureApkDownloaded', () => {
  it('APK 已下载时短路返回 false，不启动下载', async () => {
    ipcMock.mockResolvedValue(true); // android_is_apk_downloaded -> true
    const result = await ensureApkDownloaded('1.0.0', 'https://example.com/a.apk', '');
    expect(result).toBe(false);
    expect(listenMock).not.toHaveBeenCalled();
  });

  it('下载成功：返回 true，进度回调收到各阶段事件', async () => {
    ipcMock.mockResolvedValue(false); // 未下载
    // android_download_apk 触发（invoke<void>）；listen 注册回调
    let registered: ((event: { payload: ApkDownloadProgress }) => void) | undefined;
    listenMock.mockImplementation(
      (_evt: string, cb: (event: { payload: ApkDownloadProgress }) => void) => {
        registered = cb;
        return Promise.resolve(() => {});
      },
    );

    const seen: ApkDownloadProgress[] = [];
    const p = ensureApkDownloaded('1.0.0', 'https://example.com/a.apk', 'abc123', (prog) => {
      seen.push(prog);
    });

    // 等 listen 注册完成（androidDownloadApk 先 await listen 再返回）
    await flushMicrotasks();
    simulateProgress(registered!, { progress: 50, downloaded: 5, total: 10, done: false, error: null });
    simulateProgress(registered!, { progress: 100, downloaded: 10, total: 10, done: true, error: null });

    const result = await p;
    expect(result).toBe(true);
    expect(seen.map((s) => s.progress)).toEqual([50, 100]);
  });

  it('下载失败（done + error）时 reject，并清理事件监听', async () => {
    ipcMock.mockResolvedValue(false);
    let registered: ((event: { payload: ApkDownloadProgress }) => void) | undefined;
    const unlisten = vi.fn();
    listenMock.mockImplementation(
      (_evt: string, cb: (event: { payload: ApkDownloadProgress }) => void) => {
        registered = cb;
        return Promise.resolve(unlisten);
      },
    );

    const p = ensureApkDownloaded('1.0.0', 'https://example.com/a.apk', '');
    await flushMicrotasks();
    simulateProgress(registered!, { progress: 10, downloaded: 1, total: 10, done: true, error: 'disk full' });

    await expect(p).rejects.toThrow('disk full');
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('未下载但无 downloadUrl 时由调用方处理（本函数不校验 URL，正常走下载）', async () => {
    ipcMock.mockResolvedValue(false);
    let registered: ((event: { payload: ApkDownloadProgress }) => void) | undefined;
    listenMock.mockImplementation(
      (_evt: string, cb: (event: { payload: ApkDownloadProgress }) => void) => {
        registered = cb;
        return Promise.resolve(() => {});
      },
    );
    const p = ensureApkDownloaded('1.0.0', 'https://example.com/a.apk', '');
    await flushMicrotasks();
    simulateProgress(registered!, { progress: 100, downloaded: 1, total: 1, done: true, error: null });
    await expect(p).resolves.toBe(true);
  });
});
