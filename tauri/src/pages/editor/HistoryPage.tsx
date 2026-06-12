import { useState, useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Clock, RotateCcw, ChevronRight } from 'lucide-react';

interface SnapshotEntry {
  id: string;
  timestamp: number;
  triggeredBy: string;
  diffSummary: string;
}

export function HistoryPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const objectId = searchParams.get('objectId') || '';
  const [snapshots, setSnapshots] = useState<SnapshotEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [restoring, setRestoring] = useState<string | null>(null);

  useEffect(() => {
    if (objectId) {
      invoke<SnapshotEntry[]>('snapshot_list', { objectId })
        .then(setSnapshots)
        .finally(() => setLoading(false));
    }
  }, [objectId]);

  const handleRollback = async (snapshot: SnapshotEntry) => {
    if (!confirm(`Rollback to version from ${new Date(snapshot.timestamp).toLocaleString()}?`)) return;
    setRestoring(snapshot.id);
    try {
      await invoke('snapshot_rollback', { snapshotId: snapshot.id, objectId });
      navigate(-1);
    } catch (e) {
      alert(`Rollback failed: ${e}`);
    } finally {
      setRestoring(null);
    }
  };

  return (
    <AppShell title="History" onBack={() => navigate(-1)}>
      <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 8 }}>
        {loading ? (
          <Card><LoadingPlaceholder variant="elevated" minHeight={80} /></Card>
        ) : snapshots.length === 0 ? (
          <Card>
            <div style={{ textAlign: 'center', padding: 48 }}>
              <Clock size={40} style={{ marginBottom: 12, opacity: 0.25 }} />
              <p style={{ color: 'var(--text-secondary)', fontSize: 14 }}>No history yet. Changes will appear here.</p>
            </div>
          </Card>
        ) : (
          <>
            <p style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>{snapshots.length} version(s)</p>
            {snapshots.map((s, i) => (
              <Card key={s.id}>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  <div>
                    <div style={{ fontSize: 13, fontWeight: 500 }}>
                      {i === 0 ? 'Current' : new Date(s.timestamp).toLocaleString()}
                    </div>
                    <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                      {s.triggeredBy === 'user_edit' ? 'Edited' :
                       s.triggeredBy === 'rollback' ? 'Rolled back' :
                       s.diffSummary || s.triggeredBy}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: 6 }}>
                    {i > 0 && (
                      <Button size="sm" variant="secondary" onClick={() => handleRollback(s)} loading={restoring === s.id}>
                        <RotateCcw size={12} style={{ marginRight: 3 }} /> Restore
                      </Button>
                    )}
                    <ChevronRight size={16} style={{ color: 'var(--text-tertiary)', marginTop: 4 }} />
                  </div>
                </div>
              </Card>
            ))}
          </>
        )}
      </div>
    </AppShell>
  );
}
