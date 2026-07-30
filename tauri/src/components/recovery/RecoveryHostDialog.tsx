import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { QRCodeSVG } from 'qrcode.react';
import { invoke } from '@tauri-apps/api/core';
import { Loader2, X, Copy, Check, ChevronDown, ChevronUp } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';

interface RecoveryHostDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

interface RecoveryHostInfo {
  displayAddr: string;
  bindAddr: string;
  pin: string;
  qrPayload: string;
}

export function RecoveryHostDialog({ isOpen, onClose }: RecoveryHostDialogProps) {
  const { t } = useTranslation(['common']);
  const [info, setInfo] = useState<RecoveryHostInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [manualOpen, setManualOpen] = useState(false);
  const [copiedAddr, setCopiedAddr] = useState(false);
  const [copiedPin, setCopiedPin] = useState(false);

  if (!isOpen) return null;

  const handleStart = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<RecoveryHostInfo>('recovery_host_start');
      setInfo(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleCancel = async () => {
    try {
      await invoke('recovery_host_cancel');
    } catch {
      // ignore cleanup errors
    }
    setInfo(null);
    onClose();
  };

  const handleClose = () => {
    if (info) {
      invoke('recovery_host_cancel').catch(() => {});
    }
    setInfo(null);
    onClose();
  };

  const copyToClipboard = async (text: string, setCopied: (v: boolean) => void) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // fallback for older browsers
      const textarea = document.createElement('textarea');
      textarea.value = text;
      textarea.style.position = 'fixed';
      textarea.style.opacity = '0';
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      document.body.removeChild(textarea);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-modal)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg-overlay)',
        backdropFilter: 'blur(4px)',
        padding: 16,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <Card
        style={{
          maxWidth: 420,
          width: '100%',
          padding: 24,
          position: 'relative',
        }}
      >
        <button
          type="button"
          onClick={handleClose}
          style={{
            position: 'absolute',
            top: 12,
            right: 12,
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--text-tertiary)',
          }}
          aria-label={t('common:close')}
        >
          <X size={20} />
        </button>

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

        {!info ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <Button onClick={handleStart} disabled={loading} style={{ width: '100%' }}>
              {loading ? (
                <>
                  <Loader2 size={16} style={{ marginRight: 8, animation: 'spin 1s linear infinite' }} />
                  {t('common:loading')}
                </>
              ) : (
                t('common:recovery_link_new_device')
              )}
            </Button>
            {error && (
              <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)' }}>{error}</div>
            )}
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16, alignItems: 'center' }}>
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
            <p style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)', margin: 0, textAlign: 'center' }}>
              {t('common:recovery_host_qr_hint', { defaultValue: 'Scan with the other device to connect automatically' })}
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
                onClick={() => setManualOpen(!manualOpen)}
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
                    ? t('common:recovery_host_manual_hide', { defaultValue: 'Hide manual entry guide' })
                    : t('common:recovery_host_manual_show', { defaultValue: 'No camera? Enter details manually' })}
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
                    defaultValue: 'On the other device, open "Restore from another device", choose the Manual tab, and enter:'
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
                      <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)', marginBottom: 2 }}>
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
                      onClick={() => copyToClipboard(info.displayAddr, setCopiedAddr)}
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
                      {copiedAddr
                        ? t('common:copied')
                        : t('common:copy')}
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
                      <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)', marginBottom: 2 }}>
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
                      onClick={() => copyToClipboard(info.pin, setCopiedPin)}
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
                      {copiedPin
                        ? t('common:copied')
                        : t('common:copy')}
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
                    defaultValue: 'Keep this app open until the transfer completes. The session expires in 5 minutes.'
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

            <Button variant="secondary" onClick={handleCancel} style={{ width: '100%' }}>
              {t('common:recovery_host_cancel')}
            </Button>
          </div>
        )}
      </Card>
    </div>
  );
}
