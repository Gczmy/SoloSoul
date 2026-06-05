import { useState, useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore, ObjectSummary } from '@/stores/objectStore';

const categories = [
  { type: 'identity', label: 'Identity', icon: '🆔' },
  { type: 'travel', label: 'Travel', icon: '🛂' },
  { type: 'financial', label: 'Financial', icon: '💰' },
  { type: 'professional', label: 'Professional', icon: '💼' },
];

export function ObjectWorkspacePage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const sectionFilter = searchParams.get('section') || '';
  const [searchQuery, setSearchQuery] = useState('');

  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { objects, loadObjects, isLoading, error } = useObjectStore();

  useEffect(() => {
    if (accountId) {
      loadObjects(accountId, sectionFilter ? { collectionType: sectionFilter } : undefined);
    }
  }, [accountId, sectionFilter]);

  const filtered = objects.filter((obj) =>
    (obj.name.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  return (
    <AppShell
      title={sectionFilter ? `${categories.find(c => c.type === sectionFilter)?.label || 'Objects'}` : 'Objects'}
      actions={
        <button
          onClick={() => navigate('/editor')}
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
        {/* Category tabs */}
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
              <span>{cat.icon}</span> {cat.label}
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
        ) : filtered.length === 0 ? (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: '24px 0' }}>
              {searchQuery ? 'No matching objects' : 'No objects yet'}
            </p>
          </Card>
        ) : (
          filtered.map((obj) => (
            <Card
              key={obj.id}
              interactive
              onClick={() => navigate(`/editor/${obj.id}`)}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <span style={{ fontSize: 24 }}>📄</span>
                <span style={{ fontSize: 14, fontWeight: 500 }}>{obj.name}</span>
              </div>
            </Card>
          ))
        )}
      </div>
    </AppShell>
  );
}
