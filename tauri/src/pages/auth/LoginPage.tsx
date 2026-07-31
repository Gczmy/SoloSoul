import { useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore, saveLastAccountId, LAST_ACCOUNT_KEY } from '@/stores/authStore';
import { useApplyThemeFromSettings } from '@/hooks/useApplyThemeFromSettings';
import type { AccountInfo } from '@/lib/ipc';
import { getBiometricErrorMessage } from '@/lib/biometricError';
import { logger } from '@/lib/logger';

import { ShieldLogo } from '@/components/ui/ShieldLogo';
import type { PinInputHandle } from '@/components/forms/PinInput';
import { RecoveryReceiveDialog } from '@/components/recovery/RecoveryReceiveDialog';
import { Fingerprint, KeyRound, ScanFace, ShieldCheck, Grip } from 'lucide-react';
import styles from './LoginPage.module.css';
import { ICON_SIZE } from '@/lib/constants';

import { LoginBiometricView } from './LoginBiometricView';
import { LoginPinView } from './LoginPinView';
import { LoginPasswordView } from './LoginPasswordView';

/** P038: 受支持的生物识别类型白名单（显示名由 LoginBiometricView 的查表负责） */
const BIOMETRIC_INFO: Record<string, string> = {
  faceId: 'faceId',
  touchId: 'touchId',
  windowsHello: 'windowsHello',
};

/** DEBUG: 设为 true 时，底部图标栏始终显示全部 5 种解锁方式，且生物识别卡片可切换显示全部 3 种 */
const __DEBUG_SHOW_ALL = false;

/** 模块级缓存 — 跨组件卸载持久化，避免锁定后重新挂载时闪烁 */
let _cachedLoginMethod: 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password' | null = null;

export function LoginPage() {
  useApplyThemeFromSettings();
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
  const [searchParams] = useSearchParams();
  const fromExisting = searchParams.get('fromExisting') === 'true';
  const [selectedAccountId, setSelectedAccountId] = useState('');
  const [password, setPassword] = useState('');
  const { t } = useTranslation(['auth', 'common', 'settings']);
  const selectedAccount = accounts.find((a) => a.id === selectedAccountId);

  // Biometric state
  const [bioAvailable, setBioAvailable] = useState(false);
  const [biometryTypeRaw, setBiometryTypeRaw] = useState('touchId');
  const [bioLoading, setBioLoading] = useState(false);
  const [bioError, setBioError] = useState<string | null>(null);
  const [bioChecked, setBioChecked] = useState(false);
  // 系统生物识别因失败次数过多被临时锁定（Android）：指纹项仍显示，但点击时提示警告
  const [bioLockout, setBioLockout] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  // PIN state
  const [pinAvailable, setPinAvailable] = useState(false);
  const [pinChecked, setPinChecked] = useState(false);
  const [pinUnlocking, setPinUnlocking] = useState(false);
  const [pinError, setPinError] = useState<string | null>(null);
  const [pinInputKey, setPinInputKey] = useState(0);
  const pinInputRef = useRef<PinInputHandle>(null);
  const [hoveredIcon, setHoveredIcon] = useState<string | null>(null);
  const [committedIcon, setCommittedIcon] = useState<string | null>(null);
  const commitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [recoveryOpen, setRecoveryOpen] = useState(false);

  // 移动端启动性能基线：登录页首个输入框获焦时记录 T1（MOB-P1-07）
  // 注意 T1 从 __SOLOSOUL_APP_START_TIME 开始算，记录首个输入框 focus 时刻。
  const t1FiredRef = useRef(false);

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
    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;
    // 到达登录页即撤掉原生锁屏遮盖层（Android；其他平台为 no-op）。
    // 覆盖冷启动路径：进程被杀后持久化标记触发遮盖，但 JS 事件可能丢失，
    // 冷启动后 Vault 内存密钥已不存在，必然落在登录页，挂载即撤遮盖。
    invoke('dismiss_lock_mask').catch(() => {});
    checkHasAccount().then(() => {
      if (!ctrl.signal.aborted) listAccounts();
    });
    // Probe device biometry type early (do not set bioAvailable here — configured
    // status is account-specific and decided in the selectedAccountId effect).
    invoke<{ available: boolean; biometryType?: string }>('biometric_check_availability', {
      accountId: '',
    })
      .then((r) => {
        if (ctrl.signal.aborted) return;
        const info = r.biometryType ? BIOMETRIC_INFO[r.biometryType] : undefined;
        if (info) {
          setBiometryTypeRaw(info);
        }
      })
      .catch(() => {
        // Ignore: account-specific check will handle availability.
      });
    // 每次回到前台时撤掉原生锁屏遮盖层：
    // Kotlin LockStatePlugin.onResume() 在进程被杀恢复后可能重新挂遮罩，
    // 而 useAutoLock 的 screen-locked 监听器在 isAuthenticated 变为 false
    // 时已被清理，导致无人调 dismiss_lock_mask。此兜底确保无论何时回到前台都撤除。
    const onVisibleDismissMask = () => {
      if (document.visibilityState === 'visible') {
        invoke('dismiss_lock_mask').catch(() => {});
      }
    };
    document.addEventListener('visibilitychange', onVisibleDismissMask);
    return () => {
      document.removeEventListener('visibilitychange', onVisibleDismissMask);
      ctrl.abort();
    };
  }, [checkHasAccount, listAccounts]);

  useEffect(() => {
    if (hasAccount === false) navigate('/bootstrap');
    if (isAuthenticated) navigate('/');
  }, [hasAccount, isAuthenticated, navigate]);

  // Auto-select last logged-in account (fall back to first account)
  useEffect(() => {
    if (accounts.length > 0 && !selectedAccountId) {
      let lastId = '';
      try {
        lastId = localStorage.getItem(LAST_ACCOUNT_KEY) || '';
      } catch {
        lastId = '';
      }
      const target = accounts.find((a) => a.id === lastId) || accounts[0];
      setSelectedAccountId(target.id);
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
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    if (!selectedAccountId) {
      // 尚未选中账户时保持缓存值，不触发优先级设置 effect，避免覆盖缓存
      return () => controller.abort();
    }

    // 从 SAF 已有账户登录时，旧安装的生物识别/PIN 凭证已失效，强制仅显示主密码
    if (fromExisting) {
      setBioChecked(true);
      setPinChecked(true);
      setBioAvailable(false);
      setPinAvailable(false);
      setBioLockout(false);
      return () => controller.abort();
    }

    // Reset state when account changes — 不重置 loginMethod，保留缓存值避免闪烁
    setBioChecked(false);
    setPinChecked(false);
    setBioAvailable(false);
    setPinAvailable(false);
    setBioLockout(false);
    setBioError(null);
    setPinError(null);
    setSubmitError(null);

    // Check biometric
    // lockout 场景：系统因失败次数过多临时锁定生物识别（Android canAuthenticate 返回
    // ERROR_LOCKOUT），此时后端 available 会变 false，但凭证仍已配置（configured=true）。
    // 指纹项应继续显示，仅在解锁时提示"系统指纹识别未恢复"，而不是消失或显示"不支持"。
    invoke<{
      available: boolean;
      configured: boolean;
      biometryType?: string;
      lockout?: boolean;
    }>('biometric_check_availability', { accountId: selectedAccountId })
      .then((r) => {
        if (controller.signal.aborted) return;
        // 已配置凭证且（设备可用或系统临时锁定）→ 保留指纹项。
        // lockout 以 !!r.lockout 为准：即使 available 与 lockout 同时成立
        // （Android 插件 status() 在锁定期间可能仍报可用），也正确显示警告。
        if (r.configured && (r.available || r.lockout)) {
          setBioAvailable(true);
          setBioLockout(!!r.lockout);
          const info = r.biometryType ? BIOMETRIC_INFO[r.biometryType] : undefined;
          if (info) {
            setBiometryTypeRaw(info);
          }
        } else {
          setBioAvailable(false);
          setBioLockout(false);
        }
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setBioAvailable(false);
      })
      .finally(() => {
        if (!controller.signal.aborted) setBioChecked(true);
      });

    // Check PIN
    invoke<{ configured: boolean; locked: boolean }>('pin_check_availability', {
      accountId: selectedAccountId,
    })
      .then((r) => {
        if (controller.signal.aborted) return;
        setPinAvailable(r.configured && !r.locked);
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setPinAvailable(false);
      })
      .finally(() => {
        if (!controller.signal.aborted) setPinChecked(true);
      });

    return () => controller.abort();
  }, [selectedAccountId, fromExisting]);

  // 卸载时清理悬停延迟定时器
  useEffect(() => {
    return () => {
      if (commitTimerRef.current) clearTimeout(commitTimerRef.current);
    };
  }, []);

  // Set login method by priority after both checks complete
  useEffect(() => {
    if (!bioChecked || !pinChecked) return;

    // Priority: FaceID > Touch ID > Windows Hello > PIN > Password
    if (__DEBUG_SHOW_ALL || bioAvailable) {
      const raw = __DEBUG_SHOW_ALL ? 'touchId' : biometryTypeRaw;
      if (raw === 'faceId') setLoginMethod('faceId');
      else if (raw === 'touchId') setLoginMethod('touchId');
      else if (raw === 'windowsHello') setLoginMethod('windowsHello');
      else setLoginMethod('password');
    } else if (__DEBUG_SHOW_ALL || pinAvailable) {
      setLoginMethod('pin');
    } else {
      setLoginMethod('password');
    }
  }, [bioChecked, pinChecked, bioAvailable, pinAvailable, biometryTypeRaw]);

  const handlePinComplete = useCallback(
    async (pin: string) => {
      if (!selectedAccountId || pinUnlocking) return;
      setPinUnlocking(true);
      setPinError(null);
      const t0 = performance.now();
      try {
        // pin_unlock 直接返回账户信息（id + name），省去额外 vault_list_accounts 调用
        const acc = await invoke<AccountInfo>('pin_unlock', {
          accountId: selectedAccountId,
          pin,
          location: 'login_page',
          action: 'unlock',
        });
        (window as typeof window & { __SOLOSOUL_UNLOCK_TIME?: number }).__SOLOSOUL_UNLOCK_TIME = t0;
        saveLastAccountId(acc.id);
        useAuthStore.setState({ isAuthenticated: true, currentAccount: acc });
        // PIN 解锁后延迟检查备份提醒
        setTimeout(() => {
          import('@/lib/notification')
            .then((m) => m.checkBackupReminder())
            .catch((err) => logger.warn('[LoginPage] backup reminder check failed:', err));
        }, 2000);
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
    },
    [selectedAccountId, pinUnlocking, t, navigate],
  );

  const handleBiometricUnlock = useCallback(async () => {
    if (!selectedAccountId || bioLoading) return;
    // 系统生物识别处于临时锁定状态：不发起原生提示，直接显示警告并降级到主密码
    if (bioLockout) {
      setBioError(t('settings:biometric_lockout_desc'));
      setLoginMethod('password');
      return;
    }
    setBioLoading(true);
    setBioError(null);
    let success = false;
    const t0 = performance.now();
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
      saveLastAccountId(acc.id);
      useAuthStore.setState({ isAuthenticated: true, currentAccount: acc, accounts: accs });
      success = true;
      (window as typeof window & { __SOLOSOUL_UNLOCK_TIME?: number }).__SOLOSOUL_UNLOCK_TIME = t0;
      // 生物识别解锁后延迟检查备份提醒
      setTimeout(() => {
        import('@/lib/notification')
          .then((m) => m.checkBackupReminder())
          .catch((err) => logger.error('[LoginPage] backup reminder check failed:', err));
      }, 2000);
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
      } else if (msg.includes('__BIO_ERR__:lockout') || msg.toLowerCase().includes('lockout')) {
        // 系统临时锁定：保留指纹项显示，标记锁定状态并展示警告
        setBioLockout(true);
        setBioError(t('settings:biometric_lockout_desc'));
        setLoginMethod('password');
      } else {
        setBioError(getBiometricErrorMessage(e, t));
        setLoginMethod('password');
      }
    } finally {
      if (!success) setBioLoading(false);
    }
  }, [selectedAccountId, bioLoading, t, navigate, biometryTypeRaw, bioLockout]);

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
    // 从已有外部目录登录后，config.json 中可能残留旧的安全标志（biometric/pin enabled），
    // 但实际 KeyStore 凭证和 PIN 文件已被卸载清除。立即复位这些标志，
    // 避免用户在安全设置中看到「已启用」但实际无法使用的状态。
    if (fromExisting) {
      try {
        await invoke('reset_security_flags', { accountId: selectedAccountId });
        // 刷新账户列表，让 currentAccount 反映新的 hasBiometricHistory/hasPinHistory 标志，
        // 同时让安全设置页在重新进入时读取到最新的可用性状态。
        await listAccounts();
      } catch {
        // 重置失败不阻断登录流程，用户下次启动时再试
      }
    }
  };

  // ==== 构建可用解锁方式列表 ====
  // 顺序：主密码 → Face ID → Touch ID → Windows Hello → PIN
  const iconMethods: {
    id: 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password';
    icon: React.ReactNode;
    label: string;
    onClick: () => void;
  }[] = [];
  // 1. 主密码（始终可用）
  iconMethods.push({
    id: 'password',
    icon: <KeyRound size={ICON_SIZE.xl} />,
    label: t('auth:password_method', { defaultValue: '主密码' }),
    onClick: () => setLoginMethod('password'),
  });
  // 2. Face ID
  if (__DEBUG_SHOW_ALL || (bioAvailable && biometryTypeRaw === 'faceId')) {
    iconMethods.push({
      id: 'faceId',
      icon: <ScanFace size={ICON_SIZE.xl} />,
      label: 'Face ID',
      onClick: () => {
        setLoginMethod('faceId');
        setBioError(null);
      },
    });
  }
  // 3. Touch ID
  if (__DEBUG_SHOW_ALL || (bioAvailable && biometryTypeRaw === 'touchId')) {
    iconMethods.push({
      id: 'touchId',
      icon: <Fingerprint size={ICON_SIZE.xl} />,
      label: 'Touch ID',
      onClick: () => {
        setLoginMethod('touchId');
        setBioError(null);
      },
    });
  }
  // 4. Windows Hello
  if (__DEBUG_SHOW_ALL || (bioAvailable && biometryTypeRaw === 'windowsHello')) {
    iconMethods.push({
      id: 'windowsHello',
      icon: <ShieldCheck size={ICON_SIZE.xl} />,
      label: 'Windows Hello',
      onClick: () => {
        setLoginMethod('windowsHello');
        setBioError(null);
      },
    });
  }
  // 5. PIN 码
  if (__DEBUG_SHOW_ALL || pinAvailable) {
    iconMethods.push({
      id: 'pin',
      icon: <Grip size={ICON_SIZE.xl} />,
      label: t('auth:pin_method', { defaultValue: 'PIN 码' }),
      onClick: () => {
        setLoginMethod('pin');
        setPinError(null);
      },
    });
  }

  // 两阶段悬停：边框/颜色立即高亮，文字/展开延迟 300ms 后触发
  const handleIconEnter = (id: string) => {
    setHoveredIcon(id);
    if (commitTimerRef.current) clearTimeout(commitTimerRef.current);
    commitTimerRef.current = setTimeout(() => {
      setCommittedIcon(id);
      commitTimerRef.current = null;
    }, 300);
  };

  const handleIconLeave = () => {
    setHoveredIcon(null);
    setCommittedIcon(null);
    if (commitTimerRef.current) {
      clearTimeout(commitTimerRef.current);
      commitTimerRef.current = null;
    }
  };

  const isBiometricMethod =
    loginMethod === 'faceId' || loginMethod === 'touchId' || loginMethod === 'windowsHello';

  return (
    <div className={styles.loginWrapper}>
      <div className={styles.loginCard}>
        <ShieldLogo size={ICON_SIZE['5xl']} style={{ margin: '0 auto 16px' }} />
        <h1 style={{ fontSize: 'var(--text-page-title)', fontWeight: 600, marginBottom: 4 }}>
          {t('auth:login_title')}
        </h1>
        <p
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            marginBottom: 4,
          }}
        >
          {t('auth:login_subtitle')}
        </p>

        {/* Account selector / name — 始终预留空间，避免切换登录方式时下方内容位移 */}
        <div
          style={{ marginBottom: 20, width: '100%', minHeight: accounts.length > 0 ? 'auto' : 50 }}
        >
          {accounts.length > 0 &&
            (accounts.length > 1 ? (
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
        {isBiometricMethod && loginMethod && (
          <LoginBiometricView
            loginMethod={loginMethod}
            bioLoading={bioLoading}
            bioLockout={bioLockout}
            onUnlock={handleBiometricUnlock}
          />
        )}

        {/* PIN unlock — shown when PIN is the highest available method or user chose it */}
        {loginMethod === 'pin' && (
          <LoginPinView
            pinUnlocking={pinUnlocking}
            pinError={pinError}
            pinInputKey={pinInputKey}
            pinInputRef={pinInputRef}
            onPinComplete={handlePinComplete}
          />
        )}

        {/* Password input — 最低优先级；初始化或缓存回退时也显示，避免白屏 */}
        {(loginMethod === 'password' || loginMethod === null) && (
          <LoginPasswordView
            password={password}
            onPasswordChange={setPassword}
            isLoading={isLoading}
            error={error}
            bioError={bioError}
            submitError={submitError}
            pinError={pinError}
            passwordHint={selectedAccount?.passwordHint || null}
            onSubmit={handleSubmit}
            onFocus={() => {
              // T1：首个输入框获焦时记录，仅一次
              if (t1FiredRef.current) return;
              t1FiredRef.current = true;
              const start = (
                window as typeof window & { __SOLOSOUL_APP_START_TIME?: number }
              ).__SOLOSOUL_APP_START_TIME;
              if (typeof start === 'number') {
                // T1 timing is captured internally; no console output in production
              }
            }}
          />
        )}

        {/* 在已有账户的登录页提供创建新账户与从其他设备恢复入口 */}
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: 8,
            marginTop: 16,
          }}
        >
          <button
            type="button"
            onClick={() => navigate('/bootstrap?mode=create')}
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-tertiary)',
              background: 'transparent',
              border: 'none',
              padding: '6px 12px',
              cursor: 'pointer',
              fontFamily: 'inherit',
              transition: 'all 0.15s ease',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.color = 'var(--accent-primary)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.color = 'var(--text-tertiary)';
            }}
          >
            {t('common:create_new_account_link')}
          </button>
          <button
            type="button"
            onClick={() => setRecoveryOpen(true)}
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-tertiary)',
              background: 'transparent',
              border: 'none',
              padding: '6px 12px',
              cursor: 'pointer',
              fontFamily: 'inherit',
              transition: 'all 0.15s ease',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.color = 'var(--accent-primary)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.color = 'var(--text-tertiary)';
            }}
          >
            {t('common:restore_from_device_link')}
          </button>
        </div>

        {/* ===== 底部图标栏 — 切换解锁方式 ===== */}
        {loginMethod !== null && (
          <div
            style={{
              display: 'flex',
              gap: 6,
              paddingTop: 12,
              marginTop: 'auto',
              borderTop: '1px solid var(--border-subtle)',
              justifyContent: 'flex-start',
              overflow: 'hidden',
              maxWidth: '100%',
            }}
          >
            {iconMethods.map((method) => {
              const isActive = loginMethod === method.id;
              const isHovered = hoveredIcon === method.id;
              const isExpanded = committedIcon === method.id;

              return (
                <button
                  key={method.id}
                  aria-label={method.label}
                  onClick={() => {
                    setHoveredIcon(null);
                    setCommittedIcon(null);
                    if (commitTimerRef.current) {
                      clearTimeout(commitTimerRef.current);
                      commitTimerRef.current = null;
                    }
                    method.onClick();
                  }}
                  onMouseEnter={() => handleIconEnter(method.id)}
                  onMouseLeave={handleIconLeave}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                    padding: '6px 10px',
                    borderRadius: 8,
                    border: `1px solid ${
                      isHovered
                        ? 'var(--accent-primary)'
                        : isActive
                          ? 'color-mix(in srgb, var(--accent-primary) 40%, transparent)'
                          : 'transparent'
                    }`,
                    background: isActive
                      ? 'color-mix(in srgb, var(--accent-primary) 6%, transparent)'
                      : 'transparent',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                    fontSize: 'var(--text-body-sm)',
                    color: isHovered
                      ? 'var(--accent-primary)'
                      : isActive
                        ? 'var(--text-primary)'
                        : 'var(--text-tertiary)',
                    whiteSpace: 'nowrap',
                    overflow: 'hidden',
                    maxWidth: isExpanded ? 200 : 40,
                    transition:
                      isExpanded || (!isHovered && !isExpanded)
                        ? 'all 0.25s ease'
                        : 'all 0.25s ease, max-width 0.01s linear 0.2s',
                    flexShrink: 0,
                    outline: 'none',
                  }}
                >
                  <span style={{ flexShrink: 0, display: 'flex', alignItems: 'center' }}>
                    {method.icon}
                  </span>
                  <span
                    style={{
                      opacity: isExpanded ? 1 : 0,
                      transition: 'opacity 0.2s ease 0.05s',
                      overflow: 'hidden',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {method.label}
                  </span>
                </button>
              );
            })}
          </div>
        )}

        <RecoveryReceiveDialog
          isOpen={recoveryOpen}
          onClose={() => setRecoveryOpen(false)}
          onSuccess={() => {
            // 恢复成功后刷新账户列表，让登录页立即显示新恢复的账户
            listAccounts();
          }}
        />
      </div>
    </div>
  );
}
