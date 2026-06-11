import { useState, useCallback, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Search, X, Loader2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { createPortal } from 'react-dom';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import styles from './SearchPopover.module.css';

interface SearchItem {
  objectId: string;
  name: string;
  collectionType: string;
  matchedField?: string;
  matchedValue?: string;
  relevance: number;
}

interface SearchPopoverProps {
  onClose: () => void;
}

export function SearchPopover({ onClose }: SearchPopoverProps) {
  const navigate = useNavigate();
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { onError } = useToastError();
  const { t } = useTranslation('common');
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchItem[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

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
      onError(e, t('search_failed'));
    } finally {
      setIsSearching(false);
    }
  }, [accountId, onError, t]);

  const handleChange = (val: string) => {
    setQuery(val);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => doSearch(val), 300);
  };

  const handleClickResult = (item: SearchItem) => {
    onClose();
    navigate(`/editor/${item.objectId}?section=${item.collectionType}`);
  };

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) onClose();
  };

  return createPortal(
    <div className={styles.backdrop} onClick={handleBackdropClick}>
      <div ref={cardRef} className={styles.card}>
        {/* Search input */}
        <div className={styles.inputRow}>
          <Search size={16} className={styles.inputIcon} />
          <input
            ref={inputRef}
            className={styles.input}
            placeholder={t('search_placeholder')}
            value={query}
            onChange={(e) => handleChange(e.target.value)}
          />
          {isSearching ? (
            <Loader2 size={16} className={styles.spinner} />
          ) : (
            <button className={styles.closeBtn} onClick={onClose} aria-label={t('cancel')}>
              <X size={16} />
            </button>
          )}
        </div>

        {/* Results */}
        <div className={styles.results}>
          {!isSearching && hasSearched && results.length === 0 && (
            <div className={styles.empty}>{t('no_results')}</div>
          )}

          {results.map((item) => (
            <button
              key={item.objectId}
              className={styles.resultItem}
              onClick={() => handleClickResult(item)}
            >
              <PAGE_ICON_MAP.custom size={16} />
              <div className={styles.resultText}>
                <div className={styles.resultName}>{item.name}</div>
                <div className={styles.resultMeta}>
                  {item.collectionType}
                  {item.matchedField && ` · ${item.matchedField}`}
                </div>
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>,
    document.body
  );
}
