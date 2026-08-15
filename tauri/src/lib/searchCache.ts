/**
 * Simple in-memory search result cache with TTL.
 * Shared between SearchPage and SearchPopover so same keyword
 * doesn't re-request the backend within the cache window.
 */

interface CacheEntry<T> {
  data: T;
  timestamp: number;
}

export class SearchCache {
  private cache = new Map<string, CacheEntry<unknown>>();
  private ttl: number;
  private maxEntries: number;

  /** @param maxEntries LRU 容量上限——超出按最久未用淘汰（见 P025）。 */
  constructor(ttl: number, maxEntries = 200) {
    this.ttl = ttl;
    this.maxEntries = maxEntries;
  }

  /** Build a normalized cache key from search parameters. */
  buildKey(
    accountId: string,
    query: string,
    typeId?: string | null,
    parentId?: string | null,
  ): string {
    return `${accountId}::${query}::${typeId ?? ''}::${parentId ?? ''}`;
  }

  /** Retrieve cached data if fresh, or null. */
  get<T>(key: string): T | null {
    const entry = this.cache.get(key);
    if (!entry) return null;
    if (Date.now() - entry.timestamp > this.ttl) {
      this.cache.delete(key);
      return null;
    }
    // P025: 命中即刷新 LRU 顺序（与 photoAlbumPreview.createBoundedCache 同构）
    this.cache.delete(key);
    this.cache.set(key, entry);
    return entry.data as T;
  }

  /** Store data in cache. */
  set<T>(key: string, data: T): void {
    if (this.cache.has(key)) this.cache.delete(key);
    this.cache.set(key, { data, timestamp: Date.now() });
    // P025: 容量上限——TTL 只做读取时惰性淘汰，会话内解密结果明文驻留内存
    // 只增不减，必须加 LRU 硬上限兜底。
    while (this.cache.size > this.maxEntries) {
      const oldest = this.cache.keys().next().value as string | undefined;
      if (oldest === undefined) break;
      this.cache.delete(oldest);
    }
  }

  /** Clear all cached entries (e.g. on logout). */
  clear(): void {
    this.cache.clear();
  }

  /**
   * P038: 按 accountId 前缀失效该账户的全部搜索缓存（对象写操作后调用，
   * 避免 30s TTL 窗口内新建/编辑的对象搜不到）。
   */
  invalidateAccount(accountId: string): void {
    const prefix = `${accountId}::`;
    for (const key of this.cache.keys()) {
      if (key.startsWith(prefix)) {
        this.cache.delete(key);
      }
    }
  }
}

import { SEARCH_CACHE_TTL_MS } from '@/lib/constants';

/** Singleton search result cache shared by SearchPage and SearchPopover. */
export const searchCache = new SearchCache(SEARCH_CACHE_TTL_MS);
