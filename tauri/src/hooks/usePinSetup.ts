/**
 * PIN 设置向导域（PinSection）：密码验证 → 输入 PIN → 确认 PIN 三步状态与提交流程。
 */
import { useState, useCallback } from 'react';
import type { TFunction } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { logger } from '@/lib/logger';
import { invalidateLoginAvailabilityPreflight } from '@/lib/loginAvailabilityPreflight';

export interface UsePinSetupOptions {
  accountId: string;
  t: TFunction;
  onSuccess: (message: string) => void;
  /** 完成后刷新 PIN 状态。 */
  refreshStatus: () => Promise<void>;
  /** 提交期间的 loading 标志（与禁用流程共享，由父组件持有）。 */
  setPinLoading: (v: boolean) => void;
}

export function usePinSetup({
  accountId,
  t,
  onSuccess,
  refreshStatus,
  setPinLoading,
}: UsePinSetupOptions) {
  const [showSetup, setShowSetup] = useState(false);
  const [setupStep, setSetupStep] = useState<'enter_password' | 'enter_pin' | 'confirm_pin'>(
    'enter_password',
  );
  const [setupPassword, setSetupPassword] = useState('');
  const [setupPin1, setSetupPin1] = useState('');
  const [setupError, setSetupError] = useState<string | null>(null);
  /** 共享对话框的自定义错误文案（P012：密码验证段统一走共享组件） */
  const [setupPasswordError, setSetupPasswordError] = useState<string | null>(null);

  const handleSetupStart = useCallback(() => {
    setSetupStep('enter_password');
    setSetupPassword('');
    setSetupPin1('');
    setSetupError(null);
    setSetupPasswordError(null);
    setShowSetup(true);
  }, []);

  /** 共享 PasswordVerificationDialog 的验证回调（P012）：成功推进到 enter_pin 步骤 */
  const handleSetupVerify = useCallback(
    async (password: string): Promise<boolean> => {
      setSetupPasswordError(null);
      try {
        const ok = await invoke<boolean>('verify_password', {
          accountId: accountId,
          password,
        });
        if (ok) {
          setSetupPassword(password);
          return true;
        }
        setSetupPasswordError(t('settings:current_password_incorrect'));
        return false;
      } catch (e) {
        // P123: 后端异常≠密码错误——verify_password 对错误密码返回 false（不抛异常），
        // 走到 catch 的是真实后端故障（锁定/崩溃等），统一报「密码不正确」会误导用户。
        logger.warn('[PinSection] verify_password failed:', e);
        // P012：主密码阶梯锁定的原始错误串（镜像 backendError.ts 精确映射表，前缀匹配）
        setSetupPasswordError(
          String(e).toLowerCase().includes('too many failed attempts')
            ? t('common:password_locked')
            : t('settings:pin_error_setup_failed'),
        );
        return false;
      }
    },
    [accountId, t],
  );

  /** 密码验证通过后由共享对话框回调：推进向导到 PIN 输入步骤 */
  const handleSetupPasswordVerified = useCallback(() => {
    setSetupStep('enter_pin');
  }, []);

  const handlePinEntered = useCallback((pin: string) => {
    setSetupPin1(pin);
    setSetupStep('confirm_pin');
  }, []);

  const handlePinConfirm = useCallback(
    async (pin: string) => {
      if (pin !== setupPin1) {
        setSetupError(t('settings:pin_mismatch'));
        setSetupStep('enter_pin');
        setSetupPin1('');
        return;
      }

      setPinLoading(true);
      setSetupError(null);
      try {
        await invoke('pin_setup', {
          accountId: accountId,
          password: setupPassword,
          pin,
        });
        onSuccess(t('settings:pin_setup_success'));
        setShowSetup(false);
        // 登录方式已变更：失效预探测缓存，锁定后登录页立即反映新状态（不再读到旧结果）
        invalidateLoginAvailabilityPreflight(accountId);
        void refreshStatus();
      } catch (e) {
        const msg = String(e);
        if (msg.includes('__PIN_ERR__:too_short')) {
          setSetupError(t('settings:pin_error_too_short'));
        } else if (msg.includes('__PIN_ERR__:too_long')) {
          setSetupError(t('settings:pin_error_too_long'));
        } else {
          setSetupError(t('settings:pin_error_setup_failed'));
        }
      } finally {
        setPinLoading(false);
      }
    },
    [accountId, setupPassword, setupPin1, t, onSuccess, refreshStatus, setPinLoading],
  );

  const handleSetupCancel = useCallback(() => {
    setShowSetup(false);
    setSetupError(null);
    setSetupPasswordError(null);
  }, []);

  /** 向导内「返回上一步」：confirm_pin → enter_pin（清空已输 PIN）。 */
  const backToEnterPin = useCallback(() => {
    setSetupStep('enter_pin');
    setSetupPin1('');
    setSetupError(null);
  }, []);

  /** 向导内「返回密码验证步骤」（enter_pin → enter_password）。 */
  const goToPasswordStep = useCallback(() => {
    setSetupStep('enter_password');
  }, []);

  /** 密码输入变化时清除验证错误。 */
  const clearSetupPasswordError = useCallback(() => {
    setSetupPasswordError(null);
  }, []);

  return {
    showSetup,
    setupStep,
    setupPassword,
    setupPin1,
    setupError,
    setupPasswordError,
    handleSetupStart,
    handleSetupVerify,
    handleSetupPasswordVerified,
    handlePinEntered,
    handlePinConfirm,
    handleSetupCancel,
    backToEnterPin,
    goToPasswordStep,
    clearSetupPasswordError,
  };
}
