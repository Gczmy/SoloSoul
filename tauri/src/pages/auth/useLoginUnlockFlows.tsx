import { useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useShallow } from 'zustand/react/shallow';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore, saveLastAccountId } from '@/stores/authStore';
import type { AccountInfo } from '@/lib/ipc';
import { getBiometricErrorMessage } from '@/lib/biometricError';
import { translateRustError } from '@/lib/rustErrors';
import { logger } from '@/lib/logger';
import type { PinInputHandle } from '@/components/forms/PinInput';

import type { LoginMethod } from './useLoginPage';

export interface UseLoginUnlockFlowsOptions {
  selectedAccountId: string;
  /** 从已有外部目录登录时复位安全标志（config.json 残留） */
  fromExisting: boolean;
  /** 系统生物识别临时锁定标记（父 hook 探测结果，解锁时据此直接降级） */
  bioLockout: boolean;
  biometryTypeRaw: string;
  setLoginMethod: (method: LoginMethod) => void;
  setBioLockout: (v: boolean) => void;
  setPinAvailable: (v: boolean) => void;
}

/**
 * 三种解锁流程（W001-② 拆分：数据 hook）。
 * PIN 解锁 / 生物识别解锁 / 主密码登录三个 handler 与各自的状态
 * （password、bioLoading/bioError、pinUnlocking/pinError、行内密码错误、
 * submitError、pinInputKey/Ref）收敛于此；父 hook 仅组合与透传。
 */
export function useLoginUnlockFlows({
  selectedAccountId,
  fromExisting,
  bioLockout,
  biometryTypeRaw,
  setLoginMethod,
  setBioLockout,
  setPinAvailable,
}: UseLoginUnlockFlowsOptions) {
  const navigate = useNavigate();
  const { t } = useTranslation(['auth', 'common', 'settings']);
  // P022: useShallow 字段级选择——避免 store 无关字段翻转时整 hook 重渲染
  const { login, listAccounts, clearError } = useAuthStore(
    useShallow((s) => ({
      login: s.login,
      listAccounts: s.listAccounts,
      clearError: s.clearError,
    })),
  );

  const [password, setPassword] = useState('');
  const [submitError, setSubmitError] = useState<string | null>(null);
  /** 主密码输入框行内错误（空密码 / 后端密码错误），红边 + 抖动。 */
  const [passwordFieldError, setPasswordFieldError] = useState<string | null>(null);
  /** 密码错误自增计数：同串错误（Invalid password）重复提交时也重新抖动。 */
  const [passwordErrorTick, setPasswordErrorTick] = useState(0);
  const [bioLoading, setBioLoading] = useState(false);
  const [bioError, setBioError] = useState<string | null>(null);
  const [pinUnlocking, setPinUnlocking] = useState(false);
  const [pinError, setPinError] = useState<string | null>(null);
  const [pinInputKey, setPinInputKey] = useState(0);
  const pinInputRef = useRef<PinInputHandle>(null);

  // P034: 组件卸载时清空密码 state（登录成功导航离开 / 锁定返回登录页时缩短驻留）
  useEffect(() => {
    return () => setPassword('');
  }, []);

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
        // P015: 收敛到 authStore action，不再直改 setState
        useAuthStore.getState().completeUnlock(acc);
        // PIN 解锁后延迟检查备份提醒（P228: accountId 注入，避免循环依赖）
        const pinUnlockedAccountId = acc.id;
        setTimeout(() => {
          import('@/lib/notification')
            .then((m) => m.checkBackupReminder(pinUnlockedAccountId))
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
    [selectedAccountId, pinUnlocking, t, navigate, setPinAvailable, setLoginMethod],
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
      // P015: 收敛到 authStore action，不再直改 setState
      useAuthStore.getState().completeUnlock(acc, accs);
      success = true;
      (window as typeof window & { __SOLOSOUL_UNLOCK_TIME?: number }).__SOLOSOUL_UNLOCK_TIME = t0;
      // 生物识别解锁后延迟检查备份提醒（P228: accountId 注入，避免循环依赖）
      const bioUnlockedAccountId = acc.id;
      setTimeout(() => {
        import('@/lib/notification')
          .then((m) => m.checkBackupReminder(bioUnlockedAccountId))
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
  }, [
    selectedAccountId,
    bioLoading,
    t,
    navigate,
    biometryTypeRaw,
    bioLockout,
    setBioLockout,
    setLoginMethod,
  ]);

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    clearError();
    setBioError(null);
    setSubmitError(null);
    // 注意：此处不清除 passwordFieldError —— 重复提交同串错误时靠 errorTick 重抖；
    // 若在提交开头清除，Argon2 校验期间行内错误会闪烁消失（用户报的闪烁问题）。
    if (!selectedAccountId) {
      setSubmitError(t('auth:no_account_selected'));
      return;
    }
    // 空密码前置校验：不发后端请求，输入框行内必填错误（消除 Argon2 往返与 div 切换闪烁）
    if (!password) {
      setPasswordFieldError(t('auth:password_required'));
      setPasswordErrorTick((n) => n + 1);
      return;
    }
    await login(selectedAccountId, password);
    // 后端密码类错误（Invalid password / Verify failed）：i18n 后挂到输入框行内；
    // 其他后端错误回退到 submitError（独立错误区展示），避免被静默丢弃
    const state = useAuthStore.getState();
    if (state.error) {
      const translated = translateRustError(state.error);
      if (translated === 'common:invalid_password' || translated === 'common:verify_failed') {
        setPasswordFieldError(t(translated));
        setPasswordErrorTick((n) => n + 1);
      } else {
        // 非密码错误：清除可能残留的主密码行内错误，避免与 submitError 同时展示
        setPasswordFieldError(null);
        setSubmitError(translated ? t(translated) : state.error);
      }
    }
    // P034: 登录成功立即清空密码 state（JS 堆不可清零，尽早缩短驻留窗口）
    if (!state.error) {
      setPassword('');
    }
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

  return {
    password,
    setPassword,
    passwordFieldError,
    setPasswordFieldError,
    passwordErrorTick,
    bioLoading,
    bioError,
    setBioError,
    pinUnlocking,
    pinError,
    setPinError,
    pinInputKey,
    pinInputRef,
    submitError,
    setSubmitError,
    handlePinComplete,
    handleBiometricUnlock,
    handleSubmit,
  };
}
