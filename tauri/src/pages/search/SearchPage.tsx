import { useState, useCallback, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import type { LucideIcon } from 'lucide-react';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { ICON_SIZE } from '@/lib/iconSizes';
import { Search } from 'lucide-react';
import { ObjectDetailModal } from '@/components/object/ObjectDetailModal';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { resolveCollectionLabel } from '@/lib/pageLabels';
import { useSettingsStore } from '@/stores/settingsStore';
import type { CustomPage } from '@/stores/settingsStore';
import { SensitivityBadge, SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';

const SENSITIVITY_ORDER: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];
const SYSTEM_PAGE_KEYS = ['identity', 'travel', 'financial', 'professional'] as const;

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

/** Build search query with i18n page name detection.
 *  If the query matches a translated system page name, append the English page key
 *  so the backend can match it via search_pages. */
function buildSearchQuery(query: string, t: TFunction): string {
  const q = query.toLowerCase().trim();
  const extraKeys: string[] = [];
  for (const key of SYSTEM_PAGE_KEYS) {
    const label = t(`navigation:${key}`).toLowerCase();
    if (label === q || label.includes(q) || q.includes(label)) {
      extraKeys.push(key);
    }
  }
  if (extraKeys.length > 0) {
    return `${query} ${extraKeys.join(' ')}`;
  }
  return query;
}

/** Resolve icon for a search result item based on its type and collection. */
function resolveResultIcon(
  item: { itemType?: string; collectionType: string; objectId: string },
  customPages: CustomPage[],
): LucideIcon {
  if (item.itemType === 'page') {
    // System page — check PAGE_ICON_MAP by objectId (e.g. "travel" → Plane)
    if (item.objectId in PAGE_ICON_MAP) {
      return PAGE_ICON_MAP[item.objectId as keyof typeof PAGE_ICON_MAP];
    }
    // Custom page — look up its iconId
    const cp = customPages.find((p) => p.id === item.objectId);
    if (cp) {
      return resolveCustomIcon(cp.iconId);
    }
    return PAGE_ICON_MAP.custom;
  }

  // Object — use collectionType to determine icon
  if (item.collectionType in PAGE_ICON_MAP) {
    return PAGE_ICON_MAP[item.collectionType as keyof typeof PAGE_ICON_MAP];
  }
  // Check if collectionType is a custom page ID
  const cp = customPages.find((p) => p.id === item.collectionType);
  if (cp) {
    return resolveCustomIcon(cp.iconId);
  }
  return PAGE_ICON_MAP.custom;
}

interface SearchItem {
  objectId: string;
  name: string;
  collectionType: string;
  itemType?: string; // "object" | "page" — from backend, optional for backward compat
  parentId?: string;
  objectCount?: number;
  matchedField?: string;
  matchedValue?: string;
  matchType?: 'fieldName' | 'fieldValue' | 'name';
  sensitivityLevels?: string[];
  relevance: number;
}

export function SearchPage() {
  const navigate = useNavigate();
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { onError } = useToastError();
  const { t } = useTranslation(['common', 'navigation', 'settings', 'editor']);
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchItem[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  // Object detail + attachment state
  const [detailObjectId, setDetailObjectId] = useState<string | null>(null);
  const [attachmentObjId, setAttachmentObjId] = useState<string | null>(null);

  const doSearch = useCallback(
    async (q: string) => {
      if (!accountId || !q.trim()) {
        setResults([]);
        setHasSearched(false);
        return;
      }
      setIsSearching(true);
      setHasSearched(true);
      try {
        const enhancedQuery = buildSearchQuery(q, t);
        const res = await invoke<{ items: SearchItem[]; total: number; hasMore: boolean }>(
          'search_unified',
          { accountId, query: enhancedQuery, limit: 50 },
        );
        setResults(res.items);
      } catch (e) {
        onError(e, t('common:search_failed'));
      } finally {
        setIsSearching(false);
      }
    },
    [accountId, onError, t],
  );

  const handleChange = (val: string) => {
    setQuery(val);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => doSearch(val), DEBOUNCE_DELAY_MS);
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
    if (item.itemType === 'page') {
      if (item.collectionType === 'page') {
        navigate(`/workspace/custom/${item.objectId}`);
      } else {
        navigate(`/workspace?section=${item.objectId}`);
      }
    } else {
      setDetailObjectId(item.objectId);
    }
  };

  return (
    <AppShell title={t('navigation:search')} onBack={() => navigate('/home')}>
      <PageContainer variant="small" gap="default">
        <Input
          placeholder={t('common:search_placeholder')}
          value={query}
          onChange={(e) => handleChange(e.target.value)}
          onClear={() => { setQuery(''); setResults([]); setHasSearched(false); }}
          autoFocus
          prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
        />

        <div style={{ marginTop: 12 }}>
          {isSearching && (
            <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-tertiary)', textAlign: 'center' }}>
              {t('common:searching')}
            </p>
          )}

          {!isSearching && hasSearched && results.length === 0 && (
            <Card>
              <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: '24px 0' }}>
                {t('common:no_results')}
              </p>
            </Card>
          )}

          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
            {results.map((item) => {
              const ResultIcon = resolveResultIcon(item, customPages);
              const isPage = item.itemType === 'page';
              return (
                <Card
                  key={item.objectId}
                  interactive
                  onClick={() => handleClickResult(item)}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                    <span style={{ flexShrink: 0, display: 'flex' }}>
                      <ResultIcon size={18} />
                    </span>
                    <div style={{ overflow: 'hidden' }}>
                      <div style={{ fontSize: 'var(--text-body)', fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{item.name}</div>
                      <div
                        style={{
                          fontSize: 'var(--text-badge)',
                          color: 'var(--text-tertiary)',
                          display: 'flex',
                          alignItems: 'center',
                          gap: 4,
                          flexWrap: 'wrap',
                        }}
                      >
                        {isPage ? (
                          <span>{t('settings:search_type_page')}</span>
                        ) : (
                          <span>{resolveCollectionLabel(item.collectionType, customPages, t)}</span>
                        )}
                        {isPage && item.objectCount !== undefined && (
                          <span>
                            {' · '}{item.objectCount} {t('settings:search_objects_count')}
                          </span>
                        )}
                        {!isPage && item.sensitivityLevels && item.sensitivityLevels.length > 0 && (
                          <>
                            {' · '}
                            {sortSensitivityLevels(item.sensitivityLevels).map((lvl) => (
                              <SensitivityBadge key={lvl} level={lvl} />
                            ))}
                          </>
                        )}
                        {renderMatchHint(item)}
                      </div>
                    </div>
                  </div>
                </Card>
              );
            })}
          </div>
        </div>
      </PageContainer>

      {/* Object detail modal — only for object results */}
      {detailObjectId && (
        <ObjectDetailModal
          objectId={detailObjectId}
          onClose={() => setDetailObjectId(null)}
          onEdit={() => {
            setDetailObjectId(null);
            navigate(`/editor/${detailObjectId}`);
          }}
        />
      )}

      {/* Attachment viewer — opened via detail modal's onAttachments */}
      {attachmentObjId && (
        <AttachmentViewer objectId={attachmentObjId} onClose={() => setAttachmentObjId(null)} />
      )}
    </AppShell>
  );
}
