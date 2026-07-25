import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { useUiStore } from '@/stores/uiStore';
import { Loader2, CheckCircle, AlertCircle } from 'lucide-react';
import { ICON_SIZE, SAFE_AREA_TOP } from '@/lib/constants';

interface SyncProgressPayload {
  phase: string;
  current?: number;
  total?: number;
  message?: string;
  source?: string;
  silent?: boolean;
}

const AUTO_HIDE_MS = 3000;

/**
 * GlobalSyncIndicator — 全局 SAF 自动同步状态指示器。
 *
 * - 监听 `sync-progress` 事件，更新同步状态/进度。
 * - 监听 `saf-auth-revoked` 事件，显示授权失效常驻横幅。
 * - 同步完成后自动隐藏；授权失效需用户手动处理。
 */
export function GlobalSyncIndicator() {
  const { t } = useTranslation(['common', 'settings']);
  const {
    safSyncState,
    safSyncProgress,
    safSyncError,
    safAuthRevoked,
    setSafSyncState,
    setSafSyncProgress,
    setSafSyncError,
    setSafAuthRevoked,
  } = useUiStore();

  const hideTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const unlistens: Array<() => void> = [];

    listen<SyncProgressPayload>('sync-progress', (event) => {
      const { phase, current = 0, total = 0, message, silent } = event.payload;

      // 静默同步不显示任何提示（如 30 秒周期性兜底同步）。
      if (silent) {
        return;
      }

      if (phase === 'sync_start' || phase === 'sync_to_remote' || phase === 'sync_from_remote' || phase === 'migrate' || phase === 'auto_sync') {
        setSafSyncState('syncing');
        setSafSyncProgress({ current, total });
        setSafSyncError(null);
      } else if (phase === 'sync_complete') {
        setSafSyncState('complete');
        setSafSyncProgress({ current, total });
        if (hideTimeoutRef.current) clearTimeout(hideTimeoutRef.current);
        hideTimeoutRef.current = setTimeout(() => {
          setSafSyncState('idle');
          setSafSyncProgress({ current: 0, total: 0 });
        }, AUTO_HIDE_MS);
      } else if (phase === 'error') {
        setSafSyncState('error');
        setSafSyncError(message ?? t('common:error'));
        if (hideTimeoutRef.current) clearTimeout(hideTimeoutRef.current);
        hideTimeoutRef.current = setTimeout(() => {
          setSafSyncState('idle');
          setSafSyncError(null);
        }, AUTO_HIDE_MS * 2);
      }
    }).then((unlisten) => unlistens.push(unlisten));

    listen<void>('saf-auth-revoked', () => {
      setSafAuthRevoked(true);
      setSafSyncState('error');
      setSafSyncError(t('settings:vault_directory_invalid_toast'));
    }).then((unlisten) => unlistens.push(unlisten));

    return () => {
      unlistens.forEach((fn) => fn());
      if (hideTimeoutRef.current) clearTimeout(hideTimeoutRef.current);
    };
  }, [setSafSyncState, setSafSyncProgress, setSafSyncError, setSafAuthRevoked, t]);

  if (!safAuthRevoked && safSyncState === 'idle') {
    return null;
  }

  return (
    <div
      style={{
        position: 'fixed',
        top: SAFE_AREA_TOP,
        left: 0,
        right: 0,
        zIndex: 9999,
        padding: '8px 12px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 8,
        fontSize: 'var(--text-body-sm)',
        fontWeight: 500,
        backdropFilter: 'blur(6px)',
        WebkitBackdropFilter: 'blur(6px)',
        ...(safAuthRevoked
          ? {
              background: 'rgba(220, 38, 38, 0.92)',
              color: '#fff',
            }
          : safSyncState === 'error'
            ? {
                background: 'rgba(230, 126, 34, 0.92)',
                color: '#fff',
              }
            : {
                background: 'rgba(91, 124, 153, 0.92)',
                color: '#fff',
              }),
      }}
    >
      {safAuthRevoked ? (
        <>
          <AlertCircle size={ICON_SIZE.md} />
          <span>{t('settings:vault_directory_invalid_toast')}</span>
        </>
      ) : safSyncState === 'syncing' ? (
        <>
          <Loader2 size={ICON_SIZE.md} style={{ animation: 'spin 1s linear infinite' }} />
          <span>
            {t('onboarding_vault_dir_syncing')}
            {safSyncProgress.total > 0 && (
              <span style={{ marginLeft: 8, opacity: 0.9 }}>
                {safSyncProgress.current}/{safSyncProgress.total}
              </span>
            )}
          </span>
        </>
      ) : safSyncState === 'complete' ? (
        <>
          <CheckCircle size={ICON_SIZE.md} />
          <span>{t('onboarding_vault_dir_sync_done')}</span>
        </>
      ) : safSyncState === 'error' ? (
        <>
          <AlertCircle size={ICON_SIZE.md} />
          <span>{safSyncError || t('common:error')}</span>
        </>
      ) : null}
      <style>{`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}
