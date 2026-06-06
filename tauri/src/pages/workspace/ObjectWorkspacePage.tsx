import { useState, useEffect } from 'react';
import { useNavigate, useSearchParams, useParams } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';

// Category tabs — each maps to a section-based workspace filter
// Icons sourced from PAGE_ICON_MAP — §7.4 Single Source of Truth
const categories = [
  { type: 'identity', label: 'Identity', icon: PAGE_ICON_MAP.profile },
  { type: 'travel', label: 'Travel', icon: PAGE_ICON_MAP.travel },
  { type: 'financial', label: 'Financial', icon: PAGE_ICON_MAP.financial },
  { type: 'professional', label: 'Professional', icon: PAGE_ICON_MAP.professional },
];

export function ObjectWorkspacePage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { pageId } = useParams(); // from /workspace/custom/:pageId
  const sectionFilter = searchParams.get('section') || '';
  const [searchQuery, setSearchQuery] = useState('');

  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { objects, loadObjects, isLoading, error } = useObjectStore();
  const customPages = useSettingsStore((s) => s.settings.customPages);

  // Find custom page name for title
  const customPage = pageId ? customPages.find((p) => p.id === pageId) : null;

  useEffect(() => {
    if (accountId) {
      // Custom pages filter by parentId; section pages filter by collectionType
      if (pageId) {
        loadObjects(accountId, { parentId: pageId });
      } else {
        loadObjects(accountId, sectionFilter ? { collectionType: sectionFilter } : undefined);
      }
    }
  }, [accountId, sectionFilter, pageId]);

  // Filter out page-type objects and apply local search
  const visibleObjects = objects.filter(
    (obj) =>
      obj.collectionType !== 'page' &&
      obj.collectionType !== 'unknown' &&
      obj.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const activeCategory = categories.find((c) => c.type === sectionFilter);

  // Build editor URL: pass section for section pages, parentId for custom pages
  const newObjectUrl = pageId
    ? `/editor?parentId=${pageId}`
    : `/editor${sectionFilter ? `?section=${sectionFilter}` : ''}`;

  return (
    <AppShell
      title={customPage?.name || activeCategory?.label || 'Objects'}
      actions={
        <button
          onClick={() => navigate(newObjectUrl)}
          style={{
            padding: '8px 16px', borderRadius: 8, border: 'none',
            background: 'var(--accent-primary)', color: 'white',
            fontSize: 13, fontWeight: 500, cursor: 'pointer',
          }}
        >
          + New
        </button>
      }
    >
      <div style={{ maxWidth: 640, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {/* Category tabs — only show on section pages, not custom pages */}
        {!pageId && (
          <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            {categories.map((cat) => (
              <button
                key={cat.type}
                onClick={() => navigate(`/workspace?section=${cat.type}`)}
                style={{
                  padding: '6px 14px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                  background: sectionFilter === cat.type ? 'var(--accent-primary)' : 'transparent',
                  color: sectionFilter === cat.type ? 'white' : 'var(--text-primary)',
                  fontSize: 13, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4,
                }}
              >
                <span><cat.icon size={16} /></span> {cat.label}
              </button>
            ))}
            {sectionFilter && (
              <button
                onClick={() => navigate('/workspace')}
                style={{
                  padding: '6px 14px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                  background: 'transparent', color: 'var(--text-tertiary)',
                  fontSize: 13, cursor: 'pointer',
                }}
              >
                Clear
              </button>
            )}
          </div>
        )}

        <Input
          placeholder="Search objects..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />

        {isLoading ? (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: '24px 0' }}>
              Loading...
            </p>
          </Card>
        ) : error ? (
          <Card>
            <p style={{ textAlign: 'center', color: '#e74c3c', padding: '24px 0' }}>
              {error}
            </p>
          </Card>
        ) : visibleObjects.length === 0 ? (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: '24px 0', fontSize: 14 }}>
              {searchQuery ? 'No matching objects' : 'No objects yet'}
            </p>
          </Card>
        ) : (
          visibleObjects.map((obj) => (
            <Card
              key={obj.id}
              interactive
              onClick={() => navigate(`/editor/${obj.id}`)}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <span><PAGE_ICON_MAP.custom size={24} /></span>
                <div>
                  <span style={{ fontSize: 14, fontWeight: 500 }}>{obj.name}</span>
                  <span style={{
                    fontSize: 11, color: 'var(--text-tertiary)', marginLeft: 8,
                    padding: '1px 6px', borderRadius: 4,
                    background: 'var(--bg-elevated)',
                  }}>
                    {obj.collectionType}
                  </span>
                </div>
              </div>
            </Card>
          ))
        )}
      </div>
    </AppShell>
  );
}
