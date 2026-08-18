import { useState, useEffect, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';

import { HardDrive, PieChart } from 'lucide-react';
import { formatBytes } from '@/lib/utils';
import { isMobilePlatformSync } from '@/lib/platform';
import { ICON_SIZE } from '@/lib/constants';
import { VaultDirectorySection } from './VaultDirectorySection';
import { StorageBreakdownCard, type VaultStats } from './StorageBreakdownCard';
import { PIE_COLORS, type PieSlice } from './PieChartSvg';

// ── Component ────────────────────────────────────────────────

export function DataManagementPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const isMobile = isMobilePlatformSync();

  const [stats, setStats] = useState<VaultStats | null>(null);
  const [showBreakdown, setShowBreakdown] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<VaultStats>('get_vault_stats')
      .then((result) => {
        if (!cancelled) setStats(result);
      })
      .catch(() => {
        if (!cancelled) setStats(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Close breakdown card on outside click
  useEffect(() => {
    if (!showBreakdown) return;
    const onPointerDown = (e: PointerEvent) => {
      if (cardRef.current && !cardRef.current.contains(e.target as Node)) {
        setShowBreakdown(false);
      }
    };
    document.addEventListener('pointerdown', onPointerDown);
    return () => document.removeEventListener('pointerdown', onPointerDown);
  }, [showBreakdown]);

  // ── Build breakdown items ────────────────────────────────────
  const breakdownItems = useMemo(
    () =>
      stats
        ? [
            { key: 'profiles', labelKey: 'profiles_size', size: stats.profilesSize },
            { key: 'objects', labelKey: 'objects_size', size: stats.objectsSize },
            { key: 'trash', labelKey: 'trash_size', size: stats.trashSize },
            { key: 'snapshots', labelKey: 'snapshots_size', size: stats.snapshotsSize },
            { key: 'attachments', labelKey: 'attachments_size', size: stats.attachmentsSize },
            {
              key: 'ai_conversations',
              labelKey: 'ai_conversations_size',
              size: stats.aiConversationsSize,
            },
          ].filter((item) => item.size > 0)
        : [],
    [stats],
  );

  const pieSlices: PieSlice[] = useMemo(
    () =>
      breakdownItems.map((item, idx) => ({
        key: item.key,
        value: item.size,
        color: PIE_COLORS[idx % PIE_COLORS.length],
        label: t(`settings:${item.labelKey}`),
      })),
    [breakdownItems, t],
  );

  return (
    <PageShell title={t('settings:data_management')} onBack={() => navigate('/settings')}>
      <PageContainer variant="form" gap="default">
        {/* Vault stats card */}
        <Card>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div
              style={{
                width: 44,
                height: 44,
                borderRadius: 10,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                background: 'rgba(91,124,153,0.1)',
              }}
            >
              <HardDrive size={ICON_SIZE['2xl']} style={{ color: 'var(--accent-primary)' }} />
            </div>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                {t('settings:vault_size')}
              </div>
              <div style={{ fontSize: 'var(--text-page-title)', fontWeight: 600 }}>
                {stats ? formatBytes(stats.totalSizeBytes) : '—'}
              </div>
              <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                {stats ? t('settings:profile_count', { count: stats.profileCount }) : '—'}
                {stats?.lastModified &&
                  ` · ${t('settings:updated')} ${new Date(stats.lastModified).toLocaleDateString()}`}
              </div>
            </div>
            <button
              onClick={() => setShowBreakdown(!showBreakdown)}
              className="interactive-accent"
              style={{
                border: 'none',
                cursor: 'pointer',
                padding: 6,
                borderRadius: 6,
              }}
              title={t('settings:view_breakdown')}
            >
              <PieChart size={ICON_SIZE.xl} />
            </button>
          </div>

          {/* ── Breakdown sub-lines ───────────────────────────── */}
          {stats && breakdownItems.length > 0 && (
            <div
              style={{
                marginTop: 10,
                paddingTop: 10,
                borderTop: '1px solid var(--border-subtle)',
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                display: 'flex',
                flexDirection: 'column',
                gap: 3,
              }}
            >
              {breakdownItems.map((item) => (
                <div key={item.key} style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span>{t(`settings:${item.labelKey}`, item.key)}</span>
                  <span>{formatBytes(item.size)}</span>
                </div>
              ))}
            </div>
          )}
        </Card>

        {/* ── Breakdown popup card ──────────────────────────── */}
        {showBreakdown && (
          <StorageBreakdownCard
            stats={stats}
            pieSlices={pieSlices}
            cardRef={cardRef}
            onClose={() => setShowBreakdown(false)}
          />
        )}

        {/* Vault directory (mobile only) — inline content */}
        {/* 说明：VaultDirectorySection 自身已含「当前存储类型」卡片，此处不再重复标题 */}
        {isMobile && <VaultDirectorySection />}

        {/* Quick actions */}
        <Card>
          <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, marginBottom: 4 }}>
            {t('settings:backup')}
          </h3>
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              marginBottom: 12,
            }}
          >
            {t('settings:backup_desc')}
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            <button
              type="button"
              onClick={() => navigate('/settings/backup')}
              className="interactive-toolbar"
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                borderWidth: 1,
                borderStyle: 'solid',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
              }}
            >
              {t('settings:create_backup')}
            </button>
            <button
              type="button"
              onClick={() => navigate('/settings/backup')}
              className="interactive-toolbar"
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                borderWidth: 1,
                borderStyle: 'solid',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
              }}
            >
              {t('settings:restore')}
            </button>
          </div>
        </Card>

        <Card>
          <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, marginBottom: 4 }}>
            {t('settings:export_import')}
          </h3>
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              marginBottom: 12,
            }}
          >
            {t('settings:export_import_desc')}
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            <button
              type="button"
              onClick={() => navigate('/settings/export-import')}
              className="interactive-toolbar"
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                borderWidth: 1,
                borderStyle: 'solid',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
              }}
            >
              {t('settings:export')}
            </button>
            <button
              type="button"
              onClick={() => navigate('/settings/export-import')}
              className="interactive-toolbar"
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                borderWidth: 1,
                borderStyle: 'solid',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
              }}
            >
              {t('settings:import')}
            </button>
          </div>
        </Card>

        <Card>
          <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, marginBottom: 4 }}>
            {t('settings:trash')}
          </h3>
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              marginBottom: 12,
            }}
          >
            {t('settings:trash_empty')}
          </p>
          <button
            type="button"
            onClick={() => navigate('/settings/trash')}
            className="interactive-toolbar"
            style={{
              fontSize: 'var(--text-caption)',
              padding: '6px 12px',
              borderRadius: 6,
              borderWidth: 1,
              borderStyle: 'solid',
              cursor: 'pointer',
              fontFamily: 'inherit',
              fontWeight: 500,
            }}
          >
            {t('settings:trash')}
          </button>
        </Card>

        {/* Snapshot retention config */}
        <Card>
          <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, marginBottom: 4 }}>
            {t('settings:snapshot_retention')}
          </h3>
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              marginBottom: 12,
            }}
          >
            {t('settings:snapshot_retention_desc')}
          </p>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {(['unlimited', '50', '100', '200'] as const).map((opt) => (
              <label
                key={opt}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  cursor: 'pointer',
                  fontSize: 'var(--text-body-sm)',
                  padding: '4px 0',
                }}
              >
                <input
                  type="radio"
                  name="snapshotLimit"
                  defaultChecked={opt === 'unlimited'}
                  style={{ accentColor: 'var(--accent-primary)' }}
                />
                {opt === 'unlimited'
                  ? t('settings:snapshot_unlimited')
                  : t('settings:snapshot_count', { n: opt })}
              </label>
            ))}
          </div>
        </Card>
      </PageContainer>
    </PageShell>
  );
}
