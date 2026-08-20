import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MIN_PASSWORD_LENGTH } from '@/lib/constants';

export interface UseRecoveryCredentialsOptions {
  /** 密码输入被修改时回调（父 hook 用于清空账户冲突态）。 */
  onEdited: () => void;
}

/** 账户阶段（设置本机主密码）的密码表单状态与校验逻辑。 */
export function useRecoveryCredentials({ onEdited }: UseRecoveryCredentialsOptions) {
  const { t } = useTranslation(['common']);

  const [masterPassword, setMasterPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [passwordHint, setPasswordHint] = useState('');
  const [masterPasswordError, setMasterPasswordError] = useState<string | null>(null);
  const [confirmPasswordError, setConfirmPasswordError] = useState<string | null>(null);

  // ── 密码输入变更：清空对应校验错误，并通知父 hook（清账户冲突态） ──
  const handleMasterPasswordChange = useCallback(
    (v: string) => {
      setMasterPassword(v);
      onEdited();
      if (masterPasswordError) setMasterPasswordError(null);
    },
    [masterPasswordError, onEdited],
  );

  const handleConfirmPasswordChange = useCallback(
    (v: string) => {
      setConfirmPassword(v);
      onEdited();
      if (confirmPasswordError) setConfirmPasswordError(null);
    },
    [confirmPasswordError, onEdited],
  );

  /**
   * 校验优先级（与创建账户页一致）：主密码未输入 > 主密码不符合要求 > 确认密码未输入 >
   * 两次密码不一致；密码提示词为可选字段不校验。长度不足/不一致设置对应输入框 error，
   * 触发抖动+红边+红字。校验通过返回 null。
   */
  const getValidationError = useCallback((): string | null => {
    if (!masterPassword) {
      setMasterPasswordError(t('common:master_password_required'));
      return t('common:master_password_required');
    }
    if (masterPassword.length < MIN_PASSWORD_LENGTH) {
      setMasterPasswordError(t('common:password_length_requirement'));
      return t('common:password_length_requirement');
    }
    if (!confirmPassword) {
      setConfirmPasswordError(t('common:confirm_password_required'));
      return t('common:confirm_password_required');
    }
    if (masterPassword !== confirmPassword) {
      setConfirmPasswordError(t('common:password_mismatch'));
      return t('common:password_mismatch');
    }
    return null;
  }, [masterPassword, confirmPassword, t]);

  const reset = useCallback(() => {
    setMasterPassword('');
    setConfirmPassword('');
    setPasswordHint('');
    setMasterPasswordError(null);
    setConfirmPasswordError(null);
  }, []);

  return {
    masterPassword,
    confirmPassword,
    passwordHint,
    masterPasswordError,
    confirmPasswordError,
    setPasswordHint,
    handleMasterPasswordChange,
    handleConfirmPasswordChange,
    getValidationError,
    reset,
  };
}
