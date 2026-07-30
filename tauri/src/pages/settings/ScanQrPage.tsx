import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { RecoveryQrScanner } from '@/components/recovery/RecoveryQrScanner';

export function ScanQrPage() {
  const { t } = useTranslation(['common']);
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    let mounted = true;
    invoke<void>('recovery_host_start')
      .then(() => {
        if (mounted) {
          setIsReady(true);
        }
      })
      .catch((err) => {
        if (mounted) {
          setError(String(err));
        }
      });

    return () => {
      mounted = false;
      invoke('recovery_host_cancel').catch(() => {});
    };
  }, []);

  const handleScanResult = async (text: string) => {
    if (loading || !isReady) return;

    setLoading(true);
    setError(null);
    setSuccess(false);

    try {
      const parsed = JSON.parse(text);
      if (parsed.t !== 'rev' || !parsed.a || !parsed.p) {
        setError(t('common:recovery_qr_invalid_reverse'));
        setLoading(false);
        return;
      }

      await invoke<void>('recovery_host_push', {
        hostAddr: parsed.a,
        pin: String(parsed.p),
        fingerprint: parsed.f,
        nonce: parsed.n,
      });

      setSuccess(true);
      setTimeout(() => {
        navigate('/settings');
      }, 1500);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <AppShell title={t('common:scan_qr_title')} onBack={() => navigate('/settings')}>
      <PageContainer variant="form" gap="default">
        <Card>
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              margin: '0 0 16px',
              lineHeight: 1.5,
            }}
          >
            {t('common:scan_qr_desc')}
          </p>

          {!isReady ? (
            <div
              style={{
                padding: 24,
                textAlign: 'center',
                color: 'var(--text-secondary)',
                fontSize: 'var(--text-body-sm)',
              }}
            >
              {t('common:loading')}
            </div>
          ) : (
            <RecoveryQrScanner
              onScan={handleScanResult}
              onError={(message) => setError(message)}
              onCancel={() => navigate('/settings')}
            />
          )}

          {loading && (
            <p
              style={{
                marginTop: 12,
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                textAlign: 'center',
              }}
            >
              {t('common:loading')}
            </p>
          )}

          {success && (
            <div
              style={{
                marginTop: 12,
                padding: 12,
                borderRadius: 8,
                background: 'rgba(39, 174, 96, 0.12)',
                color: '#27ae60',
                fontSize: 'var(--text-body-sm)',
                textAlign: 'center',
              }}
            >
              {t('common:scan_qr_success')}
            </div>
          )}

          {error && (
            <div
              style={{
                marginTop: 12,
                padding: 12,
                borderRadius: 8,
                background: 'rgba(231, 76, 60, 0.12)',
                color: '#e74c3c',
                fontSize: 'var(--text-body-sm)',
                textAlign: 'center',
              }}
            >
              {error}
            </div>
          )}

          <Button
            variant="secondary"
            onClick={() => navigate('/settings')}
            style={{ width: '100%', marginTop: 16 }}
          >
            {t('common:back')}
          </Button>
        </Card>
      </PageContainer>
    </AppShell>
  );
}
