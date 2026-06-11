import { useState, useCallback, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Search, X, Loader2, Settings, Clock, User, Plane, CreditCard, Briefcase } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { createPortal } from 'react-dom';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useToastError } from '@/hooks/useToastError';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import styles from './SearchPopover.module.css';

const FILTER_PAGES = [
  { key: 'identity', labelKey: 'navigation:identity', icon: User },
  { key: 'travel', labelKey: 'navigation:travel', icon: Plane },
  { key: 'financial', labelKey: 'navigation:financial', icon: CreditCard },
  { key: 'professional', labelKey: 'navigation:professional', icon: Briefcase },
];

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

const RECENT_KEY = 'solosoul_recent_searches';

function loadRecent(): string[] {
  try {
    return JSON.parse(localStorage.getItem(RECENT_KEY) || '[]');
  } catch {
    return [];
  }
}

function saveRecent(query: string) {
  const prev = loadRecent();
  const next = [query, ...prev.filter((q) => q !== query)].slice(0, 3);
  localStorage.setItem(RECENT_KEY, JSON.stringify(next));
}

export function SearchPopover({ onClose }: SearchPopoverProps) {
  const navigate = useNavigate();
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const activeCustomPages = customPages.filter((p) => !p.deletedAt);
  const { onError } = useToastError();
  const { t } = useTranslation(['common', 'navigation']);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchItem[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const [recent, setRecent] = useState<string[]>(loadRecent);
  const [selectedFilter, setSelectedFilter] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
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

  const doSearch = useCallback(async (q: string, filter: string | null) => {
    if (!accountId || !q.trim()) {
      setResults([]);
      setHasSearched(false);
      return;
    }
    setIsSearching(true);
    setHasSearched(true);
    try {
      const res = await invoke<{ items: SearchItem[]; total: number; hasMore: boolean }>(
        'search_unified',
        { accountId, query: q, limit: 50, collectionType: filter || undefined }
      );
      setResults(res.items);
    } catch (e) {
      onError(e, t('common:search_failed'));
    } finally {
      setIsSearching(false);
    }
  }, [accountId, onError, t]);

  const handleChange = (val: string) => {
    setQuery(val);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => doSearch(val, selectedFilter), 300);
  };

  const handleFilter = (key: string | null) => {
    const next = selectedFilter === key ? null : key;
    setSelectedFilter(next);
    doSearch(query, next);
  };

  const handleSubmit = () => {
    if (query.trim()) {
      saveRecent(query.trim());
      setRecent(loadRecent());
    }
  };

  const handleClickResult = (item: SearchItem) => {
    if (query.trim()) saveRecent(query.trim());
    onClose();
    navigate(`/editor/${item.objectId}?section=${item.collectionType}`);
  };

  const handleRecentClick = (q: string) => {
    setQuery(q);
    doSearch(q, selectedFilter);
  };

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) onClose();
  };

  const showDefaultView = !hasSearched || (!isSearching && query.trim() === '');

  return createPortal(
    <div className={styles.backdrop} onClick={handleBackdropClick}>
      <div className={styles.card}>
        {/* 1. Search input */}
        <div className={styles.inputRow}>
          <Search size={16} className={styles.inputIcon} />
          <input
            ref={inputRef}
            className={styles.input}
            placeholder={t('common:search_placeholder')}
            value={query}
            onChange={(e) => handleChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleSubmit();
            }}
          />
          {isSearching ? (
            <Loader2 size={16} className={styles.spinner} />
          ) : (
            <button className={styles.closeBtn} onClick={onClose} aria-label={t('common:cancel')}>
              <X size={16} />
            </button>
          )}
        </div>

        {/* 2. Filter bar */}
        <div className={styles.filterBar}>
          {FILTER_PAGES.map((f) => {
            const Icon = f.icon;
            const active = selectedFilter === f.key;
            return (
              <button
                key={f.key}
                className={`${styles.filterBtn} ${active ? styles.filterBtnActive : ''}`}
                onClick={() => handleFilter(f.key)}
              >
                <Icon size={12} />
                <span>{t(f.labelKey)}</span>
              </button>
            );
          })}
          {activeCustomPages.map((p) => {
            const active = selectedFilter === p.id;
            return (
              <button
                key={p.id}
                className={`${styles.filterBtn} ${active ? styles.filterBtnActive : ''}`}
                onClick={() => handleFilter(p.id)}
              >
                <PAGE_ICON_MAP.custom size={12} />
                <span>{p.name}</span>
              </button>
            );
          })}
        </div>

        {/* Content area */}
        <div className={styles.content}>
          {/* Search results */}
          {hasSearched && query.trim() !== '' && (
            <>
              {!isSearching && results.length === 0 && (
                <div className={styles.empty}>{t('common:no_results')}</div>
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
            </>
          )}

          {/* Default view when no search active */}
          {showDefaultView && (
            <>
              {/* 3. Recent searches */}
              {recent.length > 0 && (
                <div className={styles.section}>
                  <div className={styles.sectionTitle}>
                    <Clock size={13} />
                    <span>{t('common:recent_searches')}</span>
                  </div>
                  {recent.map((q) => (
                    <button
                      key={q}
                      className={styles.recentItem}
                      onClick={() => handleRecentClick(q)}
                    >
                      <Search size={13} />
                      <span>{q}</span>
                    </button>
                  ))}
                </div>
              )}

              {/* 4. Settings */}
              <div className={styles.section}>
                <button
                  className={styles.settingsItem}
                  onClick={() => { onClose(); navigate('/settings'); }}
                >
                  <Settings size={16} />
                  <span>{t('navigation:settings')}</span>
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>,
    document.body
  );
}
