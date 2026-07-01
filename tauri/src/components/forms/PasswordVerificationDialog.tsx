import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { Dialog } from '@/components/ui/Dialog';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { PinInput } from '@/components/forms/PinInput';
import { Button } from '@/components/ui/Button';
import { useToastError } from '@/hooks/useToastError';
import { Fingerprint, KeyRound } from 'lucide-react';
import { ICON_SIZE } from '@/lib/iconSizes';


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
  /** If provided, enables PIN verification mode */
  pinAccountId?: string;
  /** Called when PIN unlock succeeds (instead of onClose, which always reports ok=false) */
  onPinSuccess?: () => void;
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
  pinAccountId,
  onPinSuccess,
}: PasswordVerificationDialogProps) {
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [bioLoading, setBioLoading] = useState(false);
  const [pinMode, setPinMode] = useState(false);
  const [pinAvailable, setPinAvailable] = useState(false);
  const [pinUnlocking, setPinUnlocking] = useState(false);
  const [pinError, setPinError] = useState<string | null>(null);
  const [pinInputKey, setPinInputKey] = useState(0);
  const { onError } = useToastError();
  const { t } = useTranslation(['auth', 'common', 'settings']);

  // Check PIN availability when dialog opens
  useEffect(() => {
    if (open && pinAccountId) {
      invoke<{ configured: boolean; locked: boolean }>('pin_check_availability', {
        accountId: pinAccountId,
      })
        .then((r) => setPinAvailable(r.configured && !r.locked))
        .catch(() => setPinAvailable(false));
    }
  }, [open, pinAccountId]);

  const handlePinComplete = useCallback(async (pin: string) => {
    if (!pinAccountId) return;
    setPinUnlocking(true);
    setPinError(null);
    try {
      await invoke('pin_unlock', { accountId: pinAccountId, pin });
      setPassword('');
      setPinMode(false);
      setPinError(null);
      onPinSuccess?.();
    } catch (e) {
      const msg = String(e);
      if (msg.includes('__PIN_ERR__:locked')) {
        setPinError(t('auth:pin_locked'));
        setPinAvailable(false);
        setPinMode(false);
      } else if (msg.includes('__PIN_ERR__:incorrect')) {
        setPinError(t('auth:pin_incorrect'));
      } else {
        setPinError(t('auth:pin_error'));
      }
      setPinInputKey((k) => k + 1);
    } finally {
      setPinUnlocking(false);
    }
  }, [pinAccountId, t, onPinSuccess]);

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
    } catch {
      // User cancelled or failed — silently close
    } finally {
      setBioLoading(false);
    }
  };

  const handleClose = () => {
    setPassword('');
    setError(null);
    setPinMode(false);
    setPinError(null);
    onClose();
  };

  return (
    <Dialog isOpen={open} onClose={handleClose}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16, minWidth: 320 }}>
        <h2 style={{ fontSize: 'var(--text-section-title)', fontWeight: 600, margin: 0 }}>
          {title || t('auth:verification_title')}
        </h2>
        {description && (
          <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', margin: 0 }}>{description}</p>
        )}

        {pinMode ? (
          <>
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: 12,
                padding: '16px 8px',
              }}
            >
              <KeyRound size={ICON_SIZE['2xl']} color="var(--accent-primary)" />
              <span style={{ fontSize: 'var(--text-card-title)', fontWeight: 500, color: 'var(--text-primary)' }}>
                {t('auth:pin_enter_title')}
              </span>
              <PinInput
                key={pinInputKey}
                length={6}
                onComplete={handlePinComplete}
                disabled={pinUnlocking}
                error={!!pinError}
                verifying={pinUnlocking}
              />
              {pinError && (
                <div style={{ color: '#dc2626', fontSize: 'var(--text-body-sm)' }}>{pinError}</div>
              )}
            </div>
            <button
              onClick={() => { setPinMode(false); setPinError(null); }}
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-tertiary)',
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                padding: 0,
                fontFamily: 'inherit',
              }}
            >
              {t('auth:use_password_instead')}
            </button>
          </>
        ) : (
          <>
            <SecurePasswordInput
              value={password}
              onChange={(v) => {
                setPassword(v);
                setError(null);
              }}
              placeholder={t('common:password_placeholder')}
              error={error}
              autoComplete="current-password"
              hint={hint}
              onEnter={handleConfirm}
            />
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginTop: 4,
              }}
            >
              <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
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
                      fontSize: 'var(--text-body-sm)',
                      cursor: 'pointer',
                      fontFamily: 'inherit',
                      transition: 'all 0.15s',
                    }}
                    title={t('settings:biometric_test_button', { type: biometricType })}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                      e.currentTarget.style.color = 'var(--accent-primary)';
                      e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.background = 'transparent';
                      e.currentTarget.style.color = 'var(--text-secondary)';
                      e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    }}
                  >
                    <Fingerprint size={ICON_SIZE.md} />
                    {bioLoading ? '…' : biometricType}
                  </button>
                ) : null}
                {/* PIN button */}
                {pinAvailable && (
                  <button
                    type="button"
                    onClick={() => setPinMode(true)}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 6,
                      padding: '8px 14px',
                      borderRadius: 8,
                      border: '1px solid var(--border-subtle)',
                      background: 'transparent',
                      color: 'var(--text-secondary)',
                      fontSize: 'var(--text-body-sm)',
                      cursor: 'pointer',
                      fontFamily: 'inherit',
                      transition: 'all 0.15s',
                    }}
                    title={t('auth:use_pin_instead')}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                      e.currentTarget.style.color = 'var(--accent-primary)';
                      e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.background = 'transparent';
                      e.currentTarget.style.color = 'var(--text-secondary)';
                      e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    }}
                  >
                    <KeyRound size={ICON_SIZE.md} />
                    {'PIN'}
                  </button>
                )}
              </div>
              <div style={{ display: 'flex', gap: 8 }}>
                <Button variant="secondary" onClick={handleClose}>
                  {t('common:cancel')}
                </Button>
                <Button onClick={handleConfirm} loading={loading} disabled={!password}>
                  {confirmLabel || t('common:confirm')}
                </Button>
              </div>
            </div>
          </>
        )}
      </div>
    </Dialog>
  );
}
