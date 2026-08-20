/**
 * PIN 禁用确认域（PinSection）：共享 PasswordVerificationDialog 验证主密码后执行 pin_disable。
 */
import { useState, useCallback } from 'react';
import type { TFunction } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';

export interface UsePinDisableOptions {
  accountId: string;
  t: TFunction;
  onSuccess: (message: string) => void;
  /** 完成后刷新 PIN 状态。 */
  refreshStatus: () => Promise<void>;
  /** 提交期间的 loading 标志（与设置流程共享，由父组件持有）。 */
  setPinLoading: (v: boolean) => void;
}

export function usePinDisable({
  accountId,
  t,
  onSuccess,
  refreshStatus,
  setPinLoading,
}: UsePinDisableOptions) {
  // Disable flow（P012：禁用确认统一走共享 PasswordVerificationDialog）
  const [showDisableConfirm, setShowDisableConfirm] = useState(false);
  const [disablePasswordError, setDisablePasswordError] = useState<string | null>(null);

  const handleDisableStart = useCallback(() => {
    setDisablePasswordError(null);
    setShowDisableConfirm(true);
  }, []);

  /** 共享 PasswordVerificationDialog 的验证回调（P012）：成功即执行 pin_disable */
  const handleDisableVerify = useCallback(
    async (password: string): Promise<boolean> => {
      setDisablePasswordError(null);
      setPinLoading(true);
      try {
        await invoke('pin_disable', { accountId: accountId, password });
        onSuccess(t('settings:pin_disabled_toast'));
        setShowDisableConfirm(false);
        void refreshStatus();
        return true;
      } catch (e) {
        const msg = String(e);
        if (msg.includes('__PIN_ERR__:invalid_password')) {
          setDisablePasswordError(t('settings:current_password_incorrect'));
        } else if (msg.includes('__PIN_ERR__:locked')) {
          // P012：主密码阶梯锁定（pin_disable 验证主密码被锁）——区别显示锁定文案
          setDisablePasswordError(t('common:password_locked'));
        } else {
          setDisablePasswordError(t('settings:pin_error_disable_failed'));
        }
        return false;
      } finally {
        setPinLoading(false);
      }
    },
    [accountId, t, onSuccess, refreshStatus, setPinLoading],
  );

  const closeDisable = useCallback(() => {
    setShowDisableConfirm(false);
    setDisablePasswordError(null);
  }, []);

  /** 密码输入变化时清除验证错误（不关闭弹窗）。 */
  const clearDisablePasswordError = useCallback(() => {
    setDisablePasswordError(null);
  }, []);

  return {
    showDisableConfirm,
    disablePasswordError,
    handleDisableStart,
    handleDisableVerify,
    closeDisable,
    clearDisablePasswordError,
  };
}
