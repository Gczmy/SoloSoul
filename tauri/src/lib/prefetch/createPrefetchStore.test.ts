import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createPrefetchStore, type PrefetchStore } from './createPrefetchStore';

const TTL = 60_000;

function makeStore<T>(
  loader: () => Promise<T>,
  opts: Partial<Parameters<typeof createPrefetchStore<T>>[0]> = {},
) {
  return createPrefetchStore<T>({
    key: 'test',
    loader,
    ttlMs: TTL,
    ...opts,
  });
}

describe('createPrefetchStore', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  it('loads data and caches it (TTL 内二次 load 不重复调用 loader)', async () => {
    const loader = vi.fn().mockResolvedValue('data-1');
    const store = makeStore(loader);

    const first = await store.load();
    expect(first).toBe('data-1');
    expect(loader).toHaveBeenCalledTimes(1);

    const second = await store.load();
    expect(second).toBe('data-1');
    expect(loader).toHaveBeenCalledTimes(1); // 缓存命中，未再调 loader

    expect(store.getSnapshot().lastLoadedAt).not.toBeNull();
  });

  it('并发 load 共享同一 in-flight promise（去重）', async () => {
    let resolveFn: (v: string) => void = () => {};
    const loader = vi.fn().mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          resolveFn = resolve;
        }),
    );
    const store = makeStore(loader);

    const p1 = store.load();
    const p2 = store.load();
    const p3 = store.load();
    expect(loader).toHaveBeenCalledTimes(1); // 三个并发只触发一次 loader

    resolveFn('loaded');
    const [r1, r2, r3] = await Promise.all([p1, p2, p3]);
    expect(r1).toBe('loaded');
    expect(r2).toBe('loaded');
    expect(r3).toBe('loaded');
  });

  it('TTL 过期后重新加载', async () => {
    const loader = vi.fn().mockResolvedValue('v1');
    const store = makeStore(loader);
    await store.load();
    expect(loader).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(TTL + 1);
    await store.load();
    expect(loader).toHaveBeenCalledTimes(2);
  });

  it('force 强制刷新跳过 TTL', async () => {
    const loader = vi.fn().mockResolvedValueOnce('v1').mockResolvedValueOnce('v2');
    const store = makeStore(loader);
    await store.load();
    await store.load({ force: true });
    expect(loader).toHaveBeenCalledTimes(2);
    expect(store.getSnapshot().data).toBe('v2');
  });

  it('invalidate 清缓存并立即重载', async () => {
    const loader = vi.fn().mockResolvedValueOnce('old').mockResolvedValueOnce('new');
    const store = makeStore(loader);
    await store.load();
    const result = await store.invalidate();
    expect(result).toBe('new');
    expect(store.getSnapshot().data).toBe('new');
    expect(store.getSnapshot().lastLoadedAt).not.toBeNull();
  });

  it('warmup 遵循平台门控（enabledOnPlatform false 时不加载）', async () => {
    const loader = vi.fn().mockResolvedValue('data');
    const store = makeStore(loader, { enabledOnPlatform: () => false });
    store.warmup();
    await vi.advanceTimersByTimeAsync(0);
    expect(loader).not.toHaveBeenCalled();
  });

  it('warmup 在门控开启时后台加载', async () => {
    const loader = vi.fn().mockResolvedValue('data');
    const store = makeStore(loader, { enabledOnPlatform: () => true });
    store.warmup();
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(0);
    expect(loader).toHaveBeenCalledTimes(1);
  });

  it('loader 失败时记录 error 并返回 null，下次 load 可重试', async () => {
    const loader = vi
      .fn()
      .mockRejectedValueOnce(new Error('backend down'))
      .mockResolvedValueOnce('ok');
    const store = makeStore(loader);
    const result = await store.load();
    expect(result).toBeNull();
    expect(store.getSnapshot().error).toBe('backend down');
    expect(store.getSnapshot().data).toBeNull();

    const retry = await store.load();
    expect(retry).toBe('ok');
    expect(store.getSnapshot().error).toBeNull();
  });

  it('subscribe 在状态变化时通知（loading → data）', async () => {
    const loader = vi.fn().mockResolvedValue('data');
    const store = makeStore(loader);
    const events: Array<'loading' | 'data'> = [];
    const unsub = store.subscribe(() => {
      const s = store.getSnapshot();
      events.push(s.loading ? 'loading' : 'data');
    });

    await store.load();
    expect(events).toContain('loading');
    expect(events).toContain('data');
    unsub();
  });

  it('reset 清空缓存与错误', async () => {
    const loader = vi.fn().mockResolvedValueOnce('v1').mockResolvedValueOnce('v2');
    const store: PrefetchStore<string> = makeStore(loader);
    await store.load();
    store.reset();
    expect(store.getSnapshot().data).toBeNull();
    expect(store.getSnapshot().lastLoadedAt).toBeNull();
    const again = await store.load();
    expect(again).toBe('v2');
  });
});
