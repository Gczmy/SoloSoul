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

/** Detect if the query matches a translated system page name.
 *  Returns the English page key if matched, null otherwise. */
function matchPageTranslation(query: string, t: TFunction): string | null {
  const q = query.toLowerCase().trim();
  for (const key of SYSTEM_PAGE_KEYS) {
    const label = t(`navigation:${key}`).toLowerCase();
    if (label === q || label.includes(q) || q.includes(label)) {
      return key;
    }
  }
  return null;
}

/** Resolve display name for a search result item.
 *  System pages return English key from backend — translate via i18n.
 *  Custom pages and objects use their stored name directly. */
function resolveResultName(
  item: { itemType?: string; objectId: string; name: string },
  customPages: CustomPage[],
  t: TFunction,
): string {
  if (item.itemType === 'page') {
    // System page — translate via navigation namespace
    const systemKey = (SYSTEM_PAGE_KEYS as readonly string[]).includes(item.objectId)
      ? item.objectId
      : null;
    if (systemKey) {
      return t(`navigation:${systemKey}`);
    }
    // Custom page — use stored name
    const cp = customPages.find((p) => p.id === item.objectId);
    if (cp) return cp.name;
  }
  return item.name;
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
  /** "object" | "page" | "template" */
  itemType?: string;
  parentId?: string;
  objectCount?: number;
  fieldCount?: number;
  matchedField?: string;
  matchedValue?: string;
  matchType?: 'fieldName' | 'fieldValue' | 'name' | 'template';
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
        // Search with original query (no modification) to find matching objects
        const payload: Record<string, unknown> = { accountId, query: q, limit: 50 };

        // If query matches a translated page name, also filter by collectionType
        // so objects in that page are found (e.g. searching "身份" with collectionType "identity")
        const pageKey = matchPageTranslation(q, t);
        if (pageKey) {
          payload.collectionType = pageKey;
        }

        const res = await invoke<{ items: SearchItem[]; total: number; hasMore: boolean }>(
          'search_unified',
          payload,
        );

        let items = res.items;

        // If a page was matched, prepend a synthetic page result so the user sees it
        if (pageKey) {
          const pageExists = items.some((i) => i.itemType === 'page' && i.objectId === pageKey);
          if (!pageExists) {
            items = [
              {
                objectId: pageKey,
                name: pageKey,
                collectionType: pageKey,
                itemType: 'page',
                objectCount: undefined,
                matchedField: undefined,
                matchedValue: undefined,
                matchType: undefined,
                sensitivityLevels: undefined,
                relevance: 99,
              } as SearchItem,
              ...items,
            ];
          }
        }

        setResults(items);
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
    if (item.matchType === 'template' && item.matchedValue) {
      return (
        <span>
          {' · '}
          {t('settings:search_type_template')}：<Highlight text={item.matchedValue} query={query} />
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
    } else if (item.itemType === 'template') {
      navigate('/settings/templates');
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
                      <div style={{ fontSize: 'var(--text-body)', fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{isPage || item.itemType === 'template' || item.matchType === 'template' ? <Highlight text={resolveResultName(item, customPages, t)} query={query} /> : resolveResultName(item, customPages, t)}</div>
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
                        ) : item.itemType === 'template' ? (
                          <span>{t('settings:search_type_template', 'Template')}</span>
                        ) : item.matchType === 'template' ? (
                          <Highlight text={resolveCollectionLabel(item.collectionType, customPages, t)} query={query} />
                        ) : (
                          <span>{resolveCollectionLabel(item.collectionType, customPages, t)}</span>
                        )}
                        {isPage && item.objectCount !== undefined && (
                          <span>
                            {' · '}{item.objectCount} {t('settings:search_objects_count')}
                          </span>
                        )}
                        {item.itemType === 'template' && item.fieldCount !== undefined && (
                          <span>
                            {' · '}{item.fieldCount} {t('settings:search_fields_count')}
                          </span>
                        )}
                        {!isPage && item.itemType !== 'template' && item.sensitivityLevels && item.sensitivityLevels.length > 0 && (
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
