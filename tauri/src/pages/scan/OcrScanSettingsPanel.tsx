import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { AlertCircle, CheckCircle, Download } from 'lucide-react';
import { getTierLabel } from '@/lib/utils';
import { ICON_SIZE } from '@/lib/constants';
import type { OcrModelStatus, OcrTierInfo } from '@/lib/ipc';

interface OcrScanSettingsPanelProps {
  isMobilePlatform: boolean;
  tiers: OcrTierInfo[];
  activeTier: string;
  statusMap: Record<string, OcrModelStatus>;
  loadingStatus: boolean;
  installingTier: string | null;
  downloadingTier: string | null;
  downloadUrl: string;
  onDownloadUrlChange: (value: string) => void;
  onTierChange: (tier: string) => void;
  onInstallBundled: (tier: string) => void;
  onDownload: (tier: string) => void;
}

/**
 * OCR 设置面板：模型档位选择/安装/下载 + 移动端系统 OCR 说明。
 * 数据与回调经 OcrPage 从 useOcrModelManager 透传（P224-⑤ 拆分）。
 */
export function OcrScanSettingsPanel({
  isMobilePlatform,
  tiers,
  activeTier,
  statusMap,
  loadingStatus,
  installingTier,
  downloadingTier,
  downloadUrl,
  onDownloadUrlChange,
  onTierChange,
  onInstallBundled,
  onDownload,
}: OcrScanSettingsPanelProps) {
  const { t } = useTranslation(['ocr', 'common']);
  return (
    <>
      {/* Model management（桌面端）；移动端使用系统 ML Kit，无需模型管理 */}
      {!isMobilePlatform && (
        <Card>
          <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
            {t('ocr:model_title')}
          </h3>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div>
              <select
                value={activeTier}
                onChange={(e) => onTierChange(e.target.value)}
                disabled={loadingStatus}
                style={{
                  width: '100%',
                  padding: '8px 10px',
                  fontSize: 'var(--text-body-sm)',
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
            </div>

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
                          onClick={() => onInstallBundled(tier.tier)}
                          loading={isInstalling}
                        >
                          {t('ocr:install')}
                        </Button>
                      )}
                      {!status?.bundled && !status?.installed && (
                        <Button
                          size="sm"
                          onClick={() => onDownload(tier.tier)}
                          loading={isDownloading}
                        >
                          <Download size={ICON_SIZE.sm} style={{ marginRight: 4 }} />
                          {t('ocr:download')}
                        </Button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>

            {!statusMap['small']?.installed && !statusMap['small']?.bundled && (
              <div>
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
                  onChange={(e) => onDownloadUrlChange(e.target.value)}
                  placeholder={t('ocr:download_url_placeholder')}
                  style={{
                    width: '100%',
                    padding: '8px 10px',
                    fontSize: 'var(--text-body-sm)',
                    borderRadius: 8,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-elevated)',
                    color: 'var(--text-primary)',
                  }}
                />
              </div>
            )}
          </div>
        </Card>
      )}

      {isMobilePlatform && (
        <Card>
          <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 8 }}>
            {t('ocr:mobile_ocr_title')}
          </h3>
          <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', margin: 0 }}>
            {t('ocr:mobile_ocr_description')}
          </p>
        </Card>
      )}
    </>
  );
}
