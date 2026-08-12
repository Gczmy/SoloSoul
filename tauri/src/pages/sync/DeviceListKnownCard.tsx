import type { TFunction } from 'i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { RefreshCw, ShieldCheck, ShieldOff } from 'lucide-react';
import { formatPeerName } from '@/lib/syncPeer';
import { formatRelativeFromTs } from '@/lib/time';
import { ICON_SIZE } from '@/lib/constants';
import { DeviceCardShell } from './DeviceCard';
import type { SyncPeer } from '@/stores/syncStore';

/**
 * DeviceListPanel 的「已知设备」卡片（P046 拆分：展示子组件）。
 * 信任徽章、在线状态、信任/配对/忘记操作。
 */
export function DeviceListKnownCard({
  connectedPeers,
  isLoading,
  onRefresh,
  onTrustPeer,
  onOpenPairTarget,
  onForgetRequest,
  onOpenDetail,
  t,
}: {
  connectedPeers: SyncPeer[];
  isLoading: boolean;
  onRefresh: () => void;
  onTrustPeer: (peerId: string) => void;
  onOpenPairTarget: (peer: SyncPeer) => void;
  onForgetRequest: (peer: SyncPeer) => void;
  onOpenDetail: (peer: SyncPeer) => void;
  t: TFunction;
}) {
  return (
    <Card>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>
          {t('settings:sync_known_devices_title', { defaultValue: 'Known Devices' })}
        </h3>
        <Button size="sm" variant="tertiary" onClick={onRefresh} loading={isLoading}>
          <RefreshCw size={ICON_SIZE.sm} />
        </Button>
      </div>
      <p
        style={{
          fontSize: 'var(--text-caption)',
          color: 'var(--text-tertiary)',
          marginTop: 8,
          marginBottom: 12,
        }}
      >
        {t('settings:sync_known_devices_hint', {
          defaultValue:
            'Devices you have discovered or connected to before; only trusted devices can sync.',
        })}
      </p>

      {connectedPeers.length > 0 ? (
        <div style={{ marginTop: 12, display: 'flex', flexDirection: 'column', gap: 8 }}>
          {connectedPeers.map((peer) => {
            const displayName = formatPeerName(peer);
            // P012: 共享外壳（交互容器 + 图标 + 名称行），副标题/操作区注入
            return (
              <DeviceCardShell
                key={peer.id}
                clientType={peer.clientType}
                name={displayName}
                subtitle={
                  <>
                    {/* 信任徽章独立一行（设备名下方） */}
                    <div style={{ marginTop: 2 }}>
                      <span
                        style={{
                          fontSize: 'var(--text-badge)',
                          padding: '1px 8px',
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
                    {/* 在线状态（i18n：offline/never）——不再展示指纹。
                        离线含义修正（P0#2）：addr 为空 = 未在局域网发现（非「设备关机/离线」），
                        附最近一次联系时间（lastSeenTs）帮助用户判断。 */}
                    <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                      {peer.addr ? (
                        `${peer.addr} · ${peer.lastSeen || t('settings:sync_never', { defaultValue: 'never' })}`
                      ) : (
                        <>
                          {t('settings:sync_offline', { defaultValue: 'Not found on LAN' })}
                          {peer.lastSeenTs
                            ? ` · ${t('settings:sync_last_seen', {
                                defaultValue: 'Last seen: {{time}}',
                                time: formatRelativeFromTs(peer.lastSeenTs),
                              })}`
                            : ''}
                        </>
                      )}
                    </div>
                  </>
                }
                actions={
                  <>
                    {peer.trusted ? (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={(e) => {
                          e.stopPropagation();
                          onTrustPeer(peer.id);
                        }}
                        title={t('settings:sync_revoke_tooltip', {
                          defaultValue: 'Revoke trust: keep the record, reject its syncs',
                        })}
                      >
                        <ShieldOff size={ICON_SIZE.sm} />
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={(e) => {
                          e.stopPropagation();
                          onOpenPairTarget(peer);
                        }}
                        title={t('settings:sync_pair_tooltip', {
                          defaultValue: 'Pair this device',
                        })}
                      >
                        <ShieldCheck size={ICON_SIZE.sm} />
                      </Button>
                    )}
                    <DeleteButton
                      onClick={(e) => {
                        e.stopPropagation();
                        onForgetRequest(peer);
                      }}
                      title={t('settings:sync_forget_tooltip', {
                        defaultValue: 'Forget: delete the record, you will need to re-pair',
                      })}
                      iconOnly
                    />
                  </>
                }
                onOpen={() => onOpenDetail(peer)}
              />
            );
          })}
        </div>
      ) : (
        <p
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-tertiary)',
            marginTop: 8,
          }}
        >
          {t('settings:sync_no_devices', {
            defaultValue:
              'No devices known yet. Enable sync and sync with another device to add it.',
          })}
        </p>
      )}
    </Card>
  );
}
