import { useState, useCallback, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { ObjectDetailModal } from '@/components/object/ObjectDetailModal';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import { resolveCollectionLabel } from '@/lib/pageLabels';
import { useSettingsStore } from '@/stores/settingsStore';
import { SensitivityBadge, SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';

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

interface SearchItem {
  objectId: string;
  name: string;
  collectionType: string;
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
        const res = await invoke<{ items: SearchItem[]; total: number; hasMore: boolean }>(
          'search_unified',
          { accountId, query: q, limit: 50 },
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
    if (!item.matchedField || item.matchType === 'name') return null;
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

  return (
    <AppShell title={t('navigation:search')} onBack={() => navigate('/home')}>
      <div style={{ maxWidth: 600, margin: '0 auto' }}>
        <Input
          placeholder={t('common:search_placeholder')}
          value={query}
          onChange={(e) => handleChange(e.target.value)}
          autoFocus
        />

        <div style={{ marginTop: 12 }}>
          {isSearching && (
            <p style={{ fontSize: 13, color: 'var(--text-tertiary)', textAlign: 'center' }}>
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

          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {results.map((item) => (
              <Card
                key={item.objectId}
                interactive
                onClick={() => setDetailObjectId(item.objectId)}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                  <PAGE_ICON_MAP.custom size={18} />
                  <div>
                    <div style={{ fontSize: 14, fontWeight: 500 }}>{item.name}</div>
                    <div
                      style={{
                        fontSize: 11,
                        color: 'var(--text-tertiary)',
                        display: 'flex',
                        alignItems: 'center',
                        gap: 4,
                        flexWrap: 'wrap',
                      }}
                    >
                      <span>{resolveCollectionLabel(item.collectionType, customPages, t)}</span>
                      {item.sensitivityLevels && item.sensitivityLevels.length > 0 && (
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
            ))}
          </div>
        </div>
      </div>

      {/* Object detail modal */}
      {detailObjectId && (
        <ObjectDetailModal
          objectId={detailObjectId}
          onClose={() => setDetailObjectId(null)}
          onEdit={() => {
            setDetailObjectId(null);
            navigate(`/editor/${detailObjectId}`);
          }}
          onAttachments={() => {
            setAttachmentObjId(detailObjectId);
            setDetailObjectId(null);
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
