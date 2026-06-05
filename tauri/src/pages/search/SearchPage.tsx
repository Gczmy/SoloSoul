import { useState, useCallback, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { FileText } from 'lucide-react';

interface SearchItem {
  objectId: string;
  name: string;
  collectionType: string;
  matchedField?: string;
  matchedValue?: string;
  relevance: number;
}

export function SearchPage() {
  const navigate = useNavigate();
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { onError } = useToastError();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchItem[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const doSearch = useCallback(async (q: string) => {
    if (!accountId || !q.trim()) {
      setResults([]);
      setHasSearched(false);
      return;
    }
    setIsSearching(true);
    setHasSearched(true);
    try {
      const res = await invoke<{ items: SearchItem[]; total: number; hasMore: boolean }>(
        'search_unified', { accountId, query: q, limit: 50 }
      );
      setResults(res.items);
    } catch (e) {
      onError(e, 'Search failed');
    } finally {
      setIsSearching(false);
    }
  }, [accountId, onError]);

  const handleChange = (val: string) => {
    setQuery(val);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => doSearch(val), 300);
  };

  return (
    <AppShell title="Search">
      <div style={{ maxWidth: 600, margin: '0 auto' }}>
        <Input
          placeholder="Search objects, profiles..."
          value={query}
          onChange={(e) => handleChange(e.target.value)}
          autoFocus
        />

        <div style={{ marginTop: 12 }}>
          {isSearching && (
            <p style={{ fontSize: 13, color: 'var(--text-tertiary)', textAlign: 'center' }}>
              Searching...
            </p>
          )}

          {!isSearching && hasSearched && results.length === 0 && (
            <Card>
              <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: '24px 0' }}>
                No results found
              </p>
            </Card>
          )}

          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {results.map((item) => (
              <Card
                key={item.objectId}
                interactive
                onClick={() => navigate(`/editor/${item.objectId}`)}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                  <FileText size={18} />
                  <div>
                    <div style={{ fontSize: 14, fontWeight: 500 }}>{item.name}</div>
                    <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                      {item.collectionType}
                      {item.matchedField && ` · matched: ${item.matchedField}`}
                    </div>
                  </div>
                </div>
              </Card>
            ))}
          </div>
        </div>
      </div>
    </AppShell>
  );
}
