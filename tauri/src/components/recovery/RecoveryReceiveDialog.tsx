import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { Loader2, X, ScanLine } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { useAuthStore } from '@/stores/authStore';

interface RecoveryReceiveDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

interface RecoveryResultSummary {
  objectCount: number;
  attachmentCount: number;
}

export function RecoveryReceiveDialog({ isOpen, onClose }: RecoveryReceiveDialogProps) {
  const { t } = useTranslation(['common']);
  const navigate = useNavigate();
  const [accountName, setAccountName] = useState('');
  const [password, setPassword] = useState('');
  const [hostAddr, setHostAddr] = useState('');
  const [pin, setPin] = useState('');
  const [fingerprint, setFingerprint] = useState<string | undefined>(undefined);
  const [nonce, setNonce] = useState<string | undefined>(undefined);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<RecoveryResultSummary | null>(null);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSuccess(null);

    if (accountName.trim().length === 0) {
      setError(t('common:account_name_required'));
      return;
    }
    if (password.length < 8) {
      setError(t('common:password_length_requirement'));
      return;
    }
    if (hostAddr.trim().length === 0) {
      setError(t('common:invalid_addr'));
      return;
    }
    if (!/^\d{6}$/.test(pin)) {
      setError(t('common:recovery_receive_invalid_pin'));
      return;
    }

    setLoading(true);
    try {
      const result = await invoke<RecoveryResultSummary>('recovery_restore_from_host', {
        accountName,
        masterPassword: password,
        hostAddr,
        pin,
        fingerprint,
        nonce,
      });
      setSuccess(result);
      await useAuthStore.getState().checkHasAccount();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    if (success) {
      navigate('/home', { replace: true });
    }
    onClose();
  };

  const handleHostAddrChange = (value: string) => {
    setHostAddr(value);
    // 允许用户粘贴 QR payload JSON，自动解析出地址、PIN、nonce 和指纹
    try {
      const parsed = JSON.parse(value);
      if (parsed.a) {
        setHostAddr(parsed.a);
        if (parsed.p && /^\d{6}$/.test(String(parsed.p))) {
          setPin(String(parsed.p));
        }
        if (parsed.n) setNonce(String(parsed.n));
        if (parsed.f) setFingerprint(String(parsed.f));
      }
    } catch {
      // 普通地址输入：清除此前由 QR 解析带来的指纹/nonce，避免状态残留
      setFingerprint(undefined);
      setNonce(undefined);
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
          {t('common:recovery_receive_title')}
        </h2>
        <p
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            margin: '0 0 20px',
            lineHeight: 1.5,
          }}
        >
          {t('common:recovery_receive_desc')}
        </p>

        {success ? (
          <div style={{ textAlign: 'center', padding: '12px 0' }}>
            <div
              style={{
                width: 56,
                height: 56,
                borderRadius: '50%',
                background: 'rgba(39,174,96,0.12)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                margin: '0 auto 16px',
              }}
            >
              <span style={{ fontSize: 28 }}>🎉</span>
            </div>
            <h3
              style={{
                fontSize: 'var(--text-body)',
                fontWeight: 600,
                margin: '0 0 8px',
                color: 'var(--text-primary)',
              }}
            >
              {t('common:recovery_receive_success')}
            </h3>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                margin: '0 0 24px',
              }}
            >
              {t('common:recovery_receive_success_desc', {
                objects: success.objectCount,
                attachments: success.attachmentCount,
              })}
            </p>
            <Button onClick={handleClose} style={{ width: '100%' }}>
              {t('common:onboarding_done')}
            </Button>
          </div>
        ) : (
          <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <Input
              label={t('common:recovery_receive_name_label')}
              value={accountName}
              onChange={(e) => setAccountName(e.target.value)}
              autoFocus
            />
            <Input
              label={t('common:recovery_receive_password_label')}
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={t('common:recovery_receive_password_hint')}
            />
            <Input
              label={t('common:recovery_receive_addr_label')}
              value={hostAddr}
              onChange={(e) => handleHostAddrChange(e.target.value)}
              placeholder={t('common:recovery_receive_addr_placeholder')}
            />
            <Input
              label={t('common:recovery_receive_pin_label')}
              value={pin}
              onChange={(e) => setPin(e.target.value)}
              maxLength={6}
              placeholder="123456"
            />

            {fingerprint && (
              <div
                style={{
                  padding: '8px 12px',
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  fontSize: 'var(--text-body-sm)',
                  color: 'var(--text-secondary)',
                }}
              >
                <span style={{ marginRight: 8 }}>
                  <ScanLine size={14} style={{ verticalAlign: 'middle', marginRight: 4 }} />
                  {t('common:recovery_receive_fingerprint_label')}
                </span>
                <code style={{ color: 'var(--text-primary)', fontFamily: 'monospace' }}>
                  {fingerprint}
                </code>
              </div>
            )}

            {error && (
              <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)' }}>{error}</div>
            )}

            <Button type="submit" disabled={loading} style={{ width: '100%', marginTop: 8 }}>
              {loading ? (
                <>
                  <Loader2
                    size={16}
                    style={{ marginRight: 8, animation: 'spin 1s linear infinite' }}
                  />
                  {t('common:loading')}
                </>
              ) : (
                t('common:recovery_receive_start')
              )}
            </Button>
          </form>
        )}
      </Card>
    </div>
  );
}
