import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { OcrTierStatusRow } from '@/components/ocr/OcrTierStatusRow';
import { getTierLabel } from '@/lib/utils';
import { isMacOSSync } from '@/lib/platform';
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
  // 骨架行数与后端 tier 行数一致（macOS 4 行，其他桌面 3 行），保证骨架高度 == 内容高度
  const rowCount = isMacOSSync() ? 4 : 3;
  return (
    <>
      {/* Model management（桌面端）；移动端使用系统 ML Kit，无需模型管理 */}
      {!isMobilePlatform && (
        <Card>
          <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
            {t('ocr:model_title')}
          </h3>

          {/* 模型状态加载期骨架：结构与真实内容同构（真实 select + 同高行骨架），
              浏览器自然算出与内容一致的高度，避免卡片展开时把下方扫描按钮推下
              （布局跳动/一闪而过）。加载完成后内容原地就位。 */}
          {loadingStatus ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              {/* select 骨架：与真实 select 同高（padding 8×2 + 行高 ~17 + 边框 2），无原生下拉箭头 */}
              <div
                aria-hidden
                style={{
                  height: 35,
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                }}
              />
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {Array.from({ length: rowCount }).map((_, i) => (
                  <div
                    key={i}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      padding: '10px 12px',
                      borderRadius: 8,
                      background: 'var(--bg-toolbar)',
                    }}
                  >
                    <div style={{ flex: 1 }}>
                      <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>&nbsp;</div>
                      <div style={{ fontSize: 'var(--text-badge)' }}>&nbsp;</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ) : (
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
                      onInstall={
                        status?.bundled && !status?.installed
                          ? () => onInstallBundled(tier.tier)
                          : undefined
                      }
                      onDownload={
                        !status?.bundled && !status?.installed
                          ? () => onDownload(tier.tier)
                          : undefined
                      }
                    />
                  );
                })}
              </div>{' '}
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
          )}
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
