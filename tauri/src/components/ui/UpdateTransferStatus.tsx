import { useTranslation } from 'react-i18next';
import { formatBytes } from '@/lib/utils';
import type { UpdateTransferInfo } from '@/lib/updater';

/** 下载线路只展示 host，防止未来传入完整 URL 时泄露路径或查询参数。 */
function sourceHost(source: string | undefined): string {
  if (!source) return '';
  try {
    return new URL(source.includes('://') ? source : `https://${source}`).hostname;
  } catch {
    return '';
  }
}

export function UpdateTransferStatus({
  transfer,
  className,
}: {
  transfer?: UpdateTransferInfo;
  className?: string;
}) {
  const { t } = useTranslation('common');
  if (!transfer) return null;
  const host = sourceHost(transfer.source);
  const speed = transfer.bytesPerSecond;
  const details = [];
  if (transfer.phase === 'probing') details.push(t('update_selecting_source'));
  if (transfer.phase === 'switching') details.push(t('update_switching_source'));
  if (transfer.phase !== 'probing' && host) {
    details.push(t('update_download_source', { source: host }));
  }
  if (transfer.phase === 'downloading' && typeof speed === 'number' && Number.isFinite(speed)) {
    details.push(t('update_download_speed', { speed: formatBytes(Math.max(0, speed)) }));
  }
  if (details.length === 0) return null;
  return (
    <span
      className={className}
      data-update-transfer-phase={transfer.phase}
      style={{ minWidth: 0, overflowWrap: 'anywhere', fontVariantNumeric: 'tabular-nums' }}
    >
      {details.join(' · ')}
    </span>
  );
}
