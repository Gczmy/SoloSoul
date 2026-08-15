import { describe, expect, it } from 'vitest';
import { SearchCache } from './searchCache';

describe('SearchCache (P038)', () => {
  it('invalidateAccount clears only that account keys', () => {
    const cache = new SearchCache(60_000);
    const a1 = cache.buildKey('acc_A', 'hello');
    const a2 = cache.buildKey('acc_A', 'world');
    const b = cache.buildKey('acc_B', 'hello');
    cache.set(a1, [1]);
    cache.set(a2, [2]);
    cache.set(b, [3]);

    cache.invalidateAccount('acc_A');

    expect(cache.get(a1)).toBeNull();
    expect(cache.get(a2)).toBeNull();
    // 其他账户不受影响
    expect(cache.get(b)).toEqual([3]);
  });

  it('invalidateAccount is a no-op when no keys match', () => {
    const cache = new SearchCache(60_000);
    const key = cache.buildKey('acc_B', 'hello');
    cache.set(key, [1]);
    cache.invalidateAccount('acc_X');
    expect(cache.get(key)).toEqual([1]);
  });

  it('P025: LRU 容量上限——超出时淘汰最久未用条目', () => {
    const cache = new SearchCache(60_000, 3);
    const k1 = cache.buildKey('acc_A', 'q1');
    const k2 = cache.buildKey('acc_A', 'q2');
    const k3 = cache.buildKey('acc_A', 'q3');
    cache.set(k1, [1]);
    cache.set(k2, [2]);
    cache.set(k3, [3]);

    // 命中 k1 刷新其 LRU 序，再插入 k4 → 最久未用的 k2 被淘汰
    expect(cache.get(k1)).toEqual([1]);
    const k4 = cache.buildKey('acc_A', 'q4');
    cache.set(k4, [4]);

    expect(cache.get(k1)).toEqual([1]);
    expect(cache.get(k3)).toEqual([3]);
    expect(cache.get(k4)).toEqual([4]);
    expect(cache.get(k2)).toBeNull();
  });

  it('P025: 覆盖写入不重复占用容量', () => {
    const cache = new SearchCache(60_000, 2);
    const k1 = cache.buildKey('acc_A', 'q1');
    const k2 = cache.buildKey('acc_A', 'q2');
    cache.set(k1, [1]);
    cache.set(k2, [2]);
    cache.set(k1, [10]); // 覆盖 k1
    cache.set(k2, [20]); // 覆盖 k2，不应触发淘汰
    expect(cache.get(k1)).toEqual([10]);
    expect(cache.get(k2)).toEqual([20]);
  });
});
