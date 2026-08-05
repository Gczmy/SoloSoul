import { useTranslation } from 'react-i18next';
import { Dialog } from '@/components/ui/Dialog';
import { Button } from '@/components/ui/Button';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { ShieldCheck, ShieldOff } from 'lucide-react';
import { ClientTypeIcon } from '@/components/sync/ClientTypeIcon';
import { formatPeerName } from '@/lib/syncPeer';
import { ICON_SIZE } from '@/lib/constants';
import type { SyncPeer } from '@/stores/syncStore';

interface DeviceDetailDialogProps {
  peer: SyncPeer | null;
  onClose: () => void;
  /** 切换信任状态（trusted → 撤销；未信任 → 配对）。 */
  onToggleTrust: (peer: SyncPeer) => void;
  onForgetRequest: (peer: SyncPeer) => void;
  /** 信任/撤销等异步操作在途：禁用操作按钮并显示加载态，避免重复点击。 */
  isLoading?: boolean;
}

function formatTime(ts?: number | null): string {
  if (!ts) {
    return '—';
  }
  const date = new Date(ts * 1000);
  if (Number.isNaN(date.getTime())) {
    return '—';
  }
  return date.toLocaleString();
}

/**
 * 已知设备详情弹窗：设备名/信任徽章/在线状态/指纹/host:port/信任时间/同步时间/客户端类型。
 * 列表卡片仅展示概要（点击展开），完整信息与操作按钮收敛于此。
 */
export function DeviceDetailDialog({
  peer,
  onClose,
  onToggleTrust,
  onForgetRequest,
  isLoading = false,
}: DeviceDetailDialogProps) {
  const { t } = useTranslation(['settings']);
  if (!peer) {
    return null;
  }
  const displayName = formatPeerName(peer);
  const clientType = peer.clientType || 'unknown';
  const statusText = peer.addr
    ? `${peer.addr} · ${
        peer.lastSeenTs
          ? t('settings:sync_last_seen', {
              defaultValue: 'Last seen: {{time}}',
              time: formatTime(peer.lastSeenTs),
            })
          : t('settings:sync_never', { defaultValue: 'never' })
      }`
    : t('settings:sync_offline', { defaultValue: 'offline' });

  return (
    <Dialog
      isOpen={!!peer}
      onClose={onClose}
      title={t('settings:sync_device_details', { defaultValue: 'Device details' })}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
        {/* 设备名 + 信任徽章 */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <div
            style={{
              width: 44,
              height: 44,
              borderRadius: 12,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: 'var(--bg-toolbar)',
              flexShrink: 0,
            }}
          >
            <ClientTypeIcon clientType={clientType} size={ICON_SIZE.lg} />
          </div>
          <div style={{ minWidth: 0 }}>
            <div
              style={{
                fontSize: 'var(--text-card-title)',
                fontWeight: 600,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {displayName}
            </div>
            <div style={{ marginTop: 2 }}>
              <span
                style={{
                  fontSize: 'var(--text-badge)',
                  padding: '2px 8px',
                  borderRadius: 999,
                  background: peer.trusted
                    ? 'rgba(39,174,96,0.12)'
                    : 'rgba(128,128,128,0.1)',
                  color: peer.trusted ? '#27ae60' : 'var(--text-tertiary)',
                  whiteSpace: 'nowrap',
                }}
              >
                {peer.trusted
                  ? t('settings:sync_trusted_badge', { defaultValue: 'Trusted' })
                  : t('settings:sync_untrusted_badge', { defaultValue: 'Not trusted' })}
              </span>
            </div>
          </div>
        </div>

        {/* 状态行 */}
        <div
          style={{
            padding: 10,
            borderRadius: 8,
            background: 'var(--bg-toolbar)',
            fontSize: 'var(--text-caption)',
            color: 'var(--text-secondary)',
          }}
        >
          {statusText}
        </div>

        {/* 详情行 */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <div style={{ fontSize: 'var(--text-caption)' }}>
            <strong>
              {t('settings:sync_forget_confirm_fp', { defaultValue: 'Fingerprint' })}:
            </strong>{' '}
            <span style={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
              {peer.fingerprint || '-'}
            </span>
          </div>
          <div style={{ fontSize: 'var(--text-caption)' }}>
            <strong>{t('settings:sync_device_addr', { defaultValue: 'Address' })}:</strong>{' '}
            {peer.addr || t('settings:sync_offline', { defaultValue: 'offline' })}
          </div>
          <div style={{ fontSize: 'var(--text-caption)' }}>
            <strong>{t('settings:sync_device_trusted_at', { defaultValue: 'Trusted at' })}:</strong>{' '}
            {formatTime(peer.trustedAt)}
          </div>
          <div style={{ fontSize: 'var(--text-caption)' }}>
            <strong>{t('settings:sync_device_last_sync', { defaultValue: 'Last sync' })}:</strong>{' '}
            {formatTime(peer.lastSeenTs)}
          </div>
          <div style={{ fontSize: 'var(--text-caption)' }}>
            <strong>{t('settings:sync_device_client_type', { defaultValue: 'Client type' })}:</strong>{' '}
            {t(`settings:sync_client_${clientType}`, {
              defaultValue: clientType === 'unknown' ? 'Unknown' : clientType,
            })}
          </div>
        </div>

        {/* 操作 */}
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 4 }}>
          {peer.trusted ? (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => onToggleTrust(peer)}
              disabled={isLoading}
              loading={isLoading}
            >
              <ShieldOff size={ICON_SIZE.sm} />
              {t('settings:sync_revoke_tooltip', { defaultValue: 'Revoke trust' })}
            </Button>
          ) : (
            <Button
              variant="primary"
              size="sm"
              onClick={() => onToggleTrust(peer)}
              disabled={isLoading}
              loading={isLoading}
            >
              <ShieldCheck size={ICON_SIZE.sm} />
              {t('settings:sync_pair_tooltip', { defaultValue: 'Pair this device' })}
            </Button>
          )}
          <DeleteButton
            onClick={() => onForgetRequest(peer)}
            disabled={isLoading}
            title={t('settings:sync_forget_tooltip', {
              defaultValue: 'Forget: delete the record, you will need to re-pair',
            })}
          >
            {t('settings:sync_forget', { defaultValue: 'Forget' })}
          </DeleteButton>
        </div>
      </div>
    </Dialog>
  );
}
