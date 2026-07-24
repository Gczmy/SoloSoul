import { useEffect, useState, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';
import { useToastError } from '@/hooks/useToastError';
import { getPlatform } from '@/lib/platform';
import { relaunch } from '@tauri-apps/plugin-process';
import { listen } from '@tauri-apps/api/event';
import {
  getVaultDirectory,
  setVaultDirectory,
  pickVaultDirectory,
  syncVaultToRemote,
  syncVaultFromRemote,
  type VaultDirectoryInfo,
} from '@/lib/vaultDirectory';
import { Folder, RefreshCw, Download, Upload, AlertCircle, Loader2 } from 'lucide-react';

interface SyncProgress {
  phase: 'sync_to_remote' | 'sync_from_remote' | 'migrate' | 'auto_sync';
  current: number;
  total: number;
}

export function VaultDirectorySection() {
  const { t } = useTranslation(['settings', 'common']);
  const { onError, onSuccess } = useToastError();

  const [info, setInfo] = useState<VaultDirectoryInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const [acting, setActing] = useState(false);
  const [needsRestart, setNeedsRestart] = useState(false);
  const [showResetConfirm, setShowResetConfirm] = useState(false);
  const [platformName, setPlatformName] = useState<string>('');
  const [syncProgress, setSyncProgress] = useState<SyncProgress | null>(null);
  const unmountedRef = useRef(false);

  const loadInfo = useCallback(async () => {
    try {
      setLoading(true);
      setLoadError(false);
      const data = await getVaultDirectory();
      setInfo(data);
    } catch (e) {
      setLoadError(true);
      onError(e, t('settings:vault_directory_load_failed'));
    } finally {
      setLoading(false);
    }
  }, [t, onError]);

  useEffect(() => {
    getPlatform().then((p) => {
      setPlatformName(p);
      if (p === 'android') {
        loadInfo();
      } else {
        setLoading(false);
      }
    });

    let unlisten: (() => void) | null = null;
    listen<SyncProgress>('sync-progress', (event) => {
      const { phase, current, total } = event.payload;
      if (phase === 'auto_sync') return;
      if (current >= total) {
        setSyncProgress({ phase, current, total });
        setTimeout(() => {
          if (!unmountedRef.current) setSyncProgress(null);
        }, 2000);
      } else {
        setSyncProgress({ phase, current, total });
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unmountedRef.current = true;
      if (unlisten) unlisten();
    };
  }, [loadInfo]);

  const handlePickAndSet = async () => {
    const { pause, resume } = await import('@/stores/autoLockPauseStore').then(
      (m) => m.useAutoLockPauseStore.getState(),
    );
    pause();
    try {
      setActing(true);
      const uri = await pickVaultDirectory();
      if (!uri) return;
      setSyncProgress({ phase: 'migrate', current: 0, total: 3 });
      const result = await setVaultDirectory(uri);
      if (result.success) {
        onSuccess(t('settings:vault_directory_set_success'));
        setNeedsRestart(true);
        await loadInfo();
      } else {
        onError(new Error(result.message), t('settings:vault_directory_set_failed'));
      }
    } catch (e) {
      onError(e, t('settings:vault_directory_set_failed'));
    } finally {
      resume();
      setActing(false);
    }
  };

  const handleResetLocal = () => {
    setShowResetConfirm(true);
  };

  const handleConfirmResetLocal = async () => {
    setShowResetConfirm(false);
    try {
      setActing(true);
      const result = await setVaultDirectory(null);
      if (result.success) {
        onSuccess(t('settings:vault_directory_reset_success'));
        setNeedsRestart(true);
        await loadInfo();
      } else {
        onError(new Error(result.message), t('settings:vault_directory_reset_failed'));
      }
    } catch (e) {
      onError(e, t('settings:vault_directory_reset_failed'));
    } finally {
      setActing(false);
    }
  };

  const handleSyncToRemote = async () => {
    try {
      setActing(true);
      setSyncProgress({ phase: 'sync_to_remote', current: 0, total: 1 });
      await syncVaultToRemote();
      onSuccess(t('settings:vault_directory_sync_to_remote_success'));
    } catch (e) {
      setSyncProgress(null);
      onError(e, t('settings:vault_directory_sync_to_remote_failed'));
    } finally {
      setActing(false);
    }
  };

  const handleSyncFromRemote = async () => {
    try {
      setActing(true);
      setSyncProgress({ phase: 'sync_from_remote', current: 0, total: 1 });
      await syncVaultFromRemote();
      onSuccess(t('settings:vault_directory_sync_from_remote_success'));
      setNeedsRestart(true);
    } catch (e) {
      setSyncProgress(null);
      onError(e, t('settings:vault_directory_sync_from_remote_failed'));
    } finally {
      setActing(false);
    }
  };

  const getProgressLabel = (phase: SyncProgress['phase']): string => {
    switch (phase) {
      case 'sync_to_remote': return t('settings:vault_directory_sync_to_remote');
      case 'sync_from_remote': return t('settings:vault_directory_sync_from_remote');
      case 'migrate': return t('settings:vault_directory_migrating_title');
      default: return '';
    }
  };

  const handleRestart = async () => {
    try {
      await relaunch();
    } catch (e) {
      onError(e, t('settings:vault_directory_restart_failed'));
    }
  };

  const renderNotAvailable = () => (
    <Card>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
        <AlertCircle size={24} color="var(--text-tertiary)" />
        <p style={{ margin: 0, color: 'var(--text-secondary)' }}>
          {t('settings:vault_directory_unavailable', { platform: platformName })}
        </p>
      </div>
    </Card>
  );

  if (platformName === '' || loading) {
    return (
      <Card>
        <p style={{ margin: 0, color: 'var(--text-secondary)' }}>{t('common:loading')}</p>
      </Card>
    );
  }

  if (platformName !== 'android') {
    return renderNotAvailable();
  }

  if (loadError) {
    return (
      <Card>
        <p style={{ margin: 0, marginBottom: 12, color: 'var(--text-secondary)' }}>
          {t('settings:vault_directory_load_failed')}
        </p>
        <Button onClick={loadInfo} loading={loading} disabled={loading}>
          <RefreshCw size={16} />
          {t('settings:vault_directory_retry')}
        </Button>
      </Card>
    );
  }

  if (info === null) {
    return (
      <Card>
        <p style={{ margin: 0, color: 'var(--text-secondary)' }}>{t('common:loading')}</p>
      </Card>
    );
  }

  return (
    <>
      {syncProgress && (
        <Card
          style={{
            border: '1px solid color-mix(in srgb, var(--accent-primary) 30%, transparent)',
            background: 'color-mix(in srgb, var(--accent-primary) 8%, var(--bg-elevated))',
            marginBottom: 12,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 8 }}>
            <Loader2 size={20} style={{ color: 'var(--accent-primary)', flexShrink: 0 }} />
            <div style={{ fontWeight: 600, fontSize: 'var(--text-body-sm)' }}>
              {getProgressLabel(syncProgress.phase)}
            </div>
          </div>
          <div
            style={{
              width: '100%', height: 6, borderRadius: 3,
              background: 'var(--bg-toolbar)', overflow: 'hidden',
            }}
          >
            <div
              style={{
                width: `${Math.round((syncProgress.current / syncProgress.total) * 100)}%`,
                height: '100%', borderRadius: 3,
                background: 'var(--accent-primary)',
                transition: 'width 0.3s ease',
              }}
            />
          </div>
          <div style={{ marginTop: 4, fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)', textAlign: 'right' }}>
            {syncProgress.current}/{syncProgress.total}
          </div>
        </Card>
      )}

      {info.directoryType === 'saf' && !info.valid && (
        <Card
          style={{
            border: '1px solid #dc2626',
            background: 'rgba(220, 38, 38, 0.06)',
            marginBottom: 12,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 10 }}>
            <AlertCircle size={20} style={{ color: '#dc2626', flexShrink: 0 }} />
            <div>
              <div style={{ fontWeight: 600, marginBottom: 4 }}>
                {t('settings:vault_directory_invalid_title')}
              </div>
              <div style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                {t('settings:vault_directory_invalid_desc')}
              </div>
            </div>
          </div>
          <Button onClick={handlePickAndSet} loading={acting} disabled={acting || needsRestart}>
            <Folder size={16} />
            {t('settings:vault_directory_choose')}
          </Button>
        </Card>
      )}

      <Card>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12 }}>
          <div
            style={{
              width: 40, height: 40, borderRadius: 10,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'rgba(91,124,153,0.1)',
            }}
          >
            <Folder size={20} style={{ color: 'var(--accent-primary)' }} />
          </div>
          <div style={{ flex: 1 }}>
            <div style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
              {t('settings:vault_directory_current_type')}
            </div>
            <div style={{ fontSize: 'var(--text-page-title)', fontWeight: 600 }}>
              {info.directoryType === 'saf'
                ? t('settings:vault_directory_type_saf')
                : t('settings:vault_directory_type_local')}
            </div>
          </div>
        </div>

        {info.directoryType === 'saf' && info.safTreeUri && (
          <div
            style={{
              padding: 10, borderRadius: 8, background: 'var(--bg-toolbar)',
              fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)',
              wordBreak: 'break-all', marginBottom: 12,
            }}
          >
            <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:vault_directory_saf_uri')}</span>
            <br />
            {info.safTreeUri}
          </div>
        )}

        <p style={{ margin: '0 0 12px 0', fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
          {t('settings:vault_directory_explanation')}
        </p>

        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
          {info.directoryType === 'local' ? (
            <Button onClick={handlePickAndSet} loading={acting} disabled={acting || needsRestart}>
              <Folder size={16} />
              {t('settings:vault_directory_choose')}
            </Button>
          ) : (
            <>
              <Button onClick={handlePickAndSet} loading={acting} disabled={acting || needsRestart}>
                <Folder size={16} />
                {t('settings:vault_directory_change')}
              </Button>
              <Button
                variant="secondary"
                onClick={handleSyncToRemote}
                loading={acting}
                disabled={acting || needsRestart}
              >
                <Upload size={16} />
                {t('settings:vault_directory_sync_to_remote')}
              </Button>
              <Button
                variant="secondary"
                onClick={handleSyncFromRemote}
                loading={acting}
                disabled={acting || needsRestart}
              >
                <Download size={16} />
                {t('settings:vault_directory_sync_from_remote')}
              </Button>
              <Button
                variant="tertiary"
                onClick={handleResetLocal}
                loading={acting}
                disabled={acting || needsRestart}
              >
                <RefreshCw size={16} />
                {t('settings:vault_directory_reset_local')}
              </Button>
            </>
          )}
        </div>
      </Card>

      {needsRestart && (
        <Card
          style={{
            border: '1px solid color-mix(in srgb, var(--accent-primary) 30%, transparent)',
            background: 'color-mix(in srgb, var(--accent-primary) 8%, var(--bg-elevated))',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12 }}>
            <AlertCircle size={20} style={{ color: 'var(--accent-primary)' }} />
            <div>
              <div style={{ fontWeight: 600, marginBottom: 4 }}>
                {t('settings:vault_directory_restart_required')}
              </div>
              <div style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                {t('settings:vault_directory_restart_required_desc')}
              </div>
            </div>
          </div>
          <Button onClick={handleRestart} disabled={acting}>
            <RefreshCw size={16} />
            {t('settings:vault_directory_restart')}
          </Button>
        </Card>
      )}

      <ConfirmDialog
        isOpen={showResetConfirm}
        title={t('settings:vault_directory_reset_local_confirm_title')}
        message={t('settings:vault_directory_reset_local_confirm_message')}
        confirmLabel={t('settings:vault_directory_reset_local_confirm_btn')}
        onConfirm={handleConfirmResetLocal}
        onCancel={() => setShowResetConfirm(false)}
        priority="important"
      />
    </>
  );
}
