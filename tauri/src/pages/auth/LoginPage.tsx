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
import { PinInput } from '@/components/forms/PinInput';
import { Fingerprint, KeyRound } from 'lucide-react';
import styles from './LoginPage.module.css';
import { ICON_SIZE } from '@/lib/iconSizes';

/** 模块级缓存 — 跨组件卸载持久化，避免锁定后重新挂载时闪烁 */
let _cachedLoginMethod: 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password' | null = null;

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
  const [bioChecked, setBioChecked] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const makeCancellable = useCancellable();

  // PIN state
  const [pinAvailable, setPinAvailable] = useState(false);
  const [pinChecked, setPinChecked] = useState(false);
  const [pinUnlocking, setPinUnlocking] = useState(false);
  const [pinError, setPinError] = useState<string | null>(null);
  const [pinInputKey, setPinInputKey] = useState(0);

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
        } else if (r.biometryType === 'windowsHello') {
          setBiometryType('Windows Hello');
          setBiometryTypeRaw('windowsHello');
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

  // Priority-based login method selection
  // Priority: FaceID > Touch ID > Windows Hello > PIN > Password
  // 从模块缓存初始化，避免锁定后重新挂载时闪烁
  const [loginMethod, setLoginMethod] = useState<
    'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password' | null
  >(_cachedLoginMethod);

  // 跨卸载持久化 — 锁定再登录后直接显示最后使用的方法
  useEffect(() => {
    if (loginMethod) _cachedLoginMethod = loginMethod;
  }, [loginMethod]);

  // Check biometric and PIN availability for selected account
  useEffect(() => {
    const { isCancelled, cancel } = makeCancellable();
    if (!selectedAccountId) {
      // 尚未选中账户时保持缓存值，不触发优先级设置 effect，避免覆盖缓存
      return cancel;
    }

    // Reset state when account changes — 不重置 loginMethod，保留缓存值避免闪烁
    setBioChecked(false);
    setPinChecked(false);
    setBioAvailable(false);
    setPinAvailable(false);
    setBioError(null);
    setPinError(null);
    setSubmitError(null);

    // Check biometric
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
          } else if (r.biometryType === 'windowsHello') {
            setBiometryType('Windows Hello');
            setBiometryTypeRaw('windowsHello');
          }
        } else {
          setBioAvailable(false);
        }
      })
      .catch(() => {
        if (isCancelled()) return;
        setBioAvailable(false);
      })
      .finally(() => {
        if (!isCancelled()) setBioChecked(true);
      });

    // Check PIN
    invoke<{ configured: boolean; locked: boolean }>(
      'pin_check_availability',
      { accountId: selectedAccountId },
    )
      .then((r) => {
        if (isCancelled()) return;
        setPinAvailable(r.configured && !r.locked);
      })
      .catch(() => {
        if (isCancelled()) return;
        setPinAvailable(false);
      })
      .finally(() => {
        if (!isCancelled()) setPinChecked(true);
      });

    return cancel;
  }, [selectedAccountId, makeCancellable]);

  // Set login method by priority after both checks complete
  useEffect(() => {
    if (!bioChecked || !pinChecked) return;

    // Priority: FaceID > Touch ID > Windows Hello > PIN > Password
    if (bioAvailable) {
      if (biometryTypeRaw === 'faceId') setLoginMethod('faceId');
      else if (biometryTypeRaw === 'touchId') setLoginMethod('touchId');
      else if (biometryTypeRaw === 'windowsHello') setLoginMethod('windowsHello');
      else setLoginMethod('password');
    } else if (pinAvailable) {
      setLoginMethod('pin');
    } else {
      setLoginMethod('password');
    }
  }, [bioChecked, pinChecked, bioAvailable, pinAvailable, biometryTypeRaw]);

  const handlePinComplete = useCallback(async (pin: string) => {
    if (!selectedAccountId || pinUnlocking) return;
    setPinUnlocking(true);
    setPinError(null);
    try {
      await invoke('pin_unlock', { accountId: selectedAccountId, pin });
      // Vault unlocked — set auth state
      const accs = (await invoke<AccountInfo[]>('vault_list_accounts')) || [];
      const acc = accs.find((a) => a.id === selectedAccountId) || {
        id: selectedAccountId,
        name: selectedAccountId,
      };
      useAuthStore.setState({ isAuthenticated: true, currentAccount: acc, accounts: accs });
      navigate('/');
    } catch (e) {
      const msg = String(e);
      if (msg.includes('__PIN_ERR__:locked')) {
        setPinError(t('auth:pin_locked'));
        setPinAvailable(false);
        // 锁定后降级到主密码
        setLoginMethod('password');
      } else if (msg.includes('__PIN_ERR__:incorrect')) {
        setPinError(t('auth:pin_incorrect'));
      } else {
        setPinError(t('auth:pin_error'));
      }
      // 清空 PinInput 控件状态
      setPinInputKey((k) => k + 1);
      setPinUnlocking(false);
    }
  }, [selectedAccountId, pinUnlocking, t, navigate]);

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
        setLoginMethod('password');
      } else {
        setBioError(getBiometricErrorMessage(e, t));
        setLoginMethod('password');
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
          minHeight: 420,
          boxShadow: '0 8px 32px rgba(0,0,0,0.08)',
          textAlign: 'center',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <ShieldLogo size={ICON_SIZE['5xl']} style={{ margin: '0 auto 16px' }} />
        <h1 style={{ fontSize: 'var(--text-page-title)', fontWeight: 600, marginBottom: 4 }}>{t('auth:login_title')}</h1>
        <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', marginBottom: 4 }}>
          {t('auth:login_subtitle')}
        </p>

        {/* Account selector / name — 始终预留空间，避免切换登录方式时下方内容位移 */}
        <div style={{ marginBottom: 20, width: '100%', minHeight: accounts.length > 0 ? 'auto' : 50 }}>
            {accounts.length > 0 && (accounts.length > 1 ? (
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
            ))}
          </div>

        {/* Biometric unlock — highest-priority method */}
        {(loginMethod === 'faceId' || loginMethod === 'touchId' || loginMethod === 'windowsHello') && (
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
                size={ICON_SIZE['4xl']}
                color="var(--accent-primary)"
                style={{ opacity: bioLoading ? 0.5 : 1 }}
              />
              <span style={{ fontSize: 'var(--text-card-title)', fontWeight: 500, color: 'var(--text-primary)' }}>
                {bioLoading
                  ? t('auth:bio_verifying')
                  : t('auth:bio_unlock_reason', { type: biometryType })}
              </span>
            </button>
            {/* Fallback: PIN (lower priority than biometric) */}
            {pinAvailable && (
              <button
                onClick={() => setLoginMethod('pin')}
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
                {t('auth:use_pin_instead')}
              </button>
            )}
            <button
              onClick={() => setLoginMethod('password')}
              className={styles.loginTextButton}
              style={{
                marginTop: 4,
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

        {/* PIN unlock — shown when PIN is the highest available method or user chose it */}
        {loginMethod === 'pin' && (
          <div style={{ marginBottom: 16 }}>
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: 12,
                padding: '16px 24px 20px',
                borderRadius: 14,
                border: '1px solid var(--border-subtle)',
                background: 'transparent',
                width: '100%',
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
                <div style={{ color: '#dc2626', fontSize: 'var(--text-body-sm)' }}>
                  {pinError}
                </div>
              )}
            </div>
            {/* Fallback: biometric (higher priority, show as quick switch) */}
            {bioAvailable && (
              <button
                onClick={() => {
                  if (biometryTypeRaw === 'faceId') setLoginMethod('faceId');
                  else if (biometryTypeRaw === 'touchId') setLoginMethod('touchId');
                  else if (biometryTypeRaw === 'windowsHello') setLoginMethod('windowsHello');
                  setPinError(null);
                }}
                className={styles.loginTextButton}
                style={{
                  marginTop: 8,
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
            <button
              onClick={() => { setLoginMethod('password'); setPinError(null); }}
              className={styles.loginTextButton}
              style={{
                marginTop: 4,
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

        {/* Password input — 最低优先级；初始化或缓存回退时也显示，避免白屏 */}
        {(loginMethod === 'password' || loginMethod === null) && (
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
            {(error || bioError || submitError || pinError) && (
              <div style={{ color: '#dc2626', fontSize: 'var(--text-body-sm)' }}>
                {pinError || submitError || bioError || (error
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
                  if (biometryTypeRaw === 'faceId') setLoginMethod('faceId');
                  else if (biometryTypeRaw === 'touchId') setLoginMethod('touchId');
                  else if (biometryTypeRaw === 'windowsHello') setLoginMethod('windowsHello');
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
            {pinAvailable && (
              <button
                onClick={() => {
                  setLoginMethod('pin');
                  setPinError(null);
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
                {t('auth:use_pin_instead')}
              </button>
            )}
          </form>
        )}
      </div>
    </div>
  );
}
