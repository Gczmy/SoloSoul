import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Dialog } from '@/components/ui/Dialog';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { Button } from '@/components/ui/Button';
import { useToastError } from '@/hooks/useToastError';

interface PasswordVerificationDialogProps {
  open: boolean;
  onClose: () => void;
  /** Called with the password. Resolve with true to confirm, false to reject */
  onVerify: (password: string) => Promise<boolean>;
  title?: string;
  description?: string;
  confirmLabel?: string;
  /** Optional password hint to display */
  hint?: string | null;
}

/**
 * Shared password verification dialog.
 * Use this everywhere a password confirmation is needed
 * (sensitivity change, data export, account deletion, etc.)
 */
export function PasswordVerificationDialog({
  open,
  onClose,
  onVerify,
  title,
  description,
  confirmLabel,
  hint,
}: PasswordVerificationDialogProps) {
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const { onError } = useToastError();
  const { t } = useTranslation(['auth', 'common']);

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
        <p style={{ fontSize: 13, color: 'var(--text-secondary)', margin: 0 }}>
          {description || t('auth:verification_description')}
        </p>
        <SecurePasswordInput
          value={password}
          onChange={(v) => { setPassword(v); setError(null); }}
          placeholder={t('common:password_placeholder')}
          error={error}
          autoComplete="current-password"
          hint={hint}
        />
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, marginTop: 4 }}>
          <Button variant="secondary" onClick={handleClose}>
            {t('common:cancel')}
          </Button>
          <Button onClick={handleConfirm} loading={loading} disabled={!password}>
            {confirmLabel || t('common:confirm')}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
