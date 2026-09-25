import { act, renderHook, cleanup } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { TFunction } from 'i18next';
import { useUnifiedSearch } from './useUnifiedSearch';
import { invokeCommand } from '@/lib/ipcClient';
import { searchCache } from '@/lib/searchCache';
import { setRequestSession } from '@/lib/sessionRequests';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';

vi.mock('@/lib/ipcClient', () => ({ invokeCommand: vi.fn() }));

function deferred() {
  let resolve!: (value: unknown) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
const item = (name: string) => ({ objectId: name, name, typeId: 'note', relevance: 1 });
const options = {
  accountId: 'a',
  customPages: [],
  t: ((key: string) => key) as TFunction,
  onError: vi.fn(),
};
const flush = async () => {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(DEBOUNCE_DELAY_MS);
  });
};

describe('useUnifiedSearch request lifecycle', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    setRequestSession('a');
    searchCache.clear();
  });
  afterEach(() => {
    cleanup();
    setRequestSession(null);
    vi.useRealTimers();
  });

  it('debounces and only commits the latest response and cache', async () => {
    const a = deferred(),
      b = deferred();
    vi.mocked(invokeCommand).mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise);
    const { result } = renderHook(() => useUnifiedSearch(options));
    act(() => result.current.changeQuery('aaa'));
    await flush();
    act(() => result.current.changeQuery('bbb'));
    await flush();
    await act(async () => {
      b.resolve({ items: [item('B')] });
    });
    await act(async () => {
      a.resolve({ items: [item('A')] });
    });
    expect(result.current.results).toEqual([item('B')]);
    expect(searchCache.get(searchCache.buildKey('a', 'aaa'))).toBeNull();
    expect(searchCache.get(searchCache.buildKey('a', 'bbb'))).toEqual([item('B')]);
  });

  it('invalidates during debounce and an old finally cannot stop the new loading state', async () => {
    const a = deferred(),
      b = deferred();
    vi.mocked(invokeCommand).mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise);
    const { result } = renderHook(() => useUnifiedSearch(options));
    act(() => result.current.searchNow('aaa'));
    act(() => result.current.changeQuery('bbb'));
    await act(async () => {
      a.reject(new Error('old failure'));
    });
    expect(result.current.isSearching).toBe(true);
    expect(options.onError).not.toHaveBeenCalled();
    await flush();
    await act(async () => {
      b.resolve({ items: [item('B')] });
    });
    expect(result.current.isSearching).toBe(false);
  });

  it('clear cancels the timer and rejects an in-flight response', async () => {
    const a = deferred();
    vi.mocked(invokeCommand).mockReturnValueOnce(a.promise);
    const { result } = renderHook(() => useUnifiedSearch(options));
    act(() => result.current.searchNow('aaa'));
    act(() => result.current.changeQuery('bbb'));
    act(() => result.current.clear());
    await flush();
    await act(async () => {
      a.resolve({ items: [item('A')] });
    });
    expect(invokeCommand).toHaveBeenCalledTimes(1);
    expect(result.current).toMatchObject({
      query: '',
      results: [],
      hasSearched: false,
      isSearching: false,
    });
    expect(searchCache.get(searchCache.buildKey('a', 'aaa'))).toBeNull();
  });

  it('filter change cancels a pending debounce and runs with the new filter immediately', async () => {
    vi.mocked(invokeCommand).mockResolvedValue({ items: [] });
    const { result } = renderHook(() => useUnifiedSearch(options));
    act(() => result.current.changeQuery('aaa'));
    await act(async () => {
      result.current.changeFilter('travel');
    });
    await flush();
    expect(invokeCommand).toHaveBeenCalledTimes(1);
    expect(invokeCommand).toHaveBeenCalledWith(
      'search_unified',
      expect.objectContaining({ query: 'aaa', typeId: 'travel' }),
      expect.any(Object),
    );
  });

  it.each(['resolve', 'reject'] as const)(
    'lock and account switch reject late %s and clear existing cache',
    async (outcome) => {
      const a = deferred(),
        b = deferred();
      vi.mocked(invokeCommand).mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise);
      const { result, rerender } = renderHook(
        ({ accountId }) => useUnifiedSearch({ ...options, accountId }),
        { initialProps: { accountId: 'a' } },
      );
      searchCache.set(searchCache.buildKey('a', 'old'), [item('secret')]);
      act(() => result.current.searchNow('aaa'));
      act(() => setRequestSession(null));
      expect(result.current).toMatchObject({ query: '', results: [], isSearching: false });
      expect(searchCache.get(searchCache.buildKey('a', 'old'))).toBeNull();
      act(() => setRequestSession('b'));
      rerender({ accountId: 'b' });
      act(() => result.current.searchNow('bbb'));
      await act(async () => {
        if (outcome === 'resolve') a.resolve({ items: [item('A')] });
        else a.reject(new Error('old failure'));
      });
      expect(result.current.isSearching).toBe(true);
      expect(result.current.results).toEqual([]);
      expect(options.onError).not.toHaveBeenCalled();
      expect(searchCache.get(searchCache.buildKey('a', 'aaa'))).toBeNull();
      await act(async () => {
        b.resolve({ items: [item('B')] });
      });
      expect(result.current.results).toEqual([item('B')]);
    },
  );

  it('unmount prevents late cache/error writes and cancels pending timers', async () => {
    const a = deferred();
    vi.mocked(invokeCommand).mockReturnValueOnce(a.promise);
    const { result, unmount } = renderHook(() => useUnifiedSearch(options));
    act(() => result.current.searchNow('aaa'));
    unmount();
    const pending = renderHook(() => useUnifiedSearch(options));
    act(() => pending.result.current.changeQuery('bbb'));
    pending.unmount();
    await flush();
    await act(async () => {
      a.resolve({ items: [item('A')] });
    });
    expect(invokeCommand).toHaveBeenCalledTimes(1);
    expect(searchCache.get(searchCache.buildKey('a', 'aaa'))).toBeNull();
    expect(options.onError).not.toHaveBeenCalled();
  });

  it('relocking the same account invalidates the old generation without canceling another search entry', async () => {
    const old = deferred(),
      left = deferred(),
      right = deferred();
    vi.mocked(invokeCommand)
      .mockReturnValueOnce(old.promise)
      .mockReturnValueOnce(left.promise)
      .mockReturnValueOnce(right.promise);
    const first = renderHook(() => useUnifiedSearch(options));
    const second = renderHook(() => useUnifiedSearch(options));
    act(() => first.result.current.searchNow('old'));
    act(() => {
      setRequestSession(null);
      setRequestSession('a');
    });
    act(() => first.result.current.searchNow('left'));
    act(() => second.result.current.searchNow('right'));
    await act(async () => {
      old.resolve({ items: [item('secret')] });
      left.resolve({ items: [item('left')] });
      right.resolve({ items: [item('right')] });
    });
    expect(first.result.current.results).toEqual([item('left')]);
    expect(second.result.current.results).toEqual([item('right')]);
    expect(searchCache.get(searchCache.buildKey('a', 'old'))).toBeNull();
  });

  it('current errors end loading and cached results avoid IPC', async () => {
    vi.mocked(invokeCommand).mockRejectedValueOnce(new Error('current failure'));
    const { result } = renderHook(() => useUnifiedSearch(options));
    await act(async () => {
      result.current.searchNow('aaa');
    });
    expect(options.onError).toHaveBeenCalledTimes(1);
    expect(result.current.isSearching).toBe(false);
    searchCache.set(searchCache.buildKey('a', 'bbb'), [item('B')]);
    await act(async () => {
      result.current.searchNow('bbb');
    });
    expect(result.current.results).toEqual([item('B')]);
    expect(invokeCommand).toHaveBeenCalledTimes(1);
  });
});
