import { QRCodeSVG } from 'qrcode.react';
import { motion } from 'framer-motion';
import { Copy, Check, ChevronDown, ChevronUp } from 'lucide-react';
import type { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import { QrStatusBlock } from './QrStatusBlock';

/** 恢复主机会话数据（recovery_host_start 返回值）。 */
export interface RecoveryHostInfo {
  displayAddr: string;
  bindAddr: string;
  pin: string;
  qrPayload: string;
}

type T = ReturnType<typeof useTranslation>['t'];

/** 恢复二维码内容区：标题 + 加载/错误占位 + 二维码/网络信息/手动输入指引。 */
export function RecoveryQrContent({
  t,
  loading,
  error,
  info,
  manualOpen,
  copiedAddr,
  copiedPin,
  onToggleManual,
  onCopyAddr,
  onCopyPin,
  onCancel,
}: {
  t: T;
  loading: boolean;
  error: string | null;
  info: RecoveryHostInfo | null;
  manualOpen: boolean;
  copiedAddr: boolean;
  copiedPin: boolean;
  onToggleManual: () => void;
  onCopyAddr: () => void;
  onCopyPin: () => void;
  onCancel: () => void;
}) {
  return (
    <>
      <h2
        style={{
          fontSize: 'var(--text-card-title)',
          fontWeight: 700,
          margin: '0 0 8px',
          color: 'var(--text-primary)',
        }}
      >
        {t('common:recovery_host_title')}
      </h2>
      <p
        style={{
          fontSize: 'var(--text-body-sm)',
          color: 'var(--text-secondary)',
          margin: '0 0 20px',
          lineHeight: 1.5,
        }}
      >
        {t('common:recovery_host_desc')}
      </p>

      <QrStatusBlock loading={loading} error={error} t={t} />

      {!loading && !error && info && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.2 }}
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 16,
            alignItems: 'center',
            minHeight: 360,
          }}
        >
          {/* QR 码 */}
          <div
            style={{
              padding: 12,
              background: '#fff',
              borderRadius: 12,
              border: '1px solid var(--border-subtle)',
            }}
          >
            <QRCodeSVG value={info.qrPayload} size={200} level="M" includeMargin />
          </div>
          <p
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
              margin: 0,
              textAlign: 'center',
            }}
          >
            {t('common:recovery_host_qr_hint', {
              defaultValue: 'Scan with the other device to connect automatically',
            })}
          </p>

          {/* 网络信息 */}
          <div style={{ width: '100%' }}>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '10px 12px',
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                marginBottom: 8,
              }}
            >
              <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                {t('common:recovery_host_pin_label')}
              </span>
              <span
                style={{
                  fontFamily: 'monospace',
                  fontSize: 'var(--text-body)',
                  fontWeight: 700,
                  letterSpacing: 4,
                  color: 'var(--accent-primary)',
                }}
              >
                {info.pin}
              </span>
            </div>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '10px 12px',
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
              }}
            >
              <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                {t('common:recovery_host_addr_label')}
              </span>
              <span
                style={{
                  fontFamily: 'monospace',
                  fontSize: 'var(--text-body-sm)',
                  color: 'var(--text-primary)',
                }}
              >
                {info.displayAddr}
              </span>
            </div>
          </div>

          {/* localhost 警告 */}
          {/^(127\.|::1|\[::1\])/.test(info.displayAddr) && (
            <p
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--warning)',
                textAlign: 'center',
                margin: 0,
              }}
            >
              {t('common:recovery_host_localhost_warning')}
            </p>
          )}

          {/* 手动输入指引 — 折叠面板 */}
          <div style={{ width: '100%' }}>
            <button
              type="button"
              onClick={onToggleManual}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                width: '100%',
                padding: '8px 10px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: manualOpen
                  ? 'color-mix(in srgb, var(--accent-primary) 6%, transparent)'
                  : 'transparent',
                color: 'var(--text-secondary)',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontSize: 'var(--text-body-sm)',
                transition: 'all 0.15s ease',
              }}
            >
              <span style={{ fontWeight: 500 }}>
                {manualOpen
                  ? t('common:recovery_host_manual_hide', {
                      defaultValue: 'Hide manual entry guide',
                    })
                  : t('common:recovery_host_manual_show', {
                      defaultValue: 'No camera? Enter details manually',
                    })}
              </span>
              {manualOpen ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
            </button>

            {manualOpen && (
              <div
                style={{
                  marginTop: 10,
                  padding: '12px 14px',
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 10,
                }}
              >
                <p
                  style={{
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-secondary)',
                    margin: 0,
                    lineHeight: 1.5,
                  }}
                >
                  {t('common:recovery_host_manual_desc', {
                    defaultValue:
                      'On the other device, open "Restore from another device", choose the Manual tab, and enter:',
                  })}
                </p>

                {/* 可复制地址 */}
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '8px 10px',
                    borderRadius: 6,
                    background: 'var(--bg-elevated)',
                    border: '1px solid var(--border-subtle)',
                  }}
                >
                  <div style={{ minWidth: 0 }}>
                    <div
                      style={{
                        fontSize: 'var(--text-caption)',
                        color: 'var(--text-tertiary)',
                        marginBottom: 2,
                      }}
                    >
                      {t('common:recovery_host_addr_label')}
                    </div>
                    <div
                      style={{
                        fontFamily: 'monospace',
                        fontSize: 'var(--text-body-sm)',
                        color: 'var(--text-primary)',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {info.displayAddr}
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={onCopyAddr}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 4,
                      padding: '4px 8px',
                      borderRadius: 4,
                      border: 'none',
                      background: copiedAddr
                        ? 'rgba(39,174,96,0.1)'
                        : 'color-mix(in srgb, var(--accent-primary) 8%, transparent)',
                      color: copiedAddr ? '#27ae60' : 'var(--accent-primary)',
                      cursor: 'pointer',
                      fontFamily: 'inherit',
                      fontSize: 'var(--text-caption)',
                      flexShrink: 0,
                      transition: 'all 0.15s ease',
                    }}
                  >
                    {copiedAddr ? <Check size={14} /> : <Copy size={14} />}
                    {copiedAddr ? t('common:copied') : t('common:copy')}
                  </button>
                </div>

                {/* 可复制 PIN */}
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '8px 10px',
                    borderRadius: 6,
                    background: 'var(--bg-elevated)',
                    border: '1px solid var(--border-subtle)',
                  }}
                >
                  <div>
                    <div
                      style={{
                        fontSize: 'var(--text-caption)',
                        color: 'var(--text-tertiary)',
                        marginBottom: 2,
                      }}
                    >
                      {t('common:recovery_host_pin_label')}
                    </div>
                    <div
                      style={{
                        fontFamily: 'monospace',
                        fontSize: 'var(--text-body-sm)',
                        fontWeight: 700,
                        letterSpacing: 4,
                        color: 'var(--accent-primary)',
                      }}
                    >
                      {info.pin}
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={onCopyPin}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 4,
                      padding: '4px 8px',
                      borderRadius: 4,
                      border: 'none',
                      background: copiedPin
                        ? 'rgba(39,174,96,0.1)'
                        : 'color-mix(in srgb, var(--accent-primary) 8%, transparent)',
                      color: copiedPin ? '#27ae60' : 'var(--accent-primary)',
                      cursor: 'pointer',
                      fontFamily: 'inherit',
                      fontSize: 'var(--text-caption)',
                      flexShrink: 0,
                      transition: 'all 0.15s ease',
                    }}
                  >
                    {copiedPin ? <Check size={14} /> : <Copy size={14} />}
                    {copiedPin ? t('common:copied') : t('common:copy')}
                  </button>
                </div>

                <p
                  style={{
                    fontSize: 'var(--text-caption)',
                    color: 'var(--text-tertiary)',
                    margin: '2px 0 0',
                    lineHeight: 1.4,
                  }}
                >
                  {t('common:recovery_host_manual_note', {
                    defaultValue:
                      'Keep this app open until the transfer completes. The session expires in 5 minutes.',
                  })}
                </p>
              </div>
            )}
          </div>

          {/* 过期时间提示 */}
          <p
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
              textAlign: 'center',
              margin: 0,
            }}
          >
            {t('common:recovery_host_expires')}
          </p>

          <Button variant="secondary" onClick={onCancel} style={{ width: '100%' }}>
            {t('common:recovery_host_cancel')}
          </Button>
        </motion.div>
      )}
    </>
  );
}
