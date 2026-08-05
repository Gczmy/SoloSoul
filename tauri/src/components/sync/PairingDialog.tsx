import { useTranslation } from 'react-i18next';
import { ShieldAlert, Smartphone, Loader2, AlertTriangle } from 'lucide-react';
import { Dialog } from '@/components/ui/Dialog';
import { Button } from '@/components/ui/Button';
import type { SyncPeer } from '@/stores/syncStore';
import { formatPeerName } from '@/lib/syncPeer';
import { ICON_SIZE } from '@/lib/constants';

interface PairingDialogProps {
  isOpen: boolean;
  peer: SyncPeer | null;
  onTrust: () => void;
  onIgnore: () => void;
  /** 等待对端确认的等待态（A 侧确认信任后自动重试中）。 */
  waiting?: boolean;
  /** 等待超时失败（对方尚未确认）。 */
  waitFailed?: boolean;
  /** 等待/失败态下的取消（停止自动重试并关闭）。 */
  onCancelWaiting?: () => void;
  /** 确认按钮文案（A 侧发起方等待流程用「确认并等待」，默认「信任并配对」）。 */
  confirmLabel?: string;
}

/**
 * 配对确认对话框。
 *
 * - 默认（pair）：B 侧或已知设备手动配对，展示指纹供核对，确认 = 信任该设备。
 * - 等待（waiting）：A 侧发起方确认信任后进入等待态，自动重试同步，等待 B 接受配对请求。
 * - 失败（waitFailed）：等待超时，提示「对方尚未确认」。
 */
export function PairingDialog({
  isOpen,
  peer,
  onTrust,
  onIgnore,
  waiting = false,
  waitFailed = false,
  onCancelWaiting,
  confirmLabel,
}: PairingDialogProps) {
  const { t } = useTranslation(['settings', 'common']);
  if (!peer) return null;

  const displayName = formatPeerName(peer);

  return (
    <Dialog
      isOpen={isOpen}
      onClose={waiting || waitFailed ? onCancelWaiting || onIgnore : onIgnore}
      title={t('settings:sync_pairing_title', { defaultValue: 'Pair New Device' })}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
        {/* 等待/失败态顶部提示 */}
        {waiting && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: 12,
              borderRadius: 10,
              background: 'color-mix(in srgb, var(--accent-primary) 8%, transparent)',
              color: 'var(--accent-primary)',
            }}
          >
            <Loader2 size={ICON_SIZE['2xl']} className="animate-spin" />
            <span style={{ fontSize: 'var(--text-body-sm)', lineHeight: 1.5 }}>
              {t('settings:sync_pairing_waiting', {
                defaultValue: '已确认信任，等待对端设备接受配对请求…（自动重试中）',
              })}
            </span>
          </div>
        )}
        {waitFailed && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: 12,
              borderRadius: 10,
              background: 'var(--danger-subtle)',
              color: 'var(--danger)',
            }}
          >
            <AlertTriangle size={ICON_SIZE['2xl']} />
            <span style={{ fontSize: 'var(--text-body-sm)', lineHeight: 1.5 }}>
              {t('settings:sync_pairing_wait_failed', {
                defaultValue: '对方尚未确认配对，可稍后重试',
              })}
            </span>
          </div>
        )}

        {/* 非等待态的安全警告 */}
        {!waiting && !waitFailed && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: 12,
              borderRadius: 10,
              background: 'var(--danger-subtle)',
              color: 'var(--danger)',
            }}
          >
            <ShieldAlert size={ICON_SIZE['2xl']} />
            <span style={{ fontSize: 'var(--text-body-sm)', lineHeight: 1.5 }}>
              {t('settings:sync_pairing_warning', {
                defaultValue:
                  'Only trust devices you physically own or control. An attacker could impersonate your device.',
              })}
            </span>
          </div>
        )}

        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <div
            style={{
              width: 40,
              height: 40,
              borderRadius: 10,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: 'var(--bg-toolbar)',
            }}
          >
            <Smartphone size={ICON_SIZE.xl} style={{ color: 'var(--accent-primary)' }} />
          </div>
          <div>
            <div style={{ fontSize: 'var(--text-card-title)', fontWeight: 600 }}>
              {displayName}
            </div>
            <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
              {peer.addr || 'offline'}
            </div>
          </div>
        </div>

        {/* 等待态：提示对端操作，不展示核对指纹（A 侧已确认） */}
        {!waiting && !waitFailed && (
          <>
            <div>
              <div
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-secondary)',
                  marginBottom: 6,
                  textAlign: 'center',
                }}
              >
                {peer.sasCode
                  ? t('settings:sync_pairing_sas_prompt', {
                      defaultValue:
                        'Compare the verification code below with the other device. They must match:',
                    })
                  : t('settings:sync_pairing_verify_prompt', {
                      defaultValue:
                        'Verify the fingerprint below matches the one shown on the other device:',
                    })}
              </div>
              {peer.sasCode ? (
                /* SAS 配对验证码：6 位数字 3-3 分块，大号展示便于目视比对 */
                <div
                  style={{
                    padding: 16,
                    borderRadius: 10,
                    background: 'var(--bg-toolbar)',
                    border: '1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)',
                    textAlign: 'center',
                    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
                    fontSize: 'var(--text-card-title)',
                    fontWeight: 700,
                    letterSpacing: 8,
                    color: 'var(--accent-primary)',
                    fontVariantNumeric: 'tabular-nums',
                  }}
                >
                  {`${peer.sasCode.slice(0, 3)} · ${peer.sasCode.slice(3, 6)}`}
                </div>
              ) : (
                <div
                  style={{
                    padding: 12,
                    borderRadius: 8,
                    background: 'var(--bg-toolbar)',
                    fontFamily: 'monospace',
                    fontSize: 'var(--text-caption)',
                    wordBreak: 'break-all',
                    color: 'var(--text-primary)',
                  }}
                >
                  {peer.fingerprint ||
                    t('settings:sync_pairing_no_fingerprint', {
                      defaultValue: 'No fingerprint available',
                    })}
                </div>
              )}
            </div>

            {!waiting && !waitFailed && (
              <div
                style={{
                  padding: 10,
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-secondary)',
                  lineHeight: 1.5,
                }}
              >
                {peer.sasCode
                  ? t('settings:sync_pairing_sas_hint', {
                      defaultValue:
                        '两台设备显示的验证码一致即确认设备无误；确认后请在对端设备上接受配对请求',
                    })
                  : t('settings:sync_pairing_confirm_hint', {
                      defaultValue: '核对指纹后确认配对，并请在对端设备上接受配对请求',
                    })}
              </div>
            )}
          </>
        )}

        <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end', marginTop: 8 }}>
          {waiting || waitFailed ? (
            <Button variant="secondary" onClick={onCancelWaiting || onIgnore}>
              {t('settings:sync_pairing_wait_cancel', { defaultValue: '取消等待' })}
            </Button>
          ) : (
            <>
              <Button variant="secondary" onClick={onIgnore}>
                {t('settings:sync_pairing_ignore', { defaultValue: 'Ignore' })}
              </Button>
              <Button onClick={onTrust}>
                {confirmLabel ||
                  t('settings:sync_pairing_trust', { defaultValue: 'Trust & Pair' })}
              </Button>
            </>
          )}
        </div>
      </div>
    </Dialog>
  );
}
