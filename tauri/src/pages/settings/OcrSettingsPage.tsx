import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { useToastError } from '@/hooks/useToastError';
import { useConfirm } from '@/hooks/useConfirm';
import { useOcrModelManager } from '@/hooks/useOcrModelManager';
import { getTierLabel } from '@/lib/utils';
import { Download, CheckCircle, AlertCircle, Trash2 } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { isMobilePlatformSync } from '@/lib/platform';

export function OcrSettingsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['ocr', 'settings', 'common']);
  const { onError, onSuccess } = useToastError();
  const { requestConfirm, dialog: confirmDialog } = useConfirm();
  const isMobilePlatform = isMobilePlatformSync();

  const {
    tiers,
    activeTier,
    statusMap,
    loading,
    installingTier,
    downloadingTier,
    deletingTier,
    downloadUrl,
    setDownloadUrl,
    handleTierChange,
    handleInstallBundled,
    handleDelete,
    handleDownload,
  } = useOcrModelManager({
    enabled: !isMobilePlatform,
    t,
    onError,
    onTierChangeSuccess: onSuccess,
    onInstallSuccess: onSuccess,
    onDeleteSuccess: onSuccess,
    onDownloadSuccess: onSuccess,
    confirmDownload: ({ message, confirmLabel, cancelLabel, onConfirm }) =>
      requestConfirm(
        t('ocr:confirm_download_title'),
        message,
        onConfirm,
        { confirmLabel, cancelLabel },
      ),
  });

  return (
    <AppShell title={t('ocr:settings_title')} onBack={() => navigate('/settings')}>
      {confirmDialog}
      <PageContainer variant="medium" gap="default">
        {!isMobilePlatform && (
          <Card>
            <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
              {t('ocr:active_model')}
            </h3>
            <select
              value={activeTier}
              onChange={(e) => handleTierChange(e.target.value)}
              disabled={loading}
              style={{
                width: '100%',
                padding: '8px 10px',
                fontSize: 'var(--text-body-sm)',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                transition: 'border-color 0.15s ease',
              }}
            >
              {tiers.map((tier) => {
                const label = getTierLabel(t, tier);
                return (
                  <option key={tier.tier} value={tier.tier}>
                    {label.name} — {label.description}
                  </option>
                );
              })}
            </select>
          </Card>
        )}

        {!isMobilePlatform && (
          <Card>
            <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
              {t('ocr:model_management')}
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {tiers.map((tier) => {
                const status = statusMap[tier.tier];
                const isInstalling = installingTier === tier.tier;
                const isDownloading = downloadingTier === tier.tier;
                const isDeleting = deletingTier === tier.tier;
                // P133: Vision 为系统内置引擎，无存储占用（tierSize 空串）。
                const tierSize =
                  tier.tier === 'tiny'
                    ? '1.5MB'
                    : tier.tier === 'medium'
                      ? '132MB'
                      : tier.tier === 'vision'
                        ? ''
                        : '30MB';
                return (
                  <div
                    key={tier.tier}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      padding: '10px 12px',
                      borderRadius: 8,
                      background: 'var(--bg-toolbar)',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1 }}>
                      {status?.installed ? (
                        <CheckCircle size={ICON_SIZE.md} color="var(--accent-primary)" />
                      ) : status?.bundled ? (
                        <AlertCircle size={ICON_SIZE.md} color="var(--text-tertiary)" />
                      ) : (
                        <AlertCircle size={ICON_SIZE.md} color="var(--error)" />
                      )}
                      <div>
                        <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
                          {getTierLabel(t, tier).name}
                        </div>
                        <div
                          style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}
                        >
                          {status?.builtin
                            ? t('ocr:status_builtin')
                            : status?.installed
                              ? `${t('ocr:status_installed')} · ${t('ocr:storage_usage_value', { size: tierSize })}`
                              : status?.bundled
                                ? t('ocr:status_bundled')
                                : t('ocr:status_not_installed')}
                        </div>
                      </div>
                    </div>
                    <div style={{ display: 'flex', gap: 8 }}>
                      {status?.bundled && !status?.installed && (
                        <button
                          onClick={() => handleInstallBundled(tier.tier)}
                          disabled={isInstalling}
                          className="interactive-toolbar"
                          style={{
                            padding: '6px 12px',
                            borderRadius: 8,
                            borderWidth: 1,
                            borderStyle: 'solid',
                            fontSize: 'var(--text-caption)',
                            fontWeight: 500,
                            cursor: isInstalling ? 'default' : 'pointer',
                            opacity: isInstalling ? 0.6 : 1,
                            fontFamily: 'inherit',
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: 4,
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {isInstalling
                            ? t('common:loading', { defaultValue: '...' })
                            : t('ocr:install')}
                        </button>
                      )}
                      {!status?.bundled && !status?.installed && (
                        <button
                          onClick={() => handleDownload(tier.tier)}
                          disabled={isDownloading}
                          className="interactive-toolbar"
                          style={{
                            padding: '6px 12px',
                            borderRadius: 8,
                            borderWidth: 1,
                            borderStyle: 'solid',
                            fontSize: 'var(--text-caption)',
                            fontWeight: 500,
                            cursor: isDownloading ? 'default' : 'pointer',
                            opacity: isDownloading ? 0.6 : 1,
                            fontFamily: 'inherit',
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: 4,
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {isDownloading ? (
                            t('common:loading', { defaultValue: '...' })
                          ) : (
                            <>
                              <Download size={ICON_SIZE.sm} />
                              {t('ocr:download')}
                            </>
                          )}
                        </button>
                      )}
                      {/* P133: 系统内置引擎（Vision）不可删除 */}
                      {status?.installed && !status?.builtin && (
                        <button
                          onClick={() => handleDelete(tier.tier)}
                          disabled={isDeleting}
                          className="interactive-danger"
                          style={{
                            padding: '6px 12px',
                            borderRadius: 8,
                            borderWidth: 1,
                            borderStyle: 'solid',
                            fontSize: 'var(--text-caption)',
                            fontWeight: 500,
                            cursor: isDeleting ? 'default' : 'pointer',
                            opacity: isDeleting ? 0.6 : 1,
                            fontFamily: 'inherit',
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: 4,
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {isDeleting ? (
                            t('common:loading', { defaultValue: '...' })
                          ) : (
                            <>
                              <Trash2 size={ICON_SIZE.sm} color="var(--error)" />
                              {t('common:delete')}
                            </>
                          )}
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>

            {!statusMap['small']?.installed && !statusMap['small']?.bundled && (
              <div style={{ marginTop: 12 }}>
                <label
                  style={{
                    display: 'block',
                    fontSize: 'var(--text-caption)',
                    color: 'var(--text-secondary)',
                    marginBottom: 6,
                  }}
                >
                  {t('ocr:download_url_label')}
                </label>
                <input
                  type="text"
                  value={downloadUrl}
                  onChange={(e) => setDownloadUrl(e.target.value)}
                  placeholder={t('ocr:download_url_placeholder')}
                  style={{
                    width: '100%',
                    padding: '8px 10px',
                    fontSize: 'var(--text-body-sm)',
                    borderRadius: 8,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    color: 'var(--text-primary)',
                    transition: 'border-color 0.15s ease',
                  }}
                />
              </div>
            )}
          </Card>
        )}

        {isMobilePlatform && (
          <Card>
            <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 8 }}>
              {t('ocr:mobile_ocr_title')}
            </h3>
            <p
              style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', margin: 0 }}
            >
              {t('ocr:mobile_ocr_description')}
            </p>
          </Card>
        )}
      </PageContainer>
    </AppShell>
  );
}
