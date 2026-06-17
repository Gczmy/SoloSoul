import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useToastError } from '@/hooks/useToastError';
import { commands, type OcrTierInfo, type OcrModelStatus } from '@/lib/ipc';
import { getTierLabel } from '@/lib/ocr';
import { Download, CheckCircle, AlertCircle } from 'lucide-react';

export function OcrSettingsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['ocr', 'settings', 'common']);
  const { onError, onSuccess } = useToastError();

  const [tiers, setTiers] = useState<OcrTierInfo[]>([]);
  const [activeTier, setActiveTier] = useState('small');
  const [statusMap, setStatusMap] = useState<Record<string, OcrModelStatus>>({});
  const [loading, setLoading] = useState(true);
  const [installingTier, setInstallingTier] = useState<string | null>(null);
  const [downloadingTier, setDownloadingTier] = useState<string | null>(null);
  const [downloadUrl, setDownloadUrl] = useState('');

  const loadTiersAndStatus = async () => {
    try {
      setLoading(true);
      const [tierList, currentTier] = await Promise.all([
        commands.ocrListAvailableTiers(),
        commands.ocrGetActiveTier(),
      ]);
      setTiers(tierList);
      setActiveTier(currentTier);

      const statuses: Record<string, OcrModelStatus> = {};
      await Promise.all(
        tierList.map(async (tier) => {
          const status = await commands.ocrGetModelStatus(tier.tier);
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

  useEffect(() => {
    loadTiersAndStatus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleTierChange = async (tier: string) => {
    try {
      await commands.ocrSetActiveTier(tier);
      setActiveTier(tier);
      onSuccess(t('ocr:set_tier_success', { tier }));
    } catch (e) {
      onError(e, t('ocr:set_tier_failed'));
    }
  };

  const handleInstallBundled = async (tier: string) => {
    setInstallingTier(tier);
    try {
      await commands.ocrInstallBundledModel(tier);
      await loadTiersAndStatus();
      onSuccess(t('ocr:install_success', { tier }));
    } catch (e) {
      onError(e, t('ocr:install_failed', { tier }));
    } finally {
      setInstallingTier(null);
    }
  };

  const handleDownload = async (tier: string) => {
    if (!downloadUrl.trim()) {
      onError(new Error(t('ocr:download_url_required')), t('ocr:download_url_required'));
      return;
    }
    setDownloadingTier(tier);
    try {
      await commands.ocrDownloadModel(tier, downloadUrl.trim());
      await loadTiersAndStatus();
      onSuccess(t('ocr:download_success', { tier }));
    } catch (e) {
      onError(e, t('ocr:download_failed', { tier }));
    } finally {
      setDownloadingTier(null);
    }
  };

  return (
    <AppShell title={t('ocr:settings_title')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 640, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
            {t('ocr:active_model')}
          </h3>
          <select
            value={activeTier}
            onChange={(e) => handleTierChange(e.target.value)}
            disabled={loading}
            style={{
              width: '100%',
              padding: '8px 10px',
              fontSize: 13,
              borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
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

        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
            {t('ocr:model_management')}
          </h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {tiers.map((tier) => {
              const status = statusMap[tier.tier];
              const isInstalling = installingTier === tier.tier;
              const isDownloading = downloadingTier === tier.tier;
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
                      <CheckCircle size={16} color="var(--accent-primary)" />
                    ) : status?.bundled ? (
                      <AlertCircle size={16} color="var(--text-tertiary)" />
                    ) : (
                      <AlertCircle size={16} color="var(--error)" />
                    )}
                    <div>
                      <div style={{ fontSize: 13, fontWeight: 500 }}>
                        {getTierLabel(t, tier).name}
                      </div>
                      <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                        {status?.installed
                          ? t('ocr:status_installed')
                          : status?.bundled
                            ? t('ocr:status_bundled')
                            : t('ocr:status_not_installed')}
                      </div>
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: 8 }}>
                    {status?.bundled && !status?.installed && (
                      <Button
                        size="sm"
                        onClick={() => handleInstallBundled(tier.tier)}
                        loading={isInstalling}
                      >
                        {t('ocr:install')}
                      </Button>
                    )}
                    {!status?.bundled && !status?.installed && (
                      <Button
                        size="sm"
                        onClick={() => handleDownload(tier.tier)}
                        loading={isDownloading}
                      >
                        <Download size={14} style={{ marginRight: 4 }} />
                        {t('ocr:download')}
                      </Button>
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
                  fontSize: 12,
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
                  fontSize: 13,
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  color: 'var(--text-primary)',
                }}
              />
            </div>
          )}
        </Card>
      </div>
    </AppShell>
  );
}
