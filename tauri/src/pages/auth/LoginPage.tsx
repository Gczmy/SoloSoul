import { useState, useEffect, useCallback, type CSSProperties } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useCancellable } from '@/hooks/useCancellable';
import { Button } from '@/components/ui/Button';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { Fingerprint } from 'lucide-react';
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
  const { t } = useTranslation(['auth', 'common']);
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
    console.log('[LoginPage] mount effect');
    const { isCancelled, cancel } = makeCancellable();
    checkHasAccount().then(() => {
      console.log('[LoginPage] checkHasAccount done, calling listAccounts');
      if (!isCancelled()) listAccounts();
    });
    // Check biometric availability
    invoke<{ available: boolean; biometryType?: string }>('biometric_check_availability', {
      accountId: '',
    })
      .then((r) => {
        if (isCancelled()) return;
        if (r.available) {
          setBioAvailable(true);
          if (r.biometryType === 'touchId') {
            setBiometryType('Touch ID');
            setBiometryTypeRaw('touchId');
          } else if (r.biometryType === 'faceId') {
            setBiometryType('Face ID');
            setBiometryTypeRaw('faceId');
          }
        }
      })
      .catch(() => {
        if (!isCancelled()) setBioAvailable(false);
      });
    return cancel;
  }, []);

  useEffect(() => {
    if (hasAccount === false) navigate('/bootstrap');
    if (isAuthenticated) navigate('/');
  }, [hasAccount, isAuthenticated, navigate]);

  // Auto-select first account
  useEffect(() => {
    console.log('[LoginPage] accounts changed:', accounts.length, accounts.map((a) => a.id));
    if (accounts.length > 0 && !selectedAccountId) {
      console.log('[LoginPage] auto-selecting', accounts[0].id);
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
  }, [selectedAccountId]);

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
      const result = await invoke<Array<{ id: string; name: string }>>('list_accounts');
      const accs = result || [];
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
      if (msg.includes('cancelled') || msg.includes('cancel')) {
        setShowPasswordInput(true);
      } else {
        setBioError(msg.slice(0, 200));
        setShowPasswordInput(true);
      }
    } finally {
      if (!success) setBioLoading(false);
    }
  }, [selectedAccountId, bioLoading, biometryType, t, navigate]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    clearError();
    setBioError(null);
    setSubmitError(null);
    if (!selectedAccountId) {
      setSubmitError(t('auth:no_account_selected'));
      return;
    }
    await login(selectedAccountId, password);
  };

  // Prevent password→biometric flash: show nothing until bio check completes
  if (!bioChecked) {
    return <div style={{ height: '100vh', background: 'var(--bg-base)' } as CSSProperties} />;
  }

  return (
    <div
      style={
        {
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100vh',
        } as CSSProperties
      }
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
        <div
          style={{
            width: 48,
            height: 48,
            borderRadius: 12,
            background: 'linear-gradient(135deg, var(--accent-primary), var(--accent-warm))',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: 'white',
            fontWeight: 700,
            fontSize: 22,
            margin: '0 auto 16px',
          }}
        >
          S
        </div>
        <h1 style={{ fontSize: 20, fontWeight: 600, marginBottom: 4 }}>{t('auth:login_title')}</h1>
        <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 4 }}>
          {t('auth:login_subtitle')}
        </p>
        <p
          style={{
            fontSize: 11,
            color: 'var(--text-tertiary)',
            marginBottom: 24,
            fontFamily: 'monospace',
          }}
        >
          DEBUG accounts={accounts.length} selected={selectedAccountId || 'none'}
        </p>

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
              <span style={{ fontSize: 15, fontWeight: 500, color: 'var(--text-primary)' }}>
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
                fontSize: 13,
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
        {(showPasswordInput || !bioAvailable) && (
          <form
            onSubmit={handleSubmit}
            style={{ display: 'flex', flexDirection: 'column', gap: 16 }}
          >
            {accounts.length > 1 ? (
              <select
                value={selectedAccountId}
                onChange={(e) => setSelectedAccountId(e.target.value)}
                style={{
                  padding: '10px 14px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  color: 'var(--text-primary)',
                  fontSize: 14,
                  fontFamily: 'inherit',
                  outline: 'none',
                }}
              >
                {accounts.map((acc) => (
                  <option key={acc.id} value={acc.id}>
                    {acc.name}
                  </option>
                ))}
              </select>
            ) : accounts.length === 1 ? (
              <div
                style={{
                  padding: '10px 14px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-elevated)',
                  color: 'var(--text-primary)',
                  fontSize: 14,
                  textAlign: 'left',
                }}
              >
                {selectedAccount?.name ?? accounts[0]?.name}
              </div>
            ) : null}
            <SecurePasswordInput
              value={password}
              onChange={(v) => setPassword(v)}
              placeholder={t('common:password_placeholder')}
              hint={selectedAccount?.passwordHint || null}
              autoComplete="current-password"
            />
            {(error || bioError || submitError) && (
              <div style={{ color: '#e74c3c', fontSize: 13 }}>
                {submitError || bioError || (error
                  ? error.toLowerCase().includes('password') ||
                    error.toLowerCase().includes('invalid')
                    ? t('auth:incorrect_password')
                    : error.toLowerCase().includes('required')
                      ? t('auth:password_required')
                      : error
                  : '')}
              </div>
            )}
            <Button type="submit" loading={isLoading} style={{ width: '100%' }}>
              {t('auth:login_button')}
            </Button>
            {bioAvailable && (
              <button
                onClick={() => {
                  setShowPasswordInput(false);
                  setBioError(null);
                }}
                className={styles.loginTextButton}
                style={{
                  fontSize: 13,
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
