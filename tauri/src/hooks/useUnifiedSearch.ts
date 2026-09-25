import { useCallback, useEffect, useRef, useState } from 'react';
import type { TFunction } from 'i18next';
import type { CustomPage } from '@/stores/settingsStore';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { runUnifiedSearch, type SearchItem } from '@/lib/searchShared';
import { searchCache } from '@/lib/searchCache';

interface SearchOptions {
  accountId: string | null | undefined;
  customPages: CustomPage[];
  t: TFunction;
  onError: (error: unknown, fallback: string) => void;
}

/** 两个搜索入口共用查询生命周期；失效发生在输入事件时，而非 debounce 到期后。 */
export function useUnifiedSearch(options: SearchOptions) {
  const [requests] = useState(createSessionRequests);
  const timeout = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<string | null>(null);
  const [results, setResults] = useState<SearchItem[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);

  const cancel = useCallback(() => {
    requests.invalidate();
    clearTimeout(timeout.current);
    timeout.current = undefined;
  }, [requests]);

  const reset = useCallback(() => {
    cancel();
    setQuery('');
    setFilter(null);
    setResults([]);
    setIsSearching(false);
    setHasSearched(false);
  }, [cancel]);

  useEffect(() => {
    reset();
    const unsubscribe = onRequestSessionChange(reset);
    return () => {
      unsubscribe();
      cancel();
    };
  }, [options.accountId, reset, cancel]);

  const search = useCallback(
    (value: string, nextFilter: string | null, immediate = false) => {
      cancel();
      setQuery(value);
      setFilter(nextFilter);
      setResults([]);
      const active = !!options.accountId && (!!value.trim() || !!nextFilter);
      setHasSearched(active);
      setIsSearching(active);
      if (!active) return;
      const request = requests.begin('search', options.accountId!);
      const execute = async () => {
        if (!request.isCurrent()) return;
        try {
          const result = await runUnifiedSearch({
            ...options,
            query: value,
            filter: nextFilter,
            invokeSearch: request.invoke,
          });
          if (!request.isCurrent()) return;
          if (result.cacheKey && !result.cached) searchCache.set(result.cacheKey, result.items);
          setResults(result.items);
          setHasSearched(result.hasSearched);
        } catch (error) {
          if (request.isCurrent()) options.onError(error, options.t('common:search_failed'));
        } finally {
          if (request.isCurrent()) setIsSearching(false);
        }
      };
      if (immediate) void execute();
      else
        timeout.current = setTimeout(() => {
          timeout.current = undefined;
          void execute();
        }, DEBOUNCE_DELAY_MS);
    },
    [cancel, requests, options],
  );

  return {
    query,
    filter,
    results,
    isSearching,
    hasSearched,
    changeQuery: (value: string) => search(value, filter),
    changeFilter: (value: string | null) => search(query, value, true),
    searchNow: (value: string) => search(value, filter, true),
    clear: () => search('', filter, true),
  };
}
