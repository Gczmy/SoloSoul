import { useState, useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useUiStore } from '@/stores/uiStore';
import { useConfirm } from '@/hooks/useConfirm';
import { Clock, RotateCcw, ChevronRight } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import type { SnapshotEntry } from '@/types/history';
import styles from './HistoryPage.module.css';

/** P039: 快照列表分页大小（快照随编辑次数无限增长，避免一次性渲染全部卡片）。 */
const SNAPSHOT_PAGE_SIZE = 20;

export function HistoryPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const objectId = searchParams.get('objectId') || '';
  const [snapshots, setSnapshots] = useState<SnapshotEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [restoring, setRestoring] = useState<string | null>(null);
  // P039: 快照随编辑次数无限增长——分页渲染，加载更多
  const [visibleLimit, setVisibleLimit] = useState(SNAPSHOT_PAGE_SIZE);
  const { t } = useTranslation(['common']);
  const showToast = useUiStore((s) => s.showToast);
  const { requestConfirm, dialog: confirmDialog } = useConfirm();

  useEffect(() => {
    if (objectId) {
      // P039(评审反馈): 切换对象时重置分页游标，避免继承上一个对象的展开深度
      setVisibleLimit(SNAPSHOT_PAGE_SIZE);
      invoke<SnapshotEntry[]>('snapshot_list', { objectId: objectId })
        .then(setSnapshots)
        .catch((err) => {
          // P059: 补齐 .catch，失败时给出提示而非 unhandled rejection
          showToast({
            type: 'error',
            message: `${t('common:history_load_failed', 'Failed to load history')}: ${err}`,
          });
        })
        .finally(() => setLoading(false));
    }
    // showToast/t 为稳定引用，仅需在 objectId 变化时重新加载
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [objectId]);

  const handleRollback = (snapshot: SnapshotEntry) => {
    requestConfirm(
      t('common:rollback_confirm_title', 'Restore version'),
      t('common:rollback_confirm_body', {
        date: new Date(snapshot.timestamp).toLocaleString(),
        defaultValue: `Rollback to version from ${new Date(snapshot.timestamp).toLocaleString()}?`,
      }),
      async () => {
        setRestoring(snapshot.id);
        try {
          await invoke('snapshot_rollback', { snapshotId: snapshot.id, objectId: objectId });
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
    <PageShell title={t('common:history')} onBack={() => navigate(-1)}>
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
              {snapshots.slice(0, visibleLimit).map((s, i) => (
                <Card key={s.id}>
                  <div className={styles.snapshotRow}>
                    <div className={styles.snapshotInfo}>
                      <div className={styles.snapshotTitle}>
                        {i === 0
                          ? t('common:snapshot_current')
                          : new Date(s.timestamp).toLocaleString()}
                      </div>
                      <div className={styles.snapshotMeta}>
                        {t(`common:trigger_${s.triggeredBy}` as const, {
                          defaultValue: s.triggeredBy,
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
              {visibleLimit < snapshots.length && (
                <Button
                  variant="secondary"
                  onClick={() => setVisibleLimit((n) => n + SNAPSHOT_PAGE_SIZE)}
                  style={{ width: '100%', marginTop: 4 }}
                >
                  {t('common:load_more', { defaultValue: '加载更多' })}
                </Button>
              )}
            </div>
          </>
        )}
      </PageContainer>
    </PageShell>
  );
}
