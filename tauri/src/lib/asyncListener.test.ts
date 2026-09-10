import { describe, expect, it, vi } from 'vitest';
import { trackAsyncListener } from './asyncListener';
import { logger } from './logger';

vi.mock('./logger', () => ({ logger: { warn: vi.fn() } }));

describe('trackAsyncListener', () => {
  it('订阅完成后的清理只释放一次', async () => {
    const unlisten = vi.fn();
    const dispose = trackAsyncListener(Promise.resolve(unlisten));
    await Promise.resolve();
    expect(unlisten).not.toHaveBeenCalled();
    dispose();
    dispose();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('订阅完成前的清理会释放迟到的句柄', async () => {
    let resolve!: (fn: () => void) => void;
    const unlisten = vi.fn();
    const dispose = trackAsyncListener(
      new Promise((done) => {
        resolve = done;
      }),
    );
    dispose();
    dispose();
    resolve(unlisten);
    await Promise.resolve();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('订阅失败被记录，无未处理的 Promise rejection', async () => {
    const error = new Error('registration failed');
    const dispose = trackAsyncListener(Promise.reject(error));
    await Promise.resolve();
    await Promise.resolve();
    dispose();
    expect(logger.warn).toHaveBeenCalledWith(expect.any(String), error);
  });

  it('不可用的监听器和失败的退订不妨碍其余清理', async () => {
    const unavailable = trackAsyncListener(Promise.resolve(null));
    const unlisten = vi.fn(() => {
      throw new Error('cleanup failed');
    });
    const dispose = trackAsyncListener(Promise.resolve(unlisten));
    await Promise.resolve();
    expect(() => {
      unavailable();
      dispose();
      dispose();
    }).not.toThrow();
    expect(unlisten).toHaveBeenCalledOnce();
  });
});
