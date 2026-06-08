import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Dialog } from '@/components/ui/Dialog';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { Button } from '@/components/ui/Button';
import { useToastError } from '@/hooks/useToastError';
import { Fingerprint } from 'lucide-react';

interface PasswordVerificationDialogProps {
  open: boolean;
  onClose: () => void;
  /** Called with the password. Return true to confirm, false to reject */
  onVerify: (password: string) => Promise<boolean>;
  /** Customizable text overrides — all i18n-able via parent */
  title?: string;
  description?: string;
  confirmLabel?: string;
  /** Optional password hint to display */
  hint?: string | null;
  /** Biometric type name (e.g. "Touch ID", "Face ID") — enables biometric button */
  biometricType?: string;
  /** Called when user clicks biometric button. Return true on success */
  onBiometric?: () => Promise<boolean>;
}

/**
 * Unified password verification dialog — single source of truth for all
 * password-gated operations across the app.
 *
 * Supports:
 * - Customizable title/description/confirm text (i18n-ready)
 * - Password show/hide toggle + hint tooltip (via SecurePasswordInput)
 * - Biometric unlock fallback (Touch ID / Face ID / Windows Hello)
 */
export function PasswordVerificationDialog({
  open,
  onClose,
  onVerify,
  title,
  description,
  confirmLabel,
  hint,
  biometricType,
  onBiometric,
}: PasswordVerificationDialogProps) {
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [bioLoading, setBioLoading] = useState(false);
  const { onError } = useToastError();
  const { t } = useTranslation(['auth', 'common', 'settings']);

  const handleConfirm = async () => {
    if (!password) {
      setError(t('auth:password_required'));
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const ok = await onVerify(password);
      if (ok) {
        setPassword('');
        onClose();
      } else {
        setError(t('auth:incorrect_password'));
      }
    } catch (e) {
      onError(e, t('common:error'));
    } finally {
      setLoading(false);
    }
  };

  const handleBiometric = async () => {
    if (!onBiometric) return;
    setBioLoading(true);
    setError(null);
    try {
      const ok = await onBiometric();
      if (ok) {
        setPassword('');
        onClose();
      }
      // User cancelled — silently close, no error
    } catch {
      // User cancelled or failed — silently close
    } finally {
      setBioLoading(false);
    }
  };

  const handleClose = () => {
    setPassword('');
    setError(null);
    onClose();
  };

  return (
    <Dialog isOpen={open} onClose={handleClose}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16, minWidth: 320 }}>
        <h2 style={{ fontSize: 16, fontWeight: 600, margin: 0 }}>
          {title || t('auth:verification_title')}
        </h2>
        {description && (
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', margin: 0 }}>
            {description}
          </p>
        )}
        <SecurePasswordInput
          value={password}
          onChange={(v) => { setPassword(v); setError(null); }}
          placeholder={t('common:password_placeholder')}
          error={error}
          autoComplete="current-password"
          hint={hint}
        />
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 4 }}>
          {/* Biometric button */}
          {biometricType && onBiometric ? (
            <button
              type="button"
              onClick={handleBiometric}
              disabled={bioLoading}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: 6,
                padding: '8px 14px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'transparent',
                color: 'var(--text-secondary)',
                fontSize: 13,
                cursor: 'pointer',
                fontFamily: 'inherit',
                transition: 'all 0.15s',
              }}
              title={t('settings:biometric_test_button', { type: biometricType })}
            >
              <Fingerprint size={16} />
              {bioLoading ? '…' : biometricType}
            </button>
          ) : (
            <span />
          )}
          <div style={{ display: 'flex', gap: 8 }}>
            <Button variant="secondary" onClick={handleClose}>
              {t('common:cancel')}
            </Button>
            <Button onClick={handleConfirm} loading={loading} disabled={!password}>
              {confirmLabel || t('common:confirm')}
            </Button>
          </div>
        </div>
      </div>
    </Dialog>
  );
}
