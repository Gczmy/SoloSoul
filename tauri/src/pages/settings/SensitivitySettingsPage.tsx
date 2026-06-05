import { useState, useEffect } from 'react';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useSensitivityStore, SensitivityLevel } from '@/stores/sensitivityStore';

const levelColors: Record<SensitivityLevel, string> = {
  public: '#27ae60',
  internal: '#5B7C99',
  sensitive: '#C4925C',
  critical: '#e74c3c',
};

export function SensitivitySettingsPage() {
  const { map, log, loadMap, updateField, loadLog, isLoading } = useSensitivityStore();
  const [editingField, setEditingField] = useState<string | null>(null);
  const [newLevel, setNewLevel] = useState<SensitivityLevel>('internal');
  const [password, setPassword] = useState('');
  const [reason, setReason] = useState('');

  useEffect(() => {
    loadMap();
    loadLog(50);
  }, []);

  const handleUpdate = async (fieldId: string) => {
    await updateField(fieldId, newLevel, password, reason || undefined);
    setEditingField(null);
    setPassword('');
    setReason('');
    await loadMap();
    await loadLog(50);
  };

  const sortedEntries = map ? Object.entries(map.entries).sort(([, a], [, b]) => {
    const order: SensitivityLevel[] = ['critical', 'sensitive', 'internal', 'public'];
    return order.indexOf(a) - order.indexOf(b);
  }) : [];

  return (
    <AppShell title="Sensitivity Settings" onBack={() => window.history.back()}>
      <div style={{ maxWidth: 600, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 4 }}>
          Manage field-level sensitivity. Downgrading a field's protection requires password verification.
          Changes are audited and cannot be reverted without trace.
        </p>

        {/* Level legend */}
        <div style={{ display: 'flex', gap: 16, fontSize: 12 }}>
          {(['public', 'internal', 'sensitive', 'critical'] as SensitivityLevel[]).map((l) => (
            <span key={l} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
              <span style={{ width: 8, height: 8, borderRadius: 4, background: levelColors[l] }} />
              {l}
            </span>
          ))}
        </div>

        {/* Field list */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {sortedEntries.map(([fieldId, level]) => (
            <Card key={fieldId}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12 }}>
                <div>
                  <div style={{ fontSize: 13, fontWeight: 500, fontFamily: 'var(--font-mono, monospace)' }}>
                    {fieldId}
                  </div>
                  <span style={{
                    fontSize: 11, padding: '2px 8px', borderRadius: 4,
                    background: `${levelColors[level]}20`,
                    color: levelColors[level], fontWeight: 600,
                  }}>
                    {level}
                  </span>
                </div>

                {editingField === fieldId ? (
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 8, minWidth: 200 }}>
                    <select
                      value={newLevel}
                      onChange={(e) => setNewLevel(e.target.value as SensitivityLevel)}
                      style={{
                        padding: '6px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)',
                        fontSize: 13, background: 'var(--bg-elevated)',
                      }}
                    >
                      <option value="public">Public</option>
                      <option value="internal">Internal</option>
                      <option value="sensitive">Sensitive</option>
                      <option value="critical">Critical</option>
                    </select>
                    <input
                      type="password"
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      placeholder="Password (required for downgrade)"
                      style={{
                        padding: '6px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)',
                        fontSize: 13, background: 'var(--bg-elevated)',
                      }}
                    />
                    <input
                      value={reason}
                      onChange={(e) => setReason(e.target.value)}
                      placeholder="Reason (optional, for audit)"
                      style={{
                        padding: '6px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)',
                        fontSize: 13, background: 'var(--bg-elevated)',
                      }}
                    />
                    <div style={{ display: 'flex', gap: 4 }}>
                      <Button size="sm" onClick={() => handleUpdate(fieldId)} loading={isLoading}>Save</Button>
                      <Button size="sm" variant="secondary" onClick={() => setEditingField(null)}>Cancel</Button>
                    </div>
                  </div>
                ) : (
                  <Button size="sm" variant="secondary" onClick={() => {
                    setEditingField(fieldId);
                    setNewLevel(level);
                  }}>
                    Change
                  </Button>
                )}
              </div>
            </Card>
          ))}
        </div>

        {/* Change history */}
        {log.length > 0 && (
          <Card>
            <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>Recent Changes</h3>
            {log.slice(0, 5).map((entry, i) => (
              <div key={i} style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 4 }}>
                <span style={{ color: 'var(--text-primary)' }}>{entry.field_id}</span>:
                {entry.old_level} → {entry.new_level}
                <span style={{ marginLeft: 8 }}>({entry.reason || 'no reason'})</span>
              </div>
            ))}
          </Card>
        )}
      </div>
    </AppShell>
  );
}
