/**
 * P010：Vault 目录设置区（Android）——纯展示组合层。
 * 状态与处理器见 useVaultDirectory.ts。
 */
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';
import { Folder, RefreshCw, Download, Upload, AlertCircle, Loader2 } from 'lucide-react';
import { useVaultDirectory } from './useVaultDirectory';

export function VaultDirectorySection() {
  const { t } = useTranslation(['settings', 'common']);
  const s = useVaultDirectory();
  const {
    info,
    loading,
    loadError,
    acting,
    needsRestart,
    platformName,
    syncProgress,
    getProgressLabel,
    loadInfo,
    handlePickAndSet,
    handleResetLocal,
    handleConfirmResetLocal,
    handleSyncToRemote,
    handleSyncFromRemote,
    handleRestart,
  } = s;

  if (platformName === '' || loading) {
    return (
      <Card>
        <p style={{ margin: 0, color: 'var(--text-secondary)' }}>{t('common:loading')}</p>
      </Card>
    );
  }

  if (platformName !== 'android') {
    return (
      <Card>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <AlertCircle size={24} color="var(--text-tertiary)" />
          <p style={{ margin: 0, color: 'var(--text-secondary)' }}>
            {t('settings:vault_directory_unavailable', { platform: platformName })}
          </p>
        </div>
      </Card>
    );
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
                background:
                  'linear-gradient(90deg, var(--accent-primary), var(--accent-warm))',
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
        isOpen={s.showResetConfirm}
        title={t('settings:vault_directory_reset_local_confirm_title')}
        message={t('settings:vault_directory_reset_local_confirm_message')}
        confirmLabel={t('settings:vault_directory_reset_local_confirm_btn')}
        onConfirm={handleConfirmResetLocal}
        onCancel={() => s.setShowResetConfirm(false)}
        priority="important"
      />
    </>
  );
}
