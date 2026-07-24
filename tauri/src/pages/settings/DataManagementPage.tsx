import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';

import { HardDrive, PieChart, X, FolderTree } from 'lucide-react';
import { formatBytes } from '@/lib/utils';
import { isMobilePlatformSync } from '@/lib/platform';
import { ICON_SIZE } from '@/lib/constants';

interface VaultStats {
  profileCount: number;
  totalSizeBytes: number;
  lastModified?: string;
  profilesSize: number;
  objectsSize: number;
  trashSize: number;
  snapshotsSize: number;
  attachmentsSize: number;
  aiConversationsSize: number;
}

// ── Color palette for the pie chart ──────────────────────────
const PIE_COLORS = [
  '#5b7c99', // profiles
  '#4a9eff', // objects
  '#e68a00', // trash
  '#7b61ff', // snapshots
  '#2e7d32', // attachments
  '#d32f2f', // AI conversations
];

// ── SVG Pie Chart ────────────────────────────────────────────

interface PieSlice {
  key: string;
  value: number;
  color: string;
  label: string;
}

function PieChartSvg({ slices, size }: { slices: PieSlice[]; size: number }) {
  const total = slices.reduce((s, p) => s + p.value, 0);
  if (total === 0) return null;
  const cx = size / 2;
  const cy = size / 2;
  const r = size / 2 - 4;
  let cumulative = 0;

  const arcs = slices.map((slice) => {
    const sliceAngle = (slice.value / total) * 360;
    const startAngle = (cumulative / total) * 360;
    cumulative += slice.value;

    const startRad = ((startAngle - 90) * Math.PI) / 180;
    const endRad = ((startAngle + sliceAngle - 90) * Math.PI) / 180;

    const x1 = cx + r * Math.cos(startRad);
    const y1 = cy + r * Math.sin(startRad);
    const x2 = cx + r * Math.cos(endRad);
    const y2 = cy + r * Math.sin(endRad);

    const largeArc = sliceAngle > 180 ? 1 : 0;

    const path =
      sliceAngle >= 360
        ? `M ${cx} ${cy - r} A ${r} ${r} 0 1 1 ${cx - 0.01} ${cy - r} Z`
        : `M ${cx} ${cy} L ${x1} ${y1} A ${r} ${r} 0 ${largeArc} 1 ${x2} ${y2} Z`;

    return (
      <path
        key={slice.key}
        d={path}
        fill={slice.color}
        stroke="var(--bg-primary)"
        strokeWidth={1}
      />
    );
  });

  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ display: 'block' }}>
      {arcs}
      {total > 0 && (
        <text
          x={cx}
          y={cy}
          textAnchor="middle"
          dominantBaseline="central"
          fontSize={13}
          fontWeight={600}
          fill="var(--text-primary)"
        >
          {formatBytes(total)}
        </text>
      )}
    </svg>
  );
}

// ── Component ────────────────────────────────────────────────

export function DataManagementPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const isMobile = isMobilePlatformSync();
  const [stats, setStats] = useState<VaultStats | null>(null);
  const [showBreakdown, setShowBreakdown] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<VaultStats>('get_vault_stats')
      .then(setStats)
      .catch(() => setStats(null));
  }, []);

  // Close breakdown card on outside click
  useEffect(() => {
    if (!showBreakdown) return;
    const handleClick = (e: MouseEvent) => {
      if (cardRef.current && !cardRef.current.contains(e.target as Node)) {
        setShowBreakdown(false);
      }
    };
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, [showBreakdown]);

  // ── Build breakdown items ────────────────────────────────────
  const breakdownItems = !stats
    ? []
    : [
        { key: 'profile', size: stats.profilesSize, labelKey: 'data_profile' },
        { key: 'objects', size: stats.objectsSize, labelKey: 'data_objects' },
        { key: 'trash', size: stats.trashSize, labelKey: 'data_trash' },
        { key: 'snapshots', size: stats.snapshotsSize, labelKey: 'data_snapshots' },
        { key: 'attachments', size: stats.attachmentsSize, labelKey: 'data_attachments' },
        { key: 'ai', size: stats.aiConversationsSize, labelKey: 'data_ai_chat' },
      ].filter((i) => i.size > 0);

  const pieSlices: PieSlice[] = breakdownItems.map((item, idx) => ({
    key: item.key,
    value: item.size,
    color: PIE_COLORS[idx % PIE_COLORS.length],
    label: t(`settings:${item.labelKey}`, item.key),
  }));

  return (
    <AppShell title={t('settings:data_management')} onBack={() => navigate('/settings')}>
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
              style={{
                background: 'transparent',
                border: 'none',
                cursor: 'pointer',
                padding: 6,
                borderRadius: 6,
                color: 'var(--text-tertiary)',
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background =
                  'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'transparent';
                e.currentTarget.style.color = 'var(--text-tertiary)';
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
        {showBreakdown && stats && (
          <div
            ref={cardRef}
            style={{
              position: 'fixed',
              top: '50%',
              left: '50%',
              transform: 'translate(-50%, -50%)',
              width: 340,
              maxHeight: '80vh',
              overflowY: 'auto',
              zIndex: 100,
              background: 'var(--bg-elevated)',
              borderRadius: 12,
              padding: 20,
              boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
            }}
          >
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: 16,
              }}
            >
              <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, margin: 0 }}>
                {t('settings:storage_breakdown')}
              </h3>
              <button
                onClick={() => setShowBreakdown(false)}
                style={{
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  padding: 4,
                  color: 'var(--text-tertiary)',
                }}
              >
                <X size={ICON_SIZE.lg} />
              </button>
            </div>

            {/* Pie chart */}
            <div style={{ display: 'flex', justifyContent: 'center', marginBottom: 16 }}>
              <PieChartSvg slices={pieSlices} size={180} />
            </div>

            {/* Legend */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {pieSlices.map((slice, _idx) => {
                const pct = ((slice.value / stats.totalSizeBytes) * 100).toFixed(1);
                return (
                  <div
                    key={slice.key}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                      fontSize: 'var(--text-body-sm)',
                    }}
                  >
                    <div
                      style={{
                        width: 12,
                        height: 12,
                        borderRadius: 3,
                        background: slice.color,
                        flexShrink: 0,
                      }}
                    />
                    <span style={{ flex: 1 }}>{slice.label}</span>
                    <span
                      style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-caption)' }}
                    >
                      {pct}%
                    </span>
                    <span style={{ fontWeight: 500 }}>{formatBytes(slice.value)}</span>
                  </div>
                );
              })}
              {/* Total */}
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  fontSize: 'var(--text-body-sm)',
                  borderTop: '1px solid var(--border-subtle)',
                  paddingTop: 8,
                  marginTop: 4,
                }}
              >
                <span style={{ flex: 1, fontWeight: 600 }}>{t('common:total')}</span>
                <span style={{ fontWeight: 600 }}>{formatBytes(stats.totalSizeBytes)}</span>
              </div>
            </div>
          </div>
        )}

        {/* ── Overlay when popup is open ───────────────────── */}
        {showBreakdown && (
          <div
            style={{
              position: 'fixed',
              inset: 0,
              background: 'rgba(0,0,0,0.3)',
              zIndex: 99,
            }}
          />
        )}

        {/* Vault directory (mobile only) */}
        {isMobile && (
          <Card interactive onClick={() => navigate('/settings/vault-directory')}>
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
                <FolderTree size={ICON_SIZE['2xl']} style={{ color: 'var(--accent-primary)' }} />
              </div>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
                  {t('settings:items.vault_directory') || '保险库目录'}
                </div>
                <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)', marginTop: 2 }}>
                  {t('settings:desc.vault_directory') || '选择保险库数据存储位置'}
                </div>
              </div>
              <span style={{ color: 'var(--text-tertiary)', fontSize: 20 }}>›</span>
            </div>
          </Card>
        )}

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
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background =
                  'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--border-subtle)';
                e.currentTarget.style.color = 'var(--text-primary)';
              }}
            >
              {t('settings:create_backup')}
            </button>
            <button
              type="button"
              onClick={() => navigate('/settings/backup')}
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background =
                  'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--border-subtle)';
                e.currentTarget.style.color = 'var(--text-primary)';
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
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background =
                  'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--border-subtle)';
                e.currentTarget.style.color = 'var(--text-primary)';
              }}
            >
              {t('settings:export')}
            </button>
            <button
              type="button"
              onClick={() => navigate('/settings/export-import')}
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background =
                  'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--border-subtle)';
                e.currentTarget.style.color = 'var(--text-primary)';
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
            style={{
              fontSize: 'var(--text-caption)',
              padding: '6px 12px',
              borderRadius: 6,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-toolbar)',
              color: 'var(--text-primary)',
              cursor: 'pointer',
              fontFamily: 'inherit',
              fontWeight: 500,
              transition: 'all 0.15s ease',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background =
                'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              e.currentTarget.style.borderColor = 'var(--accent-primary)';
              e.currentTarget.style.color = 'var(--accent-primary)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'var(--bg-toolbar)';
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.color = 'var(--text-primary)';
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
    </AppShell>
  );
}
