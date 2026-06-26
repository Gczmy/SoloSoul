import { useState, useCallback, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  Search,
  X,
  Loader2,
  Settings,
  Clock,
  User,
  Plane,
  CreditCard,
  Briefcase,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { createPortal } from 'react-dom';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useToastError } from '@/hooks/useToastError';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import { ObjectDetailModal } from '@/components/object/ObjectDetailModal';
import { SensitivityBadge, SensitivityLevel } from '@/components/ui/SensitivityBadge';
import styles from './SearchPopover.module.css';
import { ICON_SIZE } from '@/lib/iconSizes';


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
  itemType: 'object' | 'page';
  parentId?: string;
  fieldCount?: number;
  sensitivityLevels?: string[];
  objectCount?: number;
  matchedField?: string;
  matchedValue?: string;
  matchType?: 'fieldName' | 'fieldValue' | 'name';
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

/** Split a text into segments and bold the ones that match the query (case-insensitive). */
const SENSITIVITY_ORDER: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

function sortSensitivityLevels(levels: string[]): SensitivityLevel[] {
  return levels
    .filter((lvl): lvl is SensitivityLevel => SENSITIVITY_ORDER.includes(lvl as SensitivityLevel))
    .sort((a, b) => SENSITIVITY_ORDER.indexOf(a) - SENSITIVITY_ORDER.indexOf(b));
}

function Highlight({ text, query }: { text: string; query: string }) {
  if (!query.trim()) return <>{text}</>;
  const lowerQuery = query.toLowerCase();
  const lowerText = text.toLowerCase();
  const parts: React.ReactNode[] = [];
  let i = 0;
  while (i < text.length) {
    const idx = lowerText.indexOf(lowerQuery, i);
    if (idx === -1) {
      parts.push(text.slice(i));
      break;
    }
    if (idx > i) parts.push(text.slice(i, idx));
    parts.push(
      <mark
        key={`${idx}-${query}`}
        style={{
          fontWeight: 700,
          color: 'var(--accent-primary)',
          background: 'transparent',
        }}
      >
        {text.slice(idx, idx + query.length)}
      </mark>,
    );
    i = idx + query.length;
  }
  return <>{parts}</>;
}

export function SearchPopover({ onClose }: SearchPopoverProps) {
  const navigate = useNavigate();
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const activeCustomPages = customPages.filter((p) => !p.deletedAt);
  const { onError } = useToastError();
  const { t } = useTranslation(['common', 'navigation', 'settings', 'sensitivity', 'editor']);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchItem[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const [recent, setRecent] = useState<string[]>(loadRecent);
  const [selectedFilter, setSelectedFilter] = useState<string | null>(null);
  const [detailObjectId, setDetailObjectId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const filterBarRef = useRef<HTMLDivElement>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, [onClose]);

  const doSearch = useCallback(
    async (q: string, filter: string | null) => {
      if (!accountId || (!q.trim() && !filter)) {
        setResults([]);
        setHasSearched(false);
        return;
      }
      setIsSearching(true);
      setHasSearched(true);
      try {
        const isCustom = activeCustomPages.some((p) => p.id === filter);
        const payload: Record<string, unknown> = { accountId, query: q, limit: 50 };
        if (filter) {
          if (isCustom) {
            payload.parentId = filter;
          } else {
            payload.collectionType = filter;
          }
        }
        const res = await invoke<{ items: SearchItem[]; total: number; hasMore: boolean }>(
          'search_unified',
          payload,
        );
        setResults(res.items);
      } catch (e) {
        onError(e, t('common:search_failed'));
      } finally {
        setIsSearching(false);
      }
    },
    [accountId, activeCustomPages, onError, t],
  );

  const handleChange = (val: string) => {
    setQuery(val);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => doSearch(val, selectedFilter), DEBOUNCE_DELAY_MS);
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

  const resolvePageName = (item: SearchItem): string => {
    if (item.itemType === 'page') {
      const system = FILTER_PAGES.find((f) => f.key === item.objectId);
      if (system) return t(system.labelKey);
      const cp = customPages.find((p) => p.id === item.objectId);
      return cp?.name || item.name;
    }
    // Object result: show its page
    if (item.parentId) {
      const cp = customPages.find((p) => p.id === item.parentId);
      if (cp) return cp.name;
    }
    const system = FILTER_PAGES.find((f) => f.key === item.collectionType);
    if (system) return t(system.labelKey);
    return item.collectionType;
  };

  const resolveFieldLabel = (fieldPath?: string): string => {
    if (!fieldPath) return '';
    const lastSegment = fieldPath.split('.').pop() || fieldPath;
    return t(`editor:fields.${lastSegment}`, lastSegment);
  };

  const renderMatchHint = (item: SearchItem): React.ReactNode => {
    if (!item.matchedField || item.itemType === 'page' || item.matchType === 'name') return null;
    const fieldLabel = resolveFieldLabel(item.matchedField);
    if (item.matchType === 'fieldName' && item.matchedValue) {
      return (
        <span>
          {' · '}
          {t('settings:search_field_label', '字段名')}：
          <Highlight text={fieldLabel} query={query} />
        </span>
      );
    }
    if (item.matchType === 'fieldValue' && item.matchedValue) {
      return (
        <span>
          {' · '}
          <Highlight text={fieldLabel} query={query} />
          {': '}
          <Highlight text={item.matchedValue} query={query} />
        </span>
      );
    }
    return null;
  };

  const handleClickResult = (item: SearchItem) => {
    if (query.trim()) saveRecent(query.trim());
    if (item.itemType === 'page') {
      onClose();
      if (item.collectionType === 'page') {
        navigate(`/workspace/custom/${item.objectId}`);
      } else {
        navigate(`/workspace?section=${item.objectId}`);
      }
    } else {
      // Open object detail modal directly without leaving the current page
      setDetailObjectId(item.objectId);
    }
  };

  const handleRecentClick = (q: string) => {
    setQuery(q);
    doSearch(q, selectedFilter);
  };

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) onClose();
  };

  const showDefaultView = !hasSearched || (query.trim() === '' && !selectedFilter);

  const handleFilterWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    const el = filterBarRef.current;
    if (!el) return;
    if (el.scrollWidth <= el.clientWidth) return;
    // 垂直滚轮转为横向滚动，提升鼠标用户体验
    if (e.deltaY !== 0) {
      e.preventDefault();
      el.scrollLeft += e.deltaY;
    }
  };

  return createPortal(
    <>
      <div className={styles.backdrop} onClick={handleBackdropClick}>
        <div className={styles.card}>
          {/* 1. Search input */}
          <div className={styles.inputRow}>
            <div className={styles.leftControl}>
              <div className={styles.iconWrap}>
                <Search
                  size={ICON_SIZE.md}
                  className={`${styles.inputIcon} ${isSearching ? styles.iconHidden : ''}`}
                />
                <Loader2
                  size={ICON_SIZE.md}
                  className={`${styles.spinner} ${isSearching ? styles.spinnerVisible : ''}`}
                />
              </div>
            </div>
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
            {query && (
              <button
                className={styles.clearBtn}
                onClick={() => {
                  setQuery('');
                  setResults([]);
                  setHasSearched(false);
                  if (timeoutRef.current) clearTimeout(timeoutRef.current);
                }}
                aria-label={t('common:clear')}
                tabIndex={-1}
              >
                <X size={ICON_SIZE.sm} />
              </button>
            )}
          </div>

          {/* 2. Filter bar */}
          <div ref={filterBarRef} className={styles.filterBar} onWheel={handleFilterWheel}>
            {FILTER_PAGES.map((f) => {
              const Icon = f.icon;
              const active = selectedFilter === f.key;
              return (
                <button
                  key={f.key}
                  className={`${styles.filterBtn} ${active ? styles.filterBtnActive : ''}`}
                  onClick={() => handleFilter(f.key)}
                >
                  <Icon size={ICON_SIZE.xs} />
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
            {hasSearched && (
              <>
                {results.length === 0 && (
                  <div
                    className={`${styles.empty} ${
                      isSearching ? styles.emptyHidden : styles.emptyVisible
                    }`}
                  >
                    {t('common:no_results')}
                  </div>
                )}
                {results.map((item) => (
                  <button
                    key={`${item.itemType}-${item.objectId}`}
                    className={styles.resultItem}
                    onClick={() => handleClickResult(item)}
                  >
                    <PAGE_ICON_MAP.custom size={16} />
                    <div className={styles.resultText}>
                      <div className={styles.resultName}>{item.name}</div>
                      <div className={styles.resultMeta}>
                        {item.itemType === 'page' ? (
                          <>
                            <span className={styles.typeTag}>{t('settings:search_type_page')}</span>
                            <span> · {resolvePageName(item)}</span>
                            {item.objectCount !== undefined && (
                              <span>
                                {' '}
                                · {item.objectCount} {t('settings:search_objects_count')}
                              </span>
                            )}
                          </>
                        ) : (
                          <>
                            <span className={styles.typeTag}>
                              {t('settings:search_type_object')}
                            </span>
                            <span> · {resolvePageName(item)}</span>
                            {item.fieldCount !== undefined && (
                              <span>
                                {' '}
                                · {item.fieldCount} {t('settings:search_fields_count')}
                              </span>
                            )}
                            {item.sensitivityLevels && item.sensitivityLevels.length > 0 && (
                              <span
                                style={{
                                  display: 'inline-flex',
                                  alignItems: 'center',
                                  gap: 4,
                                  marginLeft: 4,
                                }}
                              >
                                {' · '}
                                {sortSensitivityLevels(item.sensitivityLevels).map((lvl) => (
                                  <SensitivityBadge key={lvl} level={lvl} />
                                ))}
                              </span>
                            )}
                          </>
                        )}
                        {renderMatchHint(item)}
                      </div>
                    </div>
                  </button>
                ))}
              </>
            )}

            {/* Default view when no search active */}
            {showDefaultView && recent.length > 0 && (
              <div className={styles.section}>
                <div className={styles.sectionTitle}>
                  <Clock size={ICON_SIZE.xs} />
                  <span>{t('common:recent_searches')}</span>
                </div>
                {recent.map((q) => (
                  <button
                    key={q}
                    className={styles.recentItem}
                    onClick={() => handleRecentClick(q)}
                  >
                    <Search size={ICON_SIZE.xs} />
                    <span>{q}</span>
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Settings — always pinned to the bottom of the card */}
          <div className={styles.footer}>
            <button
              className={styles.settingsItem}
              onClick={() => {
                onClose();
                navigate('/settings');
              }}
            >
              <Settings size={ICON_SIZE.md} />
              <span>{t('navigation:settings')}</span>
            </button>
          </div>
        </div>
      </div>

      {detailObjectId && (
        <ObjectDetailModal
          objectId={detailObjectId}
          onClose={() => setDetailObjectId(null)}
          onEdit={() => {
            setDetailObjectId(null);
            onClose();
            navigate(`/editor/${detailObjectId}`);
          }}
        />
      )}
    </>,
    document.body,
  );
}
