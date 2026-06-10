import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useSensitivityStore, SensitivityLevel } from '@/stores/sensitivityStore';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';

export function SensitivitySettingsPage() {
  const navigate = useNavigate();
  const { map, log, loadMap, updateField, loadLog, isLoading } = useSensitivityStore();
  const [editingField, setEditingField] = useState<string | null>(null);
  const [newLevel, setNewLevel] = useState<SensitivityLevel>('internal');
  const [password, setPassword] = useState('');
  const [reason, setReason] = useState('');
  const { t } = useTranslation(['sensitivity', 'settings', 'common']);
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
    <AppShell title={t('settings:sensitivity_settings')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 600, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 4 }}>
          {t('settings:sensitivity_desc')}
        </p>

        {/* Level legend */}
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {(['public', 'internal', 'sensitive', 'critical'] as SensitivityLevel[]).map((l) => (
            <SensitivityBadge key={l} level={l} />
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
                  <SensitivityBadge level={level} />
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
                      <option value="public">{t('sensitivity:public')}</option>
                      <option value="internal">{t('sensitivity:internal')}</option>
                      <option value="sensitive">{t('sensitivity:sensitive')}</option>
                      <option value="critical">{t('sensitivity:critical')}</option>
                    </select>
                    <SecurePasswordInput
                      value={password}
                      onChange={(v) => setPassword(v)}
                      placeholder={t('common:password_placeholder')}
                    />
                    <input
                      value={reason}
                      onChange={(e) => setReason(e.target.value)}
                      placeholder={t('sensitivity:reason_placeholder')}
                      style={{
                        padding: '6px 10px', borderRadius: 6, border: '1px solid var(--border-subtle)',
                        fontSize: 13, background: 'var(--bg-elevated)',
                      }}
                    />
                    <div style={{ display: 'flex', gap: 4 }}>
                      <Button size="sm" onClick={() => handleUpdate(fieldId)} loading={isLoading}>{t('common:save')}</Button>
                      <Button size="sm" variant="secondary" onClick={() => setEditingField(null)}>{t('common:cancel')}</Button>
                    </div>
                  </div>
                ) : (
                  <Button size="sm" variant="secondary" onClick={() => {
                    setEditingField(fieldId);
                    setNewLevel(level);
                  }}>
                    {t('sensitivity:change_level')}
                  </Button>
                )}
              </div>
            </Card>
          ))}
        </div>

        {/* Change history */}
        {log.length > 0 && (
          <Card>
            <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>{t('sensitivity:recent_changes')}</h3>
            {log.slice(0, 5).map((entry, i) => (
              <div key={i} style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 4 }}>
                <span style={{ color: 'var(--text-primary)' }}>{entry.field_id}</span>:
                {t(`sensitivity:${entry.old_level}`)} → {t(`sensitivity:${entry.new_level}`)}
                <span style={{ marginLeft: 8 }}>({entry.reason || t('sensitivity:no_reason', { defaultValue: 'no reason' })})</span>
              </div>
            ))}
          </Card>
        )}
      </div>
    </AppShell>
  );
}
