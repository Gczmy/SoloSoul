/**
 * 通用数据预取 store 工厂（Prefetch Runtime，docs/prefetch-runtime-design.md）。
 *
 * 模块级单例：in-flight 去重（并发 load 共享同一 promise）+ TTL 缓存复用 +
 * 显式 invalidate + 后台 warmup（吞错）+ 平台门控。页面经 usePrefetchData
 * 订阅，数据就绪时直接渲染（无骨架期），来回切换零 IPC。
 */

export type WarmupPolicy = 'always' | 'afterAuth' | 'never';

export interface PrefetchStoreOptions<T> {
  key: string;
  /** 该数据的一次 IPC 加载（失败时返回 null，错误记入 error）。 */
  loader: () => Promise<T>;
  /** 缓存有效期，默认 5 分钟。 */
  ttlMs?: number;
  /** 预热时机：'always'=App 挂载，'afterAuth'=登录/解锁后，'never'=仅按需。默认 'never'。 */
  warmupPolicy?: WarmupPolicy;
  /** 平台门控：false 时跳过后台预热（页面显式 load 仍可用）。 */
  enabledOnPlatform?: () => boolean;
}

export interface PrefetchSnapshot<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  lastLoadedAt: number | null;
}

export interface PrefetchStore<T> {
  readonly key: string;
  readonly options: PrefetchStoreOptions<T>;
  /** 幂等加载：in-flight 去重 + TTL 命中复用；force 跳过 TTL 强制刷新。 */
  load: (opts?: { force?: boolean }) => Promise<T | null>;
  /** 清缓存并立即重载（变更操作后调用）。 */
  invalidate: () => Promise<T | null>;
  /** 后台预热：吞错、平台门控，失败由页面挂载兜底。 */
  warmup: () => void;
  reset: () => void;
  /** useSyncExternalStore 订阅接口。 */
  subscribe: (listener: () => void) => () => void;
  getSnapshot: () => PrefetchSnapshot<T>;
}

export function createPrefetchStore<T>(options: PrefetchStoreOptions<T>): PrefetchStore<T> {
  const { key, loader, ttlMs = 5 * 60_000, enabledOnPlatform } = options;

  let data: T | null = null;
  let loading = false;
  let error: string | null = null;
  let lastLoadedAt: number | null = null;
  let pending: Promise<T | null> | null = null;
  const listeners = new Set<() => void>();
  let snapshot: PrefetchSnapshot<T> = {
    data: null,
    loading: false,
    error: null,
    lastLoadedAt: null,
  };

  function emit() {
    snapshot = { data, loading, error, lastLoadedAt };
    for (const listener of listeners) listener();
  }

  function isFresh(): boolean {
    return lastLoadedAt !== null && Date.now() - lastLoadedAt < ttlMs;
  }

  async function runLoader(): Promise<T | null> {
    loading = true;
    error = null;
    emit();
    try {
      const result = await loader();
      data = result;
      lastLoadedAt = Date.now();
      return result;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      return null;
    } finally {
      loading = false;
      emit();
    }
  }

  const store: PrefetchStore<T> = {
    key,
    options,
    load: (opts) => {
      const force = opts?.force ?? false;
      if (!force && pending) return pending;
      if (!force && isFresh() && data !== null) return Promise.resolve(data);
      pending = runLoader().finally(() => {
        pending = null;
      });
      return pending;
    },
    invalidate: () => {
      data = null;
      lastLoadedAt = null;
      return store.load({ force: true });
    },
    warmup: () => {
      if (enabledOnPlatform && !enabledOnPlatform()) return;
      void store.load().catch(() => {});
    },
    reset: () => {
      data = null;
      loading = false;
      error = null;
      lastLoadedAt = null;
      pending = null;
      emit();
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    getSnapshot: () => snapshot,
  };

  return store;
}
