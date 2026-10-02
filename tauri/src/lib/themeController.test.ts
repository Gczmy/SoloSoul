import { describe, expect, it, vi } from 'vitest';
import type { ThemeConfig } from '@/types';
import { ThemeController, type ThemeSnapshot } from './themeController';

const flush = async () => {
  await Promise.resolve();
  await Promise.resolve();
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
function fixture(preset: ThemeConfig['preset'] = 'system') {
  let snapshot: ThemeSnapshot = {
    config: { preset, accentColor: 'ocean', backgroundType: 'solid', backgroundValue: '' },
    context: 'account-a/session-1',
  };
  const pending = deferred<'light' | 'dark'>();
  let refresh!: () => void;
  let system!: (mode: 'light' | 'dark') => void;
  const disposeSource = vi.fn();
  const disposeSystem = vi.fn();
  const apply = vi.fn(async (_config: ThemeConfig, _guard: () => boolean) => {});
  const onError = vi.fn();
  const subscribe = vi.fn((callback: () => void) => {
    refresh = callback;
    return disposeSource;
  });
  const listenSystem = vi.fn(async (callback: (mode: 'light' | 'dark') => void) => {
    system = callback;
    return disposeSystem;
  });
  const resolveSystem = vi.fn(() => pending.promise);
  const controller = new ThemeController({
    read: () => snapshot,
    subscribe,
    resolveSystem,
    listenSystem,
    apply,
    onError,
  });
  return {
    controller,
    pending,
    apply,
    onError,
    subscribe,
    listenSystem,
    disposeSource,
    disposeSystem,
    resolveSystem,
    set: (next: Partial<ThemeSnapshot>) => {
      snapshot = { ...snapshot, ...next };
      refresh();
    },
    mode: (mode: 'light' | 'dark') => system(mode),
    config: () => snapshot.config,
  };
}

describe('RF-112 唯一主题通道', () => {
  it('旧系统解析不能覆盖新显式主题', async () => {
    const f = fixture();
    const stop = f.controller.acquire();
    f.set({ config: { ...f.config(), preset: 'warm-stone-light' } });
    f.pending.resolve('dark');
    await flush();
    expect(f.apply.mock.calls.map(([config]) => config.preset)).toEqual(['warm-stone-light']);
    stop();
    await flush();
  });
  it('账户/会话变化即使配置相同也撤销旧结果', async () => {
    const f = fixture();
    const old = deferred<'light' | 'dark'>();
    f.resolveSystem.mockImplementationOnce(() => old.promise);
    const stop = f.controller.acquire();
    f.set({ context: 'account-b/session-2' });
    f.pending.resolve('light');
    await flush();
    old.resolve('dark');
    await flush();
    expect(f.apply.mock.calls.map(([config]) => config.resolvedSystemTheme)).toEqual(['light']);
    stop();
    await flush();
  });
  it('卸载立即撤销尚未解析的任务', async () => {
    const f = fixture();
    const stop = f.controller.acquire();
    stop();
    f.pending.resolve('dark');
    await flush();
    expect(f.apply).not.toHaveBeenCalled();
    expect(f.disposeSource).toHaveBeenCalledOnce();
    expect(f.disposeSystem).toHaveBeenCalledOnce();
  });
  it('StrictMode 重挂只保留一个系统和 Store 订阅', async () => {
    const f = fixture();
    const first = f.controller.acquire();
    first();
    const second = f.controller.acquire();
    await flush();
    expect(f.listenSystem).toHaveBeenCalledOnce();
    expect(f.subscribe).toHaveBeenCalledOnce();
    expect(f.disposeSystem).not.toHaveBeenCalled();
    f.pending.resolve('light');
    await flush();
    expect(f.apply).toHaveBeenCalledOnce();
    second();
    second();
    await flush();
    expect(f.disposeSystem).toHaveBeenCalledOnce();
  });
  it('多个持有者不重复应用相同快照，最后一个释放才停止', async () => {
    const f = fixture('warm-stone-light');
    const a = f.controller.acquire();
    const b = f.controller.acquire();
    expect(f.apply).toHaveBeenCalledOnce();
    a();
    await flush();
    expect(f.disposeSource).not.toHaveBeenCalled();
    b();
    await flush();
    expect(f.disposeSource).toHaveBeenCalledOnce();
  });
  it.each(['warm-stone-light', 'warm-stone-dark'] as const)('%s 忽略系统切色', async (preset) => {
    const f = fixture(preset);
    const stop = f.controller.acquire();
    f.mode('dark');
    f.mode('light');
    await flush();
    expect(f.apply).toHaveBeenCalledOnce();
    expect(f.resolveSystem).not.toHaveBeenCalled();
    stop();
    await flush();
  });
  it('系统事件使较早的检测请求失效', async () => {
    const f = fixture();
    const stop = f.controller.acquire();
    f.mode('light');
    f.pending.resolve('dark');
    await flush();
    expect(f.apply.mock.calls.map(([config]) => config.resolvedSystemTheme)).toEqual(['light']);
    stop();
    await flush();
  });
  it('监听创建晚于卸载时立即释放，旧回调不可交付', async () => {
    const f = fixture('warm-stone-light');
    const subscription = deferred<typeof f.disposeSystem>();
    f.listenSystem.mockImplementationOnce((callback) => {
      // 监听建立未完成时原生源已可能持有回调。
      f.mode = callback;
      return subscription.promise;
    });
    const stop = f.controller.acquire();
    stop();
    await flush();
    subscription.resolve(f.disposeSystem);
    await flush();
    f.mode('dark');
    expect(f.disposeSystem).toHaveBeenCalledOnce();
    expect(f.apply).toHaveBeenCalledOnce();
  });
  it('启动缓存交接后不再允许回填，交接前的 guard 失效', async () => {
    const f = fixture('warm-stone-light');
    await f.controller.applyStartup(f.config());
    const guard = f.apply.mock.calls[0][1];
    expect(guard()).toBe(true);
    const stop = f.controller.acquire();
    expect(guard()).toBe(false);
    await f.controller.applyStartup({ ...f.config(), preset: 'warm-stone-dark' });
    expect(f.apply).toHaveBeenCalledTimes(2);
    stop();
    await flush();
  });
  it('应用失败允许同一快照重试，且记录当前错误', async () => {
    const f = fixture('warm-stone-light');
    f.apply.mockRejectedValueOnce(new Error('native failure'));
    const stop = f.controller.acquire();
    await flush();
    expect(f.onError).toHaveBeenCalledOnce();
    await f.controller.refresh();
    expect(f.apply).toHaveBeenCalledTimes(2);
    stop();
    await flush();
  });
});
