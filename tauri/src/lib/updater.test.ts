import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Update } from '@tauri-apps/plugin-updater';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  check: vi.fn(),
  relaunch: vi.fn(),
  close: vi.fn(),
  channels: [] as Array<{ onmessage: (message: unknown) => void }>,
}));

vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: mocks.check }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: mocks.relaunch }));
vi.mock('@tauri-apps/api/core', () => ({
  Channel: class {
    onmessage = (_message: unknown) => {};
    constructor() {
      mocks.channels.push(this);
    }
  },
  Resource: class {
    constructor(readonly rid: number) {}
    close(): Promise<void> {
      return mocks.close(this.rid);
    }
  },
}));
vi.mock('@/lib/logger', () => ({ logger: { warn: vi.fn(), error: vi.fn() } }));

import {
  downloadAndInstallUpdate,
  downloadDesktopUpdate,
  ensureApkDownloaded,
  isUpdateDownloadCancelled,
  type ApkDownloadProgress,
} from './updater';

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const flushMicrotasks = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

// 默认原生管线成功；各测试只替换需要控制的异步边界。
let native: {
  cache: () => Promise<boolean>;
  create: () => Promise<number>;
  cancel: () => Promise<void>;
  androidDownload: () => Promise<void>;
  desktopDownload: () => Promise<number>;
  install: () => Promise<void>;
};

beforeEach(() => {
  vi.resetAllMocks();
  mocks.channels.length = 0;
  mocks.close.mockResolvedValue(undefined);
  mocks.relaunch.mockResolvedValue(undefined);
  native = {
    cache: async () => false,
    create: async () => 41,
    cancel: async () => undefined,
    androidDownload: async () => undefined,
    desktopDownload: async () => 91,
    install: async () => undefined,
  };
  mocks.invoke.mockImplementation((command: string) => {
    switch (command) {
      case 'android_is_apk_downloaded':
        return native.cache();
      case 'create_update_download':
        return native.create();
      case 'cancel_update_download':
        return native.cancel();
      case 'android_download_apk':
        return native.androidDownload();
      case 'desktop_download_update':
        return native.desktopDownload();
      case 'desktop_install_update':
        return native.install();
      default:
        throw new Error(`Unexpected IPC command: ${command}`);
    }
  });
});

function desktopUpdate() {
  return {
    rid: 17,
    version: '2.13.0',
    close: vi.fn().mockResolvedValue(undefined),
  } as unknown as Update;
}

const progress: ApkDownloadProgress = {
  progress: 50,
  downloaded: 5,
  total: 10,
  done: false,
  error: null,
};

describe('ensureApkDownloaded', () => {
  it('APK 已缓存时返回 false，不创建下载操作或 Channel', async () => {
    native.cache = async () => true;
    await expect(ensureApkDownloaded('2.13.0')).resolves.toBe(false);
    expect(mocks.invoke.mock.calls).toEqual([['android_is_apk_downloaded', { version: '2.13.0' }]]);
    expect(mocks.channels).toHaveLength(0);
    expect(mocks.close).not.toHaveBeenCalled();
  });

  it('开始前已取消时不进行缓存查询或原生更新检查', async () => {
    const controller = new AbortController();
    controller.abort();
    await expect(ensureApkDownloaded('2.13.0', undefined, controller.signal)).rejects.toMatchObject(
      {
        name: 'AbortError',
      },
    );
    await expect(downloadAndInstallUpdate(undefined, controller.signal)).rejects.toMatchObject({
      name: 'AbortError',
    });
    await expect(
      downloadDesktopUpdate(desktopUpdate(), undefined, controller.signal),
    ).rejects.toMatchObject({ name: 'AbortError' });
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(mocks.check).not.toHaveBeenCalled();
  });

  it('缓存查询期间取消后不再创建下载操作', async () => {
    const cache = deferred<boolean>();
    native.cache = () => cache.promise;
    const controller = new AbortController();
    const result = ensureApkDownloaded('2.13.0', undefined, controller.signal);
    const cancelled = expect(result).rejects.toMatchObject({ name: 'AbortError' });
    controller.abort();
    cache.resolve(false);
    await cancelled;
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.channels).toHaveLength(0);
  });

  it('登记操作期间取消时，等到拿到操作 ID 后撤销并释放，绝不启动下载', async () => {
    const create = deferred<number>();
    native.create = () => create.promise;
    const controller = new AbortController();
    const result = ensureApkDownloaded('2.13.0', undefined, controller.signal);
    const cancelled = expect(result).rejects.toMatchObject({ name: 'AbortError' });
    await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith('create_update_download'));
    controller.abort();
    expect(mocks.invoke).not.toHaveBeenCalledWith('cancel_update_download', expect.anything());
    create.resolve(41);
    await cancelled;
    expect(mocks.invoke).toHaveBeenCalledWith('cancel_update_download', { operationId: 41 });
    expect(mocks.invoke).not.toHaveBeenCalledWith('android_download_apk', expect.anything());
    expect(mocks.close).toHaveBeenCalledExactlyOnceWith(41);
  });

  it('done 进度事件不能提前完成；原生命令成功后才返回并释放操作', async () => {
    const download = deferred<void>();
    native.androidDownload = () => download.promise;
    const onProgress = vi.fn();
    let settled = false;
    const result = ensureApkDownloaded('2.13.0', onProgress).finally(() => {
      settled = true;
    });
    await vi.waitFor(() => expect(mocks.channels).toHaveLength(1));
    mocks.channels[0].onmessage(progress);
    mocks.channels[0].onmessage({ ...progress, progress: 100, done: true });
    await flushMicrotasks();
    expect(settled).toBe(false);
    expect(mocks.close).not.toHaveBeenCalled();
    expect(onProgress.mock.calls.map(([event]) => event.progress)).toEqual([50, 100]);

    // 下载地址和校验和必须由原生重新读取、验签，前端只携带版本和操作资源。
    expect(mocks.invoke).toHaveBeenCalledWith('android_download_apk', {
      version: '2.13.0',
      operationId: 41,
      onEvent: mocks.channels[0],
    });
    download.resolve(undefined);
    await expect(result).resolves.toBe(true);
    expect(mocks.close).toHaveBeenCalledExactlyOnceWith(41);
  });

  it('done 事件后原生校验失败仍拒绝完成，并释放操作', async () => {
    const download = deferred<void>();
    native.androidDownload = () => download.promise;
    const result = ensureApkDownloaded('2.13.0');
    const failed = expect(result).rejects.toThrow('checksum mismatch');
    await vi.waitFor(() => expect(mocks.channels).toHaveLength(1));
    mocks.channels[0].onmessage({ ...progress, progress: 100, done: true });
    download.reject(new Error('checksum mismatch'));
    await failed;
    expect(mocks.close).toHaveBeenCalledExactlyOnceWith(41);
  });

  it('取消后等待原生下载退出，再等待取消命令完成；迟到进度不再通知 UI', async () => {
    const download = deferred<void>();
    const cancellation = deferred<void>();
    native.androidDownload = () => download.promise;
    native.cancel = () => cancellation.promise;
    const controller = new AbortController();
    const onProgress = vi.fn();
    let settled = false;
    const result = ensureApkDownloaded('2.13.0', onProgress, controller.signal).finally(() => {
      settled = true;
    });
    const cancelled = expect(result).rejects.toMatchObject({ name: 'AbortError' });
    await vi.waitFor(() => expect(mocks.channels).toHaveLength(1));
    mocks.channels[0].onmessage(progress);
    controller.abort();
    mocks.channels[0].onmessage({ ...progress, progress: 100, done: true });
    await flushMicrotasks();
    expect(settled).toBe(false);
    expect(onProgress).toHaveBeenCalledExactlyOnceWith(progress);
    expect(mocks.close).not.toHaveBeenCalled();

    download.reject(new Error('UPDATE_DOWNLOAD_CANCELLED'));
    await flushMicrotasks();
    expect(settled).toBe(false);
    expect(mocks.close).not.toHaveBeenCalled();
    cancellation.resolve(undefined);
    await cancelled;
    expect(mocks.close).toHaveBeenCalledExactlyOnceWith(41);
    expect(
      mocks.invoke.mock.calls.filter(([command]) => command === 'cancel_update_download'),
    ).toHaveLength(1);
  });

  it('原生取消错误归一化为 AbortError，且释放下载操作', async () => {
    native.androidDownload = async () => {
      throw 'UPDATE_DOWNLOAD_CANCELLED';
    };
    await expect(ensureApkDownloaded('2.13.0')).rejects.toMatchObject({ name: 'AbortError' });
    expect(mocks.close).toHaveBeenCalledExactlyOnceWith(41);
    expect(isUpdateDownloadCancelled('UPDATE_DOWNLOAD_CANCELLED')).toBe(true);
    expect(isUpdateDownloadCancelled(new Error('disk full'))).toBe(false);
  });
});

describe('desktop update resources and cancellation', () => {
  it('下载只传原生 Update ID，进度结束后保留安装资源；丢弃资源只关闭一次', async () => {
    const download = deferred<number>();
    native.desktopDownload = () => download.promise;
    const update = desktopUpdate();
    const onProgress = vi.fn();
    const result = downloadDesktopUpdate(update, onProgress);
    await vi.waitFor(() => expect(mocks.channels).toHaveLength(1));
    const started = { event: 'Started', data: { contentLength: 10 } };
    mocks.channels[0].onmessage(started);
    expect(onProgress).toHaveBeenCalledExactlyOnceWith(started);
    expect(mocks.invoke).toHaveBeenCalledWith('desktop_download_update', {
      updateRid: 17,
      operationId: 41,
      onEvent: mocks.channels[0],
    });
    download.resolve(91);
    const downloaded = await result;
    expect(downloaded.rid).toBe(91);
    expect(mocks.close.mock.calls).toEqual([[41]]);
    await downloaded.close();
    await downloaded.close();
    expect(mocks.close.mock.calls).toEqual([[41], [91]]);
    await expect(downloaded.install()).rejects.toThrow('already released');
    expect(mocks.invoke).not.toHaveBeenCalledWith('desktop_install_update', expect.anything());
  });

  it('更新检查期间取消：检查完成后释放 Update，不启动下载或安装', async () => {
    const check = deferred<Update>();
    mocks.check.mockReturnValue(check.promise);
    const update = desktopUpdate();
    const controller = new AbortController();
    const result = downloadAndInstallUpdate(undefined, controller.signal);
    const cancelled = expect(result).rejects.toMatchObject({ name: 'AbortError' });
    controller.abort();
    check.resolve(update);
    await cancelled;
    expect(update.close).toHaveBeenCalledOnce();
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(mocks.relaunch).not.toHaveBeenCalled();
  });

  it('取消与下载完成同时发生：丢弃迟到的下载资源，绝不安装或重启', async () => {
    const download = deferred<number>();
    native.desktopDownload = () => download.promise;
    const update = desktopUpdate();
    mocks.check.mockResolvedValue(update);
    const controller = new AbortController();
    const onProgress = vi.fn();
    const onInstalling = vi.fn();
    let settled = false;
    const result = downloadAndInstallUpdate(onProgress, controller.signal, onInstalling).finally(
      () => {
        settled = true;
      },
    );
    const cancelled = expect(result).rejects.toMatchObject({ name: 'AbortError' });
    await vi.waitFor(() => expect(mocks.channels).toHaveLength(1));
    controller.abort();
    mocks.channels[0].onmessage({ event: 'Finished' });
    await flushMicrotasks();
    expect(settled).toBe(false);
    expect(onProgress).not.toHaveBeenCalled();
    download.resolve(91);
    await cancelled;
    expect(mocks.close.mock.calls).toEqual([[41], [91]]);
    expect(update.close).toHaveBeenCalledOnce();
    expect(onInstalling).not.toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalledWith('desktop_install_update', expect.anything());
    expect(mocks.relaunch).not.toHaveBeenCalled();
  });

  it('清理操作资源期间取消也必须丢弃已完成的下载', async () => {
    const operationClose = deferred<void>();
    mocks.close.mockImplementation((rid: number) =>
      rid === 41 ? operationClose.promise : Promise.resolve(),
    );
    const controller = new AbortController();
    const result = downloadDesktopUpdate(desktopUpdate(), undefined, controller.signal);
    const cancelled = expect(result).rejects.toMatchObject({ name: 'AbortError' });
    await vi.waitFor(() => expect(mocks.close).toHaveBeenCalledWith(41));
    controller.abort();
    operationClose.resolve(undefined);
    await cancelled;
    expect(mocks.close.mock.calls).toEqual([[41], [91]]);
  });

  it('成功安装按下载→安装提示→安装→重启顺序执行，资源不重复消费', async () => {
    const update = desktopUpdate();
    mocks.check.mockResolvedValue(update);
    const order: string[] = [];
    native.desktopDownload = async () => {
      order.push('download');
      return 91;
    };
    native.install = async () => {
      order.push('install');
    };
    mocks.relaunch.mockImplementation(async () => {
      order.push('relaunch');
    });
    await downloadAndInstallUpdate(undefined, undefined, () => order.push('installing'));
    expect(order).toEqual(['download', 'installing', 'install', 'relaunch']);
    expect(mocks.check).toHaveBeenCalledExactlyOnceWith({ timeout: 15_000 });
    expect(mocks.invoke).toHaveBeenCalledWith('desktop_install_update', { downloadRid: 91 });
    // Rust 安装成功已消费 91；finally 只能释放操作 41 和检查得到的 Update。
    expect(mocks.close.mock.calls).toEqual([[41]]);
    expect(update.close).toHaveBeenCalledOnce();
  });

  it('安装失败时释放保留的下载资源与 Update，且不重启', async () => {
    const update = desktopUpdate();
    mocks.check.mockResolvedValue(update);
    native.install = async () => {
      throw new Error('installer failed');
    };
    await expect(downloadAndInstallUpdate()).rejects.toThrow('installer failed');
    expect(mocks.close.mock.calls).toEqual([[41], [91]]);
    expect(update.close).toHaveBeenCalledOnce();
    expect(mocks.relaunch).not.toHaveBeenCalled();
  });
});
