import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { useToastError } from '@/hooks/useToastError';
import { useConfirm } from '@/hooks/useConfirm';
import { invoke } from '@tauri-apps/api/core';
import type { OcrTierInfo, OcrModelStatus } from '@/lib/ipc';
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

  const [tiers, setTiers] = useState<OcrTierInfo[]>([]);
  const [activeTier, setActiveTier] = useState('small');
  const [statusMap, setStatusMap] = useState<Record<string, OcrModelStatus>>({});
  const [loading, setLoading] = useState(!isMobilePlatform);
  const [installingTier, setInstallingTier] = useState<string | null>(null);
  const [downloadingTier, setDownloadingTier] = useState<string | null>(null);
  const [deletingTier, setDeletingTier] = useState<string | null>(null);
  const [downloadUrl, setDownloadUrl] = useState('');

  const loadTiersAndStatus = async () => {
    try {
      setLoading(true);
      const [tierList, currentTier] = await Promise.all([
        invoke<OcrTierInfo[]>('ocr_list_available_tiers'),
        invoke<string>('ocr_get_active_tier'),
      ]);
      setTiers(tierList);
      setActiveTier(currentTier);

      const statuses: Record<string, OcrModelStatus> = {};
      await Promise.all(
        tierList.map(async (tier) => {
          const status = await invoke<OcrModelStatus>('ocr_get_model_status', { tier: tier.tier });
          statuses[tier.tier] = status;
        }),
      );
      setStatusMap(statuses);
    } catch (e) {
      onError(e, t('ocr:load_status_failed'));
    } finally {
      setLoading(false);
    }
  };

  // P212: mount-only init — loadTiersAndStatus/onError/t are stable, omitted intentionally.
  useEffect(() => {
    if (!isMobilePlatform) {
      loadTiersAndStatus();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleTierChange = async (tier: string) => {
    try {
      await invoke<void>('ocr_set_active_tier', { tier });
      setActiveTier(tier);
      onSuccess(t('ocr:set_tier_success', { tier }));
    } catch (e) {
      onError(e, t('ocr:set_tier_failed'));
    }
  };

  const handleInstallBundled = async (tier: string) => {
    setInstallingTier(tier);
    try {
      await invoke<void>('ocr_install_bundled_model', { tier });
      await loadTiersAndStatus();
      onSuccess(t('ocr:install_success', { tier }));
    } catch (e) {
      onError(e, t('ocr:install_failed', { tier }));
    } finally {
      setInstallingTier(null);
    }
  };

  const handleDelete = async (tier: string) => {
    setDeletingTier(tier);
    try {
      await invoke<void>('ocr_delete_model', { tier });
      await loadTiersAndStatus();
      onSuccess(t('ocr:delete_success', { tier }));
    } catch (e) {
      onError(e, t('ocr:delete_failed', { tier }));
    } finally {
      setDeletingTier(null);
    }
  };

  const handleDownload = async (tier: string) => {
    if (!downloadUrl.trim()) {
      onError(new Error(t('ocr:download_url_required')), t('ocr:download_url_required'));
      return;
    }
    const size = tier === 'tiny' ? '1.5MB' : tier === 'medium' ? '132MB' : '30MB';
    requestConfirm(
      t('ocr:confirm_download_title'),
      t('ocr:confirm_download_message', { tier, size }),
      async () => {
        setDownloadingTier(tier);
        try {
          await invoke<void>('ocr_download_model', { tier, baseUrl: downloadUrl.trim() });
          await loadTiersAndStatus();
          onSuccess(t('ocr:download_success', { tier }));
        } catch (e) {
          onError(e, t('ocr:download_failed', { tier }));
        } finally {
          setDownloadingTier(null);
        }
      },
      { confirmLabel: t('ocr:confirm_download_ok'), cancelLabel: t('common:cancel') },
    );
  };

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
                const tierSize =
                  tier.tier === 'tiny' ? '1.5MB' : tier.tier === 'medium' ? '132MB' : '30MB';
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
                          {status?.installed
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
                          onMouseEnter={(e) => {
                            if (!isInstalling) {
                              e.currentTarget.style.background =
                                'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                              e.currentTarget.style.borderColor = 'var(--accent-primary)';
                              e.currentTarget.style.color = 'var(--accent-primary)';
                            }
                          }}
                          onMouseLeave={(e) => {
                            if (!isInstalling) {
                              e.currentTarget.style.background = 'var(--bg-toolbar)';
                              e.currentTarget.style.borderColor = 'var(--border-subtle)';
                              e.currentTarget.style.color = 'var(--text-primary)';
                            }
                          }}
                          style={{
                            padding: '6px 12px',
                            borderRadius: 8,
                            border: '1px solid var(--border-subtle)',
                            background: 'var(--bg-toolbar)',
                            color: 'var(--text-primary)',
                            fontSize: 'var(--text-caption)',
                            fontWeight: 500,
                            cursor: isInstalling ? 'default' : 'pointer',
                            opacity: isInstalling ? 0.6 : 1,
                            transition: 'all 0.15s ease',
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
                          onMouseEnter={(e) => {
                            if (!isDownloading) {
                              e.currentTarget.style.background =
                                'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                              e.currentTarget.style.borderColor = 'var(--accent-primary)';
                              e.currentTarget.style.color = 'var(--accent-primary)';
                            }
                          }}
                          onMouseLeave={(e) => {
                            if (!isDownloading) {
                              e.currentTarget.style.background = 'var(--bg-toolbar)';
                              e.currentTarget.style.borderColor = 'var(--border-subtle)';
                              e.currentTarget.style.color = 'var(--text-primary)';
                            }
                          }}
                          style={{
                            padding: '6px 12px',
                            borderRadius: 8,
                            border: '1px solid var(--border-subtle)',
                            background: 'var(--bg-toolbar)',
                            color: 'var(--text-primary)',
                            fontSize: 'var(--text-caption)',
                            fontWeight: 500,
                            cursor: isDownloading ? 'default' : 'pointer',
                            opacity: isDownloading ? 0.6 : 1,
                            transition: 'all 0.15s ease',
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
                      {status?.installed && (
                        <button
                          onClick={() => handleDelete(tier.tier)}
                          disabled={isDeleting}
                          onMouseEnter={(e) => {
                            if (!isDeleting) {
                              e.currentTarget.style.background =
                                'color-mix(in srgb, var(--error) 12%, transparent)';
                              e.currentTarget.style.borderColor = 'var(--error)';
                              e.currentTarget.style.color = 'var(--error)';
                            }
                          }}
                          onMouseLeave={(e) => {
                            if (!isDeleting) {
                              e.currentTarget.style.background = 'var(--bg-toolbar)';
                              e.currentTarget.style.borderColor = 'var(--border-subtle)';
                              e.currentTarget.style.color = 'var(--text-primary)';
                            }
                          }}
                          style={{
                            padding: '6px 12px',
                            borderRadius: 8,
                            border: '1px solid var(--border-subtle)',
                            background: 'var(--bg-toolbar)',
                            color: 'var(--text-primary)',
                            fontSize: 'var(--text-caption)',
                            fontWeight: 500,
                            cursor: isDeleting ? 'default' : 'pointer',
                            opacity: isDeleting ? 0.6 : 1,
                            transition: 'all 0.15s ease',
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
