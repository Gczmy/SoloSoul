import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { QRCodeSVG } from 'qrcode.react';
import { invoke } from '@tauri-apps/api/core';
import { X, Loader2 } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';

interface SyncShowQrDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

interface SyncQrInfo {
  payload: string;
  addr: string;
  fingerprint: string;
  deviceName: string;
}

export function SyncShowQrDialog({ isOpen, onClose }: SyncShowQrDialogProps) {
  const { t } = useTranslation(['common']);
  const [info, setInfo] = useState<SyncQrInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) {
      setInfo(null);
      setError(null);
      return;
    }

    setLoading(true);
    invoke<string>('sync_generate_qr_payload')
      .then((payload) => {
        try {
          const parsed = JSON.parse(payload);
          setInfo({
            payload,
            addr: parsed.a || '',
            fingerprint: parsed.f || '',
            deviceName: parsed.n || '',
          });
        } catch {
          setError(t('common:sync_qr_invalid_payload'));
        }
      })
      .catch((err) => {
        setError(String(err));
      })
      .finally(() => {
        setLoading(false);
      });
  }, [isOpen, t]);

  if (!isOpen) return null;

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
        if (e.target === e.currentTarget) onClose();
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
          onClick={onClose}
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
          {t('common:sync_qr_show_title')}
        </h2>
        <p
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            margin: '0 0 20px',
            lineHeight: 1.5,
          }}
        >
          {t('common:sync_qr_show_desc')}
        </p>

        {loading ? (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 12,
              padding: '32px 0',
            }}
          >
            <Loader2 size={32} style={{ animation: 'spin 1s linear infinite' }} />
            <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
              {t('common:loading')}
            </span>
          </div>
        ) : error ? (
          <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)', padding: '12px 0' }}>
            {error}
          </div>
        ) : info ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16, alignItems: 'center' }}>
            <div
              style={{
                padding: 12,
                background: '#fff',
                borderRadius: 12,
                border: '1px solid var(--border-subtle)',
              }}
            >
              <QRCodeSVG value={info.payload} size={200} level="M" includeMargin />
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
                  {t('common:sync_qr_device_name')}
                </span>
                <span
                  style={{
                    fontSize: 'var(--text-body-sm)',
                    fontWeight: 500,
                    color: 'var(--text-primary)',
                  }}
                >
                  {info.deviceName}
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
                  marginBottom: 8,
                }}
              >
                <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
                  {t('common:sync_qr_addr')}
                </span>
                <span
                  style={{
                    fontFamily: 'monospace',
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-primary)',
                  }}
                >
                  {info.addr}
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
                  {t('common:sync_qr_fingerprint')}
                </span>
                <span
                  style={{
                    fontFamily: 'monospace',
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-primary)',
                    wordBreak: 'break-all',
                    maxWidth: '60%',
                    textAlign: 'right',
                  }}
                >
                  {info.fingerprint}
                </span>
              </div>
            </div>

            {info.addr.startsWith('127.') && (
              <p
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--warning)',
                  textAlign: 'center',
                  margin: 0,
                }}
              >
                {t('common:sync_qr_localhost_warning')}
              </p>
            )}

            <Button variant="secondary" onClick={onClose} style={{ width: '100%' }}>
              {t('common:close')}
            </Button>
          </div>
        ) : null}
      </Card>
    </div>
  );
}
