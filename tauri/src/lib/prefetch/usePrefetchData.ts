/**
 * 预取数据消费 hook（Prefetch Runtime，docs/prefetch-runtime-design.md）。
 *
 * 订阅 store 快照；挂载时若缓存缺失/过期则现场 load（冷启动兜底），
 * 缓存命中则直接使用——进入页面 0 加载期、无骨架。reload 强制刷新。
 */
import { useEffect, useSyncExternalStore } from 'react';
import type { PrefetchStore, PrefetchSnapshot } from './createPrefetchStore';

export interface UsePrefetchDataResult<T> extends PrefetchSnapshot<T> {
  /** 强制刷新（跳过 TTL）。 */
  reload: () => Promise<T | null>;
}

export function usePrefetchData<T>(
  store: PrefetchStore<T>,
  options?: { enabled?: boolean },
): UsePrefetchDataResult<T> {
  const enabled = options?.enabled ?? true;
  const snapshot = useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot);

  useEffect(() => {
    if (!enabled) return;
    void store.load().catch(() => {});
  }, [store, enabled]);

  return {
    ...snapshot,
    reload: () => store.load({ force: true }),
  };
}
