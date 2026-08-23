import { useState, useCallback, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
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
  FileText,
} from 'lucide-react';
import { createPortal } from 'react-dom';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore, type CustomPage } from '@/stores/settingsStore';
import { useToastError } from '@/hooks/useToastError';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import { ObjectDetailModal } from '@/components/object/ObjectDetailModal';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import {
  Highlight,
  MatchHint,
  SearchItem,
  resolveResultIcon,
  resolveResultName,
  runUnifiedSearch,
  sortSensitivityLevels,
} from '@/lib/searchShared';
import styles from './SearchPopover.module.css';
import { ICON_SIZE } from '@/lib/constants';

const FILTER_PAGES = [
  { key: 'identity', labelKey: 'navigation:identity', icon: User },
  { key: 'travel', labelKey: 'navigation:travel', icon: Plane },
  { key: 'financial', labelKey: 'navigation:financial', icon: CreditCard },
  { key: 'professional', labelKey: 'navigation:professional', icon: Briefcase },
  { key: 'document', labelKey: 'navigation:document', icon: FileText },
];

interface SearchPopoverProps {
  onClose: () => void;
}

const RECENT_KEY_PREFIX = 'solosoul_recent_searches';

/** P005：按账户隔离存储最近搜索词；锁定/退出时由 authStore 统一清除。 */
function recentStorageKey(accountId?: string): string | null {
  return accountId ? `${RECENT_KEY_PREFIX}:${accountId}` : null;
}

function loadRecent(accountId?: string): string[] {
  const key = recentStorageKey(accountId);
  if (!key) return [];
  try {
    return JSON.parse(localStorage.getItem(key) || '[]');
  } catch {
    return [];
  }
}

function saveRecent(query: string, accountId?: string) {
  const key = recentStorageKey(accountId);
  if (!key) return;
  const prev = loadRecent(accountId);
  const next = [query, ...prev.filter((q) => q !== query)].slice(0, 3);
  localStorage.setItem(key, JSON.stringify(next));
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

  // P018: doSearch 收敛到 lib/searchShared 的 runUnifiedSearch（SearchPage 共用）
  const doSearch = useCallback(
    async (q: string, filter: string | null) => {
      await runUnifiedSearch({
        accountId,
        query: q,
        filter,
        customPages,
        t,
        onError,
        setResults,
        setHasSearched,
        setIsSearching,
      });
    },
    [accountId, customPages, onError, t],
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
      setRecent(loadRecent(accountId));
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
    const system = FILTER_PAGES.find((f) => f.key === item.typeId);
    if (system) return t(system.labelKey);
    return item.typeId;
  };

  // 直接跳转避免闪烁：全局已禁用路由 startTransition（App/index.tsx 的
  // <BrowserRouter useTransitions={false}>），navigate 与 onClose 同为紧急更新，
  // React 在同一事件回调内自动批处理为一次提交——新页面渲染与卡片卸载同帧完成，
  // 不会先露出底层页面。顺序先导航后关闭，保证新页面已就绪才移除弹层。
  const closeAndNavigate = (to: string) => {
    navigate(to);
    onClose();
  };

  const handleClickResult = (item: SearchItem) => {
    if (query.trim()) saveRecent(query.trim());
    if (item.itemType === 'page') {
      if (item.typeId === 'page') {
        closeAndNavigate(`/workspace/custom/${item.objectId}`);
      } else {
        closeAndNavigate(`/workspace?section=${item.objectId}`);
      }
    } else if (item.itemType === 'template') {
      closeAndNavigate('/settings/templates');
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

  // P027: 结果行抽为子组件（降 JSX 嵌套深度，resolvePageName/resolveResultName 经 props 传入）
  const renderResultRow = (item: SearchItem) => (
    <SearchResultRow
      key={`${item.itemType}-${item.objectId}`}
      item={item}
      query={query}
      customPages={customPages}
      t={t}
      resolvePageName={resolvePageName}
      onClick={() => handleClickResult(item)}
    />
  );

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
                {results.map(renderResultRow)}
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
              onClick={() => closeAndNavigate('/settings')}
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
            closeAndNavigate(`/editor/${detailObjectId}`);
          }}
        />
      )}
    </>,
    document.body,
  );
}

/**
 * P027: 搜索结果单行（从 SearchPopover 抽出，降低主组件 JSX 嵌套深度）。
 * 渲染图标 + 名称（含高亮）+ 元信息（页面/模板/对象三种类型 + 敏感度徽章 + 匹配提示）。
 */
function SearchResultRow({
  item,
  query,
  customPages,
  t,
  resolvePageName,
  onClick,
}: {
  item: SearchItem;
  query: string;
  customPages: CustomPage[];
  t: TFunction;
  resolvePageName: (item: SearchItem) => string;
  onClick: () => void;
}) {
  const ResultIcon = resolveResultIcon(item, customPages);
  return (
    <button className={styles.resultItem} onClick={onClick}>
      <ResultIcon size={16} />
      <div className={styles.resultText}>
        <div className={styles.resultName}>
          {item.itemType === 'page' ||
          item.itemType === 'template' ||
          item.matchType === 'template' ? (
            <Highlight text={resolveResultName(item, customPages, t)} query={query} />
          ) : (
            resolveResultName(item, customPages, t)
          )}
        </div>
        <div className={styles.resultMeta}>
          {item.itemType === 'page' ? (
            <>
              <span className={styles.typeTag}>{t('settings:search_type_page')}</span>
              <span> · {resolvePageName(item)}</span>
              {item.objectCount !== undefined && (
                <span>
                  {' '}· {item.objectCount} {t('settings:search_objects_count')}
                </span>
              )}
            </>
          ) : item.itemType === 'template' ? (
            <>
              <span className={styles.typeTag}>{t('settings:search_type_template', 'Template')}</span>
              {item.fieldCount !== undefined && (
                <span>
                  {' · '}
                  {item.fieldCount} {t('settings:search_fields_count')}
                </span>
              )}
            </>
          ) : (
            <>
              <span className={styles.typeTag}>{t('settings:search_type_object')}</span>
              <span>
                {' '}·{' '}
                {item.matchType === 'template' ? (
                  <Highlight text={resolvePageName(item)} query={query} />
                ) : (
                  resolvePageName(item)
                )}
              </span>
              {item.templateName && (
                <span>
                  {' · '}
                  <span
                    style={item.templateDeleted ? { textDecoration: 'line-through' } : undefined}
                  >
                    {item.templateName}
                  </span>
                </span>
              )}
              {item.fieldCount !== undefined && (
                <span>
                  {' '}· {item.fieldCount} {t('settings:search_fields_count')}
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
          <MatchHint item={item} query={query} t={t} />
        </div>
      </div>
    </button>
  );
}
