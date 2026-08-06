import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { ICON_SIZE } from '@/lib/constants';
import { Search, Info, Type, FolderOpen } from 'lucide-react';
import { ObjectDetailModal } from '@/components/object/ObjectDetailModal';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { resolveCollectionLabel } from '@/lib/utils';
import { useSettingsStore } from '@/stores/settingsStore';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import { searchCache } from '@/lib/searchCache';
import {
  Highlight,
  MatchHint,
  SearchItem,
  buildSearchCacheParams,
  buildSearchPayload,
  ensurePageResultExists,
  matchPageTranslation,
  resolveResultIcon,
  resolveResultName,
  sortSensitivityLevels,
} from '@/lib/searchShared';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import styles from './SearchPage.module.css';
import { PageGuideButton } from '@/components/guide/PageGuideButton';

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

      const pageKey = matchPageTranslation(q, t);
      const { cacheKey } = buildSearchCacheParams(accountId, q, pageKey, null, customPages);
      const cached = searchCache.get<SearchItem[]>(cacheKey);
      if (cached) {
        setResults(cached);
        setHasSearched(true);
        return;
      }

      setIsSearching(true);
      setHasSearched(true);
      try {
        const res = await invoke<{ items: SearchItem[]; total: number; hasMore: boolean }>(
          'search_unified',
          buildSearchPayload(accountId, q, pageKey, null, customPages),
        );

        const items = pageKey ? ensurePageResultExists(res.items, pageKey) : res.items;

        searchCache.set(cacheKey, items);
        setResults(items);
      } catch (e) {
        onError(e, t('common:search_failed'));
      } finally {
        setIsSearching(false);
      }
    },
    [accountId, onError, t, customPages],
  );

  const handleChange = (val: string) => {
    setQuery(val);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => doSearch(val), DEBOUNCE_DELAY_MS);
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

  const searchGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_search_title', { defaultValue: 'Search Guide' }),
        steps: [
          {
            icon: Search,
            title: t('common:guide_search_step1_title', { defaultValue: 'Enter Keywords' }),
            description:
              t('common:guide_search_step1_desc', { defaultValue: 'Type keywords to search across objects, fields, and attachments. Use quoted phrases for exact matches.' }),
          },
          {
            icon: Type,
            title: t('common:guide_search_step2_title', { defaultValue: 'Filter Results' }),
            description:
              t('common:guide_search_step2_desc', { defaultValue: 'Filter results by sensitivity, object type, or date to narrow down the matches.' }),
          },
          {
            icon: FolderOpen,
            title: t('common:guide_search_step3_title', { defaultValue: 'Open Objects' }),
            description:
              t('common:guide_search_step3_desc', { defaultValue: 'Tap a result to open the object detail. You can edit or copy values from the detail view.' }),
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_search', { defaultValue: 'Global Search' }),
            description:
              t('common:guide_help_search_desc', { defaultValue: 'Search objects, fields, and attachments across the vault' }),
            href: '/help?id=search',
          },
        ],
      },
    ],
    [t],
  );

  return (
    <AppShell
      title={t('navigation:search')}
      onBack={() => navigate('/home')}
      actions={<PageGuideButton pages={searchGuidePages} />}
    >
      <PageContainer variant="small" gap="default">
        <Input
          placeholder={t('common:search_placeholder')}
          value={query}
          onChange={(e) => handleChange(e.target.value)}
          onClear={() => {
            setQuery('');
            setResults([]);
            setHasSearched(false);
          }}
          autoFocus
          prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
        />

        <div style={{ marginTop: 12 }}>
          {hasSearched && results.length === 0 && !isSearching && (
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
                <Card key={item.objectId} interactive onClick={() => handleClickResult(item)}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                    <span style={{ flexShrink: 0, display: 'flex' }}>
                      <ResultIcon size={18} />
                    </span>
                    <div style={{ overflow: 'hidden' }}>
                      <div className={styles.resultTitle}>
                        {isPage || item.itemType === 'template' || item.matchType === 'template' ? (
                          <Highlight text={resolveResultName(item, customPages, t)} query={query} />
                        ) : (
                          resolveResultName(item, customPages, t)
                        )}
                      </div>
                      <div className={styles.resultMeta}>
                        {isPage ? (
                          <span>{t('settings:search_type_page')}</span>
                        ) : item.itemType === 'template' ? (
                          <span>{t('settings:search_type_template', 'Template')}</span>
                        ) : item.matchType === 'template' ? (
                          <Highlight
                            text={resolveCollectionLabel(item.collectionType, customPages, t)}
                            query={query}
                          />
                        ) : (
                          <span>{resolveCollectionLabel(item.collectionType, customPages, t)}</span>
                        )}
                        {!isPage && item.itemType !== 'template' && item.templateName && (
                          <span>
                            {' · '}
                            <span
                              style={
                                item.templateDeleted
                                  ? { textDecoration: 'line-through' }
                                  : undefined
                              }
                            >
                              {item.templateName}
                            </span>
                          </span>
                        )}
                        {isPage && item.objectCount !== undefined && (
                          <span>
                            {' · '}
                            {item.objectCount} {t('settings:search_objects_count')}
                          </span>
                        )}
                        {item.itemType === 'template' && item.fieldCount !== undefined && (
                          <span>
                            {' · '}
                            {item.fieldCount} {t('settings:search_fields_count')}
                          </span>
                        )}
                        {!isPage &&
                          item.itemType !== 'template' &&
                          item.sensitivityLevels &&
                          item.sensitivityLevels.length > 0 && (
                            <>
                              {' · '}
                              {sortSensitivityLevels(item.sensitivityLevels).map((lvl) => (
                                <SensitivityBadge key={lvl} level={lvl} />
                              ))}
                            </>
                          )}
                        <MatchHint item={item} query={query} t={t} />
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
