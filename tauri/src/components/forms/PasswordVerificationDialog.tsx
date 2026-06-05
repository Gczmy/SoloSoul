import { useState } from 'react';
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
  title = 'Enter Password',
  description = 'Please enter your vault password to continue.',
  confirmLabel = 'Confirm',
  hint,
}: PasswordVerificationDialogProps) {
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const { onError } = useToastError();

  const handleConfirm = async () => {
    if (!password) {
      setError('Password is required');
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
        setError('Incorrect password');
      }
    } catch (e) {
      onError(e, 'Verification failed');
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
        <h2 style={{ fontSize: 16, fontWeight: 600, margin: 0 }}>{title}</h2>
        <p style={{ fontSize: 13, color: 'var(--text-secondary)', margin: 0 }}>
          {description}
        </p>
        <SecurePasswordInput
          value={password}
          onChange={(v) => { setPassword(v); setError(null); }}
          placeholder="Vault password"
          error={error}
          autoComplete="current-password"
          hint={hint}
        />
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, marginTop: 4 }}>
          <Button variant="secondary" onClick={handleClose}>
            Cancel
          </Button>
          <Button onClick={handleConfirm} loading={loading} disabled={!password}>
            {confirmLabel}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
