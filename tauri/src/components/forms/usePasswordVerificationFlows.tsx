import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useToastError } from '@/hooks/useToastError';
import { useAutoLockPauseStore } from '@/stores/autoLockPauseStore';

import type { LoginMethod } from './usePasswordVerification';

export interface UsePasswordVerificationFlowsOptions {
  open: boolean;
  onClose: () => void;
  /** Called with the password. Return true to confirm, false to reject */
  onVerify: (password: string) => Promise<boolean>;
  /** onVerify 返回 true 后回调（多步向导保持打开；不传则验证成功即关闭）。 */
  onVerifySuccess?: () => void;
  /** Called when user clicks biometric button. Return true on success */
  onBiometric?: () => Promise<boolean>;
  /** If provided, enables PIN verification mode */
  pinAccountId?: string;
  /** Called when PIN unlock succeeds */
  onPinSuccess?: () => void;
  /** 密码输入变化回调（父组件用于清空自定义 errorMessage）。 */
  onPasswordChange?: () => void;
  /** 解锁方式切换（父 hook 持有 loginMethod 状态；PIN 锁定降级/打开重置时调用）。 */
  setLoginMethod: (method: LoginMethod | null) => void;
}

/**
 * 统一密码验证对话框的三种解锁流程（W001-⑤ 拆分：数据 hook）。
 * PIN（可用性探测 + handlePinComplete）、主密码（handleConfirm + 行内错误）、
 * 生物识别（handleBiometric）与各自状态收敛于此；父 hook 仅组合与透传。
 * 对话框打开时的状态重置与 auto-lock 暂停亦归此（对话框生命周期语义）。
 */
export function usePasswordVerificationFlows({
  open,
  onClose,
  onVerify,
  onVerifySuccess,
  onBiometric,
  pinAccountId,
  onPinSuccess,
  onPasswordChange,
  setLoginMethod,
}: UsePasswordVerificationFlowsOptions) {
  const { onError } = useToastError();
  const { t } = useTranslation(['auth', 'common', 'settings']);

  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [bioLoading, setBioLoading] = useState(false);
  const [pinAvailable, setPinAvailable] = useState(false);
  const [pinChecked, setPinChecked] = useState(false);
  const [pinUnlocking, setPinUnlocking] = useState(false);
  const [pinError, setPinError] = useState<string | null>(null);
  const [pinInputKey, setPinInputKey] = useState(0);

  // 对话框打开期间暂停自动锁定计时（与 CLI 的 auto_lock_paused 语义一致），
  // 避免用户长时间未输入时验证框被锁定流程变成孤儿状态
  useEffect(() => {
    if (!open) return;
    const { pause, resume } = useAutoLockPauseStore.getState();
    pause();
    return () => resume();
  }, [open]);

  // 对话框打开时重置状态、检查可用性
  useEffect(() => {
    if (!open) {
      setPinChecked(false);
      setPinAvailable(false);
      setLoginMethod(null);
      return;
    }

    setPassword('');
    setError(null);
    setPinError(null);
    setPinUnlocking(false);
    setBioLoading(false);
    setLoginMethod(null);
    setPinChecked(false);
    setPinAvailable(false);

    if (pinAccountId) {
      invoke<{ configured: boolean; locked: boolean }>('pin_check_availability', {
        accountId: pinAccountId,
      })
        .then((r) => setPinAvailable(r.configured && !r.locked))
        .catch(() => setPinAvailable(false))
        .finally(() => setPinChecked(true));
    } else {
      setPinChecked(true);
    }
  }, [open, pinAccountId, setLoginMethod]);

  const handlePinComplete = useCallback(
    async (pin: string) => {
      if (!pinAccountId) return;
      setPinUnlocking(true);
      setPinError(null);
      try {
        await invoke('pin_unlock', {
          accountId: pinAccountId,
          pin,
          location: 'critical_data_access',
          action: 'unlock',
        });
        setPassword('');
        setPinError(null);
        onPinSuccess?.();
      } catch (e) {
        const msg = String(e);
        if (msg.includes('__PIN_ERR__:locked')) {
          setPinError(t('auth:pin_locked'));
          setPinAvailable(false);
          setLoginMethod('password');
        } else if (msg.includes('__PIN_ERR__:incorrect')) {
          setPinError(t('auth:pin_incorrect'));
        } else {
          setPinError(t('auth:pin_error'));
        }
        setPinInputKey((k) => k + 1);
      } finally {
        setPinUnlocking(false);
      }
    },
    [pinAccountId, t, onPinSuccess, setLoginMethod],
  );

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
        if (onVerifySuccess) {
          // 多步向导：验证成功后保持对话框打开，由父组件推进下一步
          onVerifySuccess();
        } else {
          onClose();
        }
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
      // User cancelled or failed — silently stay on screen
    } finally {
      setBioLoading(false);
    }
  };

  // 密码输入变化：清行内错误 + 通知父组件清自定义 errorMessage
  const handlePasswordChange = (v: string) => {
    setPassword(v);
    setError(null);
    onPasswordChange?.();
  };

  return {
    password,
    setPassword,
    error,
    setError,
    loading,
    bioLoading,
    pinAvailable,
    pinChecked,
    pinUnlocking,
    pinError,
    setPinError,
    pinInputKey,
    handlePinComplete,
    handleConfirm,
    handleBiometric,
    handlePasswordChange,
  };
}
