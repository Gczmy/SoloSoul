import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { OcrTierStatusRow } from '@/components/ocr/OcrTierStatusRow';
import { getTierLabel } from '@/lib/utils';
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
  // 状态加载完成（statusMap['small'] 存在）后才判断——否则加载瞬间会误显下载 URL
  // 输入框（含 https placeholder）造成闪烁
  const showDownloadUrl =
    !!statusMap['small'] && !statusMap['small'].installed && !statusMap['small'].bundled;
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
                const statusText = status?.builtin
                  ? t('ocr:status_builtin')
                  : status?.installed
                    ? t('ocr:status_installed')
                    : status?.bundled
                      ? t('ocr:status_bundled')
                      : t('ocr:status_not_installed');
                return (
                  <OcrTierStatusRow
                    key={tier.tier}
                    tierKey={tier.tier}
                    status={status}
                    label={getTierLabel(t, tier).name}
                    statusText={statusText}
                    isInstalling={installingTier === tier.tier}
                    isDownloading={downloadingTier === tier.tier}
                    onInstall={status?.bundled && !status?.installed ? () => onInstallBundled(tier.tier) : undefined}
                    onDownload={
                      !status?.bundled && !status?.installed ? () => onDownload(tier.tier) : undefined
                    }
                  />
                );
              })}
            </div>

            {showDownloadUrl && (
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
