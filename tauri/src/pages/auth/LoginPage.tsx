import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import type { AccountInfo } from '@/lib/ipc';
import { getBiometricErrorMessage } from '@/lib/biometricError';
import { useCancellable } from '@/hooks/useCancellable';

import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { Fingerprint, Loader2 } from 'lucide-react';
import styles from './LoginPage.module.css';

export function LoginPage() {
  const navigate = useNavigate();
  const {
    login,
    checkHasAccount,
    listAccounts,
    hasAccount,
    isAuthenticated,
    isLoading,
    error,
    accounts,
    clearError,
  } = useAuthStore();
  const [selectedAccountId, setSelectedAccountId] = useState('');
  const [password, setPassword] = useState('');
  const { t } = useTranslation(['auth', 'common', 'settings']);
  const selectedAccount = accounts.find((a) => a.id === selectedAccountId);

  // Biometric state
  const [bioAvailable, setBioAvailable] = useState(false);
  const [biometryType, setBiometryType] = useState('Touch ID');
  const [biometryTypeRaw, setBiometryTypeRaw] = useState('touchId');
  const [bioLoading, setBioLoading] = useState(false);
  const [bioError, setBioError] = useState<string | null>(null);
  const [showPasswordInput, setShowPasswordInput] = useState(false);
  const [bioChecked, setBioChecked] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const makeCancellable = useCancellable();

  useEffect(() => {
    // Defensive load: fetch the account list directly in case Vite HMR keeps
    // authStore.listAccounts pointing at a stale command name.
    invoke<AccountInfo[]>('vault_list_accounts')
      .then((parsed) => {
        useAuthStore.setState({
          accounts: parsed,
          hasAccount: parsed.length > 0,
        });
      })
      .catch(() => {
        // Fall through to the store-level loader below.
      });
    const { isCancelled, cancel } = makeCancellable();
    checkHasAccount().then(() => {
      if (!isCancelled()) listAccounts();
    });
    // Probe device biometry type early (do not set bioAvailable here — configured
    // status is account-specific and decided in the selectedAccountId effect).
    invoke<{ available: boolean; biometryType?: string }>('biometric_check_availability', {
      accountId: '',
    })
      .then((r) => {
        if (isCancelled()) return;
        if (r.biometryType === 'touchId') {
          setBiometryType('Touch ID');
          setBiometryTypeRaw('touchId');
        } else if (r.biometryType === 'faceId') {
          setBiometryType('Face ID');
          setBiometryTypeRaw('faceId');
        }
      })
      .catch(() => {
        // Ignore: account-specific check will handle availability.
      });
    return cancel;
  }, [checkHasAccount, listAccounts, makeCancellable]);

  useEffect(() => {
    if (hasAccount === false) navigate('/bootstrap');
    if (isAuthenticated) navigate('/');
  }, [hasAccount, isAuthenticated, navigate]);

  // Auto-select first account
  useEffect(() => {
    if (accounts.length > 0 && !selectedAccountId) {
      setSelectedAccountId(accounts[0].id);
    }
  }, [accounts, selectedAccountId]);

  // Check biometric availability for selected account (only if account exists)
  useEffect(() => {
    const { isCancelled, cancel } = makeCancellable();
    if (!selectedAccountId) {
      setBioChecked(true);
      return cancel;
    }
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>(
      'biometric_check_availability',
      { accountId: selectedAccountId },
    )
      .then((r) => {
        if (isCancelled()) return;
        if (r.available && r.configured) {
          setBioAvailable(true);
          if (r.biometryType === 'touchId') {
            setBiometryType('Touch ID');
            setBiometryTypeRaw('touchId');
          } else if (r.biometryType === 'faceId') {
            setBiometryType('Face ID');
            setBiometryTypeRaw('faceId');
          }
        } else {
          setBioAvailable(false);
          setShowPasswordInput(true);
        }
      })
      .catch(() => {
        if (isCancelled()) return;
        setBioAvailable(false);
        setShowPasswordInput(true);
      })
      .finally(() => {
        if (!isCancelled()) setBioChecked(true);
      });
    return cancel;
  }, [selectedAccountId, makeCancellable]);

  const handleBiometricUnlock = useCallback(async () => {
    if (!selectedAccountId || bioLoading) return;
    setBioLoading(true);
    setBioError(null);
    let success = false;
    try {
      await invoke('biometric_unlock', {
        accountId: selectedAccountId,
        location: 'login_page',
        action: 'unlock',
        biometryType: biometryTypeRaw,
      });
      // Vault already unlocked — set auth state directly
      const accs = (await invoke<AccountInfo[]>('vault_list_accounts')) || [];
      const acc = accs.find((a) => a.id === selectedAccountId) || {
        id: selectedAccountId,
        name: selectedAccountId,
      };
      useAuthStore.setState({ isAuthenticated: true, currentAccount: acc, accounts: accs });
      success = true;
      // Navigate immediately to avoid showing the biometric UI after success
      navigate('/');
    } catch (e) {
      const msg = String(e);
      if (
        msg.toLowerCase().includes('cancelled') ||
        msg.toLowerCase().includes('cancel') ||
        msg.includes('__BIO_ERR__:cancelled')
      ) {
        setShowPasswordInput(true);
      } else {
        setBioError(getBiometricErrorMessage(e, t));
        setShowPasswordInput(true);
      }
    } finally {
      if (!success) setBioLoading(false);
    }
  }, [selectedAccountId, bioLoading, t, navigate, biometryTypeRaw]);

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    clearError();
    setBioError(null);
    setSubmitError(null);
    if (!selectedAccountId) {
      setSubmitError(t('auth:no_account_selected'));
      return;
    }
    await login(selectedAccountId, password);
  };

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: '100vh',
      }}
    >
      <div
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          padding: 32,
          width: 360,
          boxShadow: '0 8px 32px rgba(0,0,0,0.08)',
          textAlign: 'center',
        }}
      >
        <ShieldLogo size={48} style={{ margin: '0 auto 16px' }} />
        <h1 style={{ fontSize: 'var(--text-page-title)', fontWeight: 600, marginBottom: 4 }}>{t('auth:login_title')}</h1>
        <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', marginBottom: 4 }}>
          {t('auth:login_subtitle')}
        </p>

        {/* Loading state while bio check runs — same layout, no layout jump */}
        {!bioChecked && (
          <div style={{ display: 'flex', justifyContent: 'center', padding: '24px 0' }}>
            <Loader2 size={24} className={styles.loadingSpinner} style={{ color: 'var(--text-tertiary)' }} />
          </div>
        )}

        {/* Account selector / name — visible for both biometric and password login */}
        {bioChecked && accounts.length > 0 && (
          <div style={{ marginBottom: 20, width: '100%' }}>
            {accounts.length > 1 ? (
              <select
                value={selectedAccountId}
                onChange={(e) => setSelectedAccountId(e.target.value)}
                style={{
                  width: '100%',
                  padding: '10px 14px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  color: 'var(--text-primary)',
                  fontSize: 'var(--text-body)',
                  fontFamily: 'inherit',
                  outline: 'none',
                  textAlign: 'left',
                }}
              >
                {accounts.map((acc) => (
                  <option key={acc.id} value={acc.id}>
                    {acc.name} · {acc.id}
                  </option>
                ))}
              </select>
            ) : (
              <div
                style={{
                  width: '100%',
                  padding: '10px 14px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  color: 'var(--text-primary)',
                  fontSize: 'var(--text-body)',
                  textAlign: 'left',
                }}
              >
                <div>{selectedAccount?.name ?? accounts[0]?.name}</div>
                <div
                  style={{
                    fontSize: 'var(--text-badge)',
                    color: 'var(--text-tertiary)',
                    marginTop: 2,
                    fontFamily: 'monospace',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {selectedAccount?.id ?? accounts[0]?.id}
                </div>
              </div>
            )}
          </div>
        )}

        {/* Biometric unlock */}
        {bioAvailable && !showPasswordInput && (
          <div style={{ marginBottom: 16 }}>
            <button
              onClick={handleBiometricUnlock}
              disabled={bioLoading}
              className={styles.loginFloatButton}
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: 12,
                padding: '20px 24px',
                borderRadius: 14,
                border: '1px solid var(--border-subtle)',
                background: bioLoading ? 'var(--bg-toolbar)' : 'transparent',
                cursor: bioLoading ? 'wait' : 'pointer',
                width: '100%',
              }}
            >
              <Fingerprint
                size={40}
                color="var(--accent-primary)"
                style={{ opacity: bioLoading ? 0.5 : 1 }}
              />
              <span style={{ fontSize: 'var(--text-card-title)', fontWeight: 500, color: 'var(--text-primary)' }}>
                {bioLoading
                  ? t('auth:bio_verifying')
                  : t('auth:bio_unlock_reason', { type: biometryType })}
              </span>
            </button>
            <button
              onClick={() => setShowPasswordInput(true)}
              className={styles.loginTextButton}
              style={{
                marginTop: 12,
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-tertiary)',
                background: 'none',
                border: 'none',
                cursor: 'pointer',
              }}
            >
              {t('auth:use_password_instead')}
            </button>
          </div>
        )}

        {/* Password input — always shown when bio not available or user chose password */}
        {bioChecked && (showPasswordInput || !bioAvailable) && (
          <form
            onSubmit={handleSubmit}
            style={{ display: 'flex', flexDirection: 'column', gap: 16 }}
          >
            <SecurePasswordInput
              value={password}
              onChange={(v) => setPassword(v)}
              placeholder={t('common:password_placeholder')}
              hint={selectedAccount?.passwordHint || null}
              autoComplete="current-password"
              onEnter={handleSubmit}
            />
            {(error || bioError || submitError) && (
              <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)' }}>
                {submitError || bioError || (error
                  ? error.toLowerCase().includes('8 characters') ||
                    error.toLowerCase().includes('至少')
                    ? t('auth:password_too_short')
                    : error.toLowerCase().includes('password') ||
                      error.toLowerCase().includes('invalid')
                      ? t('auth:incorrect_password')
                      : error.toLowerCase().includes('required')
                        ? t('auth:password_required')
                        : error
                  : '')}
              </div>
            )}
            <button
              type="submit"
              disabled={isLoading}
              style={{
                width: '100%',
                padding: '8px 16px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: isLoading ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'var(--bg-toolbar)',
                color: isLoading ? 'var(--accent-primary)' : 'var(--text-primary)',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                fontFamily: 'inherit',
                cursor: isLoading ? 'default' : 'pointer',
                opacity: isLoading ? 0.6 : 1,
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                if (!isLoading) {
                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }
              }}
              onMouseLeave={(e) => {
                if (!isLoading) {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.color = 'var(--text-primary)';
                }
              }}
            >
              {isLoading ? t('common:loading', { defaultValue: '...' }) : t('auth:login_button')}
            </button>
            {bioAvailable && (
              <button
                onClick={() => {
                  setShowPasswordInput(false);
                  setBioError(null);
                }}
                className={styles.loginTextButton}
                style={{
                  fontSize: 'var(--text-body-sm)',
                  color: 'var(--text-tertiary)',
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                }}
              >
                {t('auth:use_biometric_instead', { type: biometryType })}
              </button>
            )}
          </form>
        )}
      </div>
    </div>
  );
}
