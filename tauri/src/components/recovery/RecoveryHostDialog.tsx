import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { QRCodeSVG } from 'qrcode.react';
import { invoke } from '@tauri-apps/api/core';
import { Loader2, X } from 'lucide-react';
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
    } catch (err) {
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
