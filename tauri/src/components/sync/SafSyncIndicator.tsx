import { useTranslation } from 'react-i18next';
import { useSafSyncStore } from '@/stores/safSyncStore';
import { isMobilePlatformSync } from '@/lib/platform';
import { CheckCircle2, AlertCircle, Loader2 } from 'lucide-react';
import { ICON_SIZE, SAFE_AREA_BOTTOM_OFFSET } from '@/lib/constants';

/**
 * SAF 同步状态微指示器。
 *
 * - 仅在 Android 平台上渲染。
 * - 闲置时不显示。
 * - 同步中：底部显示小 spinner + "同步中…"。
 * - 同步完成：显示绿色勾 + "已同步"（3 秒后自动消失，由 store 控制）。
 * - 同步失败：显示红色叹号 + "同步失败"（5 秒后自动消失，由 store 控制）。
 */
export function SafSyncIndicator() {
  const { t } = useTranslation(['settings', 'common']);
  const status = useSafSyncStore((s) => s.status);
  const error = useSafSyncStore((s) => s.error);
  const isMobile = isMobilePlatformSync();

  if (!isMobile || status === 'idle') return null;

  const isSyncing = status === 'syncing';
  const isCompleted = status === 'completed';
  const isError = status === 'error';

  const icon = isSyncing ? (
    <Loader2 size={ICON_SIZE.sm} className="sync-spinner" />
  ) : isCompleted ? (
    <CheckCircle2 size={ICON_SIZE.sm} style={{ color: '#22c55e' }} />
  ) : (
    <AlertCircle size={ICON_SIZE.sm} style={{ color: '#ef4444' }} />
  );

  const label = isSyncing
    ? t('settings:vault_directory_syncing', '同步中…')
    : isCompleted
      ? t('settings:vault_directory_synced', '已同步')
      : error || t('settings:vault_directory_sync_failed', '同步失败');

  return (
    <div
      style={{
        position: 'fixed',
        bottom: `calc(72px + ${SAFE_AREA_BOTTOM_OFFSET})`,
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 999,
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        padding: '6px 12px',
        borderRadius: 20,
        background: isError
          ? 'rgba(239, 68, 68, 0.12)'
          : 'color-mix(in srgb, var(--accent-primary) 12%, var(--bg-elevated))',
        border: `1px solid ${
          isError
            ? 'rgba(239, 68, 68, 0.3)'
            : 'color-mix(in srgb, var(--accent-primary) 25%, transparent)'
        }`,
        backdropFilter: 'blur(8px)',
        WebkitBackdropFilter: 'blur(8px)',
        opacity: 1,
        transition: 'opacity 0.3s ease',
        pointerEvents: 'none',
        fontSize: 'var(--text-caption)',
        color: isError ? '#ef4444' : 'var(--text-primary)',
        whiteSpace: 'nowrap',
      }}
    >
      {icon}
      <style>{`
        @keyframes syncSpin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
        .sync-spinner {
          animation: syncSpin 1s linear infinite;
        }
      `}</style>
      <span>{label}</span>
    </div>
  );
}
