/**
 * P021：恢复主机手动输入指引折叠面板（自 RecoveryQrContent 拆出的静态展示块）。
 */
import { Copy, Check, ChevronDown, ChevronUp } from 'lucide-react';
import type { useTranslation } from 'react-i18next';

type T = ReturnType<typeof useTranslation>['t'];

interface RecoveryManualEntryPanelProps {
  t: T;
  open: boolean;
  onToggle: () => void;
  copiedAddr: boolean;
  copiedPin: boolean;
  onCopyAddr: () => void;
  onCopyPin: () => void;
  displayAddr: string;
  pin: string;
}

export function RecoveryManualEntryPanel({
  t,
  open,
  onToggle,
  copiedAddr,
  copiedPin,
  onCopyAddr,
  onCopyPin,
  displayAddr,
  pin,
}: RecoveryManualEntryPanelProps) {
  return (
          <div style={{ width: '100%' }}>
            <button
              type="button"
              onClick={onToggle}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                width: '100%',
                padding: '8px 10px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: open
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
                {open
                  ? t('common:recovery_host_manual_hide', {
                      defaultValue: 'Hide manual entry guide',
                    })
                  : t('common:recovery_host_manual_show', {
                      defaultValue: 'No camera? Enter details manually',
                    })}
              </span>
              {open ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
            </button>

            {open && (
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
                      {displayAddr}
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
                      {pin}
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


  );
}
