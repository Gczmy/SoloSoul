import { useState, useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { Clock, RotateCcw, ChevronRight } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

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
  const { t } = useTranslation(['common']);
  const showToast = useUiStore((s) => s.showToast);
  const { requestConfirm, dialog: confirmDialog } = useConfirm();

  useEffect(() => {
    if (objectId) {
      invoke<SnapshotEntry[]>('snapshot_list', { objectId })
        .then(setSnapshots)
        .finally(() => setLoading(false));
    }
  }, [objectId]);

  const handleRollback = (snapshot: SnapshotEntry) => {
    requestConfirm(
      t('common:rollback_confirm_title', 'Restore version'),
      t('common:rollback_confirm_body', { date: new Date(snapshot.timestamp).toLocaleString() }) ||
        `Rollback to version from ${new Date(snapshot.timestamp).toLocaleString()}?`,
      async () => {
        setRestoring(snapshot.id);
        try {
          await invoke('snapshot_rollback', { snapshotId: snapshot.id, objectId });
          navigate(-1);
        } catch (e) {
          showToast({ type: 'error', message: `${t('common:rollback_failed')}: ${e}` });
        } finally {
          setRestoring(null);
        }
      },
      { confirmLabel: t('common:restore'), cancelLabel: t('common:cancel') },
    );
  };

  return (
    <AppShell title={t('common:history')} onBack={() => navigate(-1)}>
      {confirmDialog}
      <PageContainer variant="xs" gap="default">
        {loading ? (
          <Card>
            <LoadingPlaceholder variant="elevated" minHeight={80} />
          </Card>
        ) : snapshots.length === 0 ? (
          <Card>
            <div style={{ textAlign: 'center', padding: 48 }}>
              <Clock size={ICON_SIZE['4xl']} style={{ marginBottom: 12, opacity: 0.25 }} />
              <p style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-sm)' }}>
                {t('common:no_history')}
              </p>
            </div>
          </Card>
        ) : (
          <>
            <p style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
              {t('common:version_count', { n: snapshots.length })}
            </p>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
              {snapshots.map((s, i) => (
                <Card key={s.id}>
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                    }}
                  >
                    <div>
                      <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
                        {i === 0
                          ? t('common:snapshot_current')
                          : new Date(s.timestamp).toLocaleString()}
                      </div>
                      <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                        {t(`common:trigger_${s.triggeredBy}` as const, {
                          defaultValue: s.diffSummary || s.triggeredBy,
                        })}
                      </div>
                    </div>
                    <div style={{ display: 'flex', gap: 6 }}>
                      {i > 0 && (
                        <Button
                          size="sm"
                          variant="secondary"
                          onClick={() => handleRollback(s)}
                          loading={restoring === s.id}
                        >
                          <RotateCcw size={ICON_SIZE.xs} style={{ marginRight: 3 }} />{' '}
                          {t('common:restore')}
                        </Button>
                      )}
                      <ChevronRight
                        size={ICON_SIZE.md}
                        style={{ color: 'var(--text-tertiary)', marginTop: 4 }}
                      />
                    </div>
                  </div>
                </Card>
              ))}
            </div>
          </>
        )}
      </PageContainer>
    </AppShell>
  );
}
