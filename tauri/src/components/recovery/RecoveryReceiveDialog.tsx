import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { Loader2, X, ScanLine, QrCode } from 'lucide-react';
import { QRCodeSVG } from 'qrcode.react';
import { useNavigate } from 'react-router-dom';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { useAuthStore } from '@/stores/authStore';


interface RecoveryReceiveDialogProps {
  isOpen: boolean;
  onClose: () => void;
  /** 恢复成功后调用；若提供则替代默认的 /home 导航 */
  onSuccess?: () => void;
}

interface RecoveryResultSummary {
  objectCount: number;
  attachmentCount: number;
}

interface ReverseListenInfo {
  displayAddr: string;
  pin: string;
  qrPayload: string;
}

export function RecoveryReceiveDialog({ isOpen, onClose, onSuccess }: RecoveryReceiveDialogProps) {
  const { t } = useTranslation(['common']);
  const navigate = useNavigate();
  const [password, setPassword] = useState('');
  const [hostAddr, setHostAddr] = useState('');
  const [pin, setPin] = useState('');
  const [fingerprint, setFingerprint] = useState<string | undefined>(undefined);
  const [nonce, setNonce] = useState<string | undefined>(undefined);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<RecoveryResultSummary | null>(null);
  const [reverseInfo, setReverseInfo] = useState<ReverseListenInfo | null>(null);
  const [reverseSession, setReverseSession] = useState<{
    info: ReverseListenInfo;
    password: string;
  } | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // 二维码显示后立即在后台等待传输；避免把长轮询阻塞在按钮事件里导致 UI 不更新
  useEffect(() => {
    if (!reverseSession) return;
    let active = true;
    const wait = async () => {
      try {
        const result = await invoke<RecoveryResultSummary>('recovery_receive_listen_wait', {
          masterPassword: reverseSession.password,
        });
        if (!active || !mountedRef.current) return;
        setSuccess(result);
        setReverseSession(null);
        await useAuthStore.getState().checkHasAccount();
      } catch (err) {
        if (!active || !mountedRef.current) return;
        const msg = String(err);
        if (msg.includes('Recovery session cancelled')) return;
        setError(msg);
        setReverseInfo(null);
        setReverseSession(null);
      }
    };
    wait();
    return () => {
      active = false;
      invoke('recovery_host_cancel').catch(() => {});
    };
  }, [reverseSession]);

  if (!isOpen) return null;

  const validateForm = (forReverse: boolean) => {
    if (password.length < 8) {
      setError(t('common:password_length_requirement'));
      return false;
    }
    if (!forReverse) {
      if (hostAddr.trim().length === 0) {
        setError(t('common:invalid_addr'));
        return false;
      }
      if (!/^\d{6}$/.test(pin)) {
        setError(t('common:recovery_receive_invalid_pin'));
        return false;
      }
    }
    return true;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSuccess(null);

    if (!validateForm(false)) return;

    setLoading(true);
    try {
      const result = await invoke<RecoveryResultSummary>('recovery_restore_from_host', {
        masterPassword: password,
        hostAddr: hostAddr,
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
    if (reverseInfo) {
      invoke('recovery_host_cancel').catch(() => {});
    }
    if (success) {
      if (onSuccess) {
        onSuccess();
      } else {
        navigate('/home', { replace: true });
      }
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

  const handleStartReverseListen = async () => {
    setError(null);
    setSuccess(null);

    if (!validateForm(true)) return;

    setLoading(true);
    try {
      const info = await invoke<ReverseListenInfo>('recovery_receive_listen_start');
      if (!mountedRef.current) return;
      setReverseInfo(info);
      setReverseSession({ info, password });
    } catch (err) {
      if (!mountedRef.current) return;
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleCancelReverse = () => {
    invoke('recovery_host_cancel').catch(() => {});
    setReverseInfo(null);
    setReverseSession(null);
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
        ) : reverseInfo ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16, alignItems: 'center' }}>
            <div
              style={{
                padding: 12,
                background: '#fff',
                borderRadius: 12,
                border: '1px solid var(--border-subtle)',
              }}
            >
              <QRCodeSVG value={reverseInfo.qrPayload} size={200} level="M" includeMargin />
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
                  {reverseInfo.pin}
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
                  {reverseInfo.displayAddr}
                </span>
              </div>
            </div>

            <Button variant="secondary" onClick={handleCancelReverse} style={{ width: '100%' }}>
              {t('common:recovery_host_cancel')}
            </Button>
          </div>
        ) : (
          <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <Input
              label={t('common:recovery_receive_password_label')}
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={t('common:recovery_receive_password_hint')}
              autoFocus
            />
            <Input
              label={t('common:recovery_receive_addr_label')}
              value={hostAddr}
              onChange={(e) => handleHostAddrChange(e.target.value)}
              placeholder={t('common:recovery_receive_addr_placeholder')}
            />

            <button
              type="button"
              onClick={() => {
                setError(null);
                void handleStartReverseListen();
              }}
              disabled={loading}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 8,
                padding: '10px 12px',
                borderRadius: 8,
                border: '1px dashed var(--border-subtle)',
                background: 'transparent',
                color: loading ? 'var(--text-tertiary)' : 'var(--text-secondary)',
                cursor: loading ? 'not-allowed' : 'pointer',
                fontFamily: 'inherit',
                fontSize: 'var(--text-body-sm)',
                transition: 'all 0.15s ease',
                opacity: loading ? 0.6 : 1,
              }}
              onMouseEnter={(e) => {
                if (loading) return;
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                if (loading) return;
                e.currentTarget.style.borderColor = 'var(--border-subtle)';
                e.currentTarget.style.color = 'var(--text-secondary)';
              }}
            >
              <QrCode size={18} />
              {t('common:recovery_qr_show_button')}
            </button>
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
