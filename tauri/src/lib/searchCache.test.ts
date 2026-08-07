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
});
