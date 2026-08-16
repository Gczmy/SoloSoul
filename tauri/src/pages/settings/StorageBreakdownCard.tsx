import { useTranslation } from 'react-i18next';
import type { RefObject } from 'react';
import { X } from 'lucide-react';
import { formatBytes } from '@/lib/utils';
import { ICON_SIZE } from '@/lib/constants';
import { PieChartSvg, type PieSlice } from './PieChartSvg';

export interface VaultStats {
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

interface StorageBreakdownCardProps {
  stats: VaultStats | null;
  pieSlices: PieSlice[];
  cardRef: RefObject<HTMLDivElement | null>;
  onClose: () => void;
}

/**
 * 存储构成弹层卡片（饼图 + 图例 + 合计）及其半透明遮罩。
 * 从 DataManagementPage 抽出，保持渲染结构逐字等价。
 */
export function StorageBreakdownCard({
  stats,
  pieSlices,
  cardRef,
  onClose,
}: StorageBreakdownCardProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <>
      {/* ── Overlay when popup is open ───────────────────── */}
      <div
        style={{
          position: 'fixed',
          inset: 0,
          background: 'rgba(0,0,0,0.3)',
          zIndex: 99,
        }}
      />

      {stats && (
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
              onClick={onClose}
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
                  <span style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-caption)' }}>
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
    </>
  );
}
