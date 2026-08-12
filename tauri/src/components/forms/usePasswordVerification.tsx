import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';

import { usePasswordVerificationFlows } from './usePasswordVerificationFlows';
import { usePasswordVerificationIconBar } from './usePasswordVerificationIconBar';

export interface PasswordVerificationDialogProps {
  open: boolean;
  onClose: () => void;
  /** Called with the password. Return true to confirm, false to reject */
  onVerify: (password: string) => Promise<boolean>;
  /**
   * onVerify 返回 true 后回调（用于「验证成功后继续向导而非关闭」的多步流程，
   * 如 PIN 设置向导；不传则维持默认行为：验证成功即关闭）。
   */
  onVerifySuccess?: () => void;
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
  /**
   * 动态错误文案（优先于内置 auth:incorrect_password）。
   * 用于父组件需要展示自定义错误语义的场景（如生物识别错误码、锁定提示），
   * 父组件在 onVerify 返回 false 前设置；密码输入变化时需自行清空（或使用 onPasswordChange）。
   */
  errorMessage?: string | null;
  /** 密码输入变化回调（父组件用于清空自定义 errorMessage）。 */
  onPasswordChange?: () => void;
}

/** 生物识别类型的可读标签映射 */
const BIOMETRIC_LABEL: Record<string, string> = {
  faceId: 'Face ID',
  touchId: 'Touch ID',
  windowsHello: 'Windows Hello',
};

/** 解锁方式类型（与 LoginIconBar 的 LoginMethodOption.id 对齐）。 */
export type LoginMethod = 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password';

/**
 * 统一密码验证对话框的全部编排逻辑（P046 拆分：数据 hook；W001-⑤ 再拆后为组合层）。
 * 解锁方式状态（loginMethod）与优先级推导、生物识别标签推导保留于此；
 * 三种解锁流程（PIN/密码/生物识别 + 各自状态）收敛于 usePasswordVerificationFlows，
 * 底部图标栏状态收敛于 usePasswordVerificationIconBar。
 * PasswordVerificationDialog 组件退化为纯展示组合层。
 */
export function usePasswordVerification(props: PasswordVerificationDialogProps) {
  const {
    open,
    onClose,
    onVerify,
    onVerifySuccess,
    biometricType,
    onBiometric,
    pinAccountId,
    onPinSuccess,
    errorMessage,
    onPasswordChange,
  } = props;

  const { t } = useTranslation(['auth', 'common', 'settings']);

  const hasBiometric = !!biometricType && !!onBiometric;

  // 当前显示的解锁方式（按优先级选择）
  const [loginMethod, setLoginMethod] = useState<LoginMethod | null>(null);

  // 三种解锁流程（PIN / 主密码 / 生物识别）+ 各自状态
  const flows = usePasswordVerificationFlows({
    open,
    onClose,
    onVerify,
    onVerifySuccess,
    onBiometric,
    pinAccountId,
    onPinSuccess,
    onPasswordChange,
    setLoginMethod,
  });

  // 稳定 setter 解构：供组合 handler 复用（useState setter 身份稳定）
  const { setPassword, setError, setPinError } = flows;

  // PIN 检查完成后按优先级设置默认解锁方式
  useEffect(() => {
    if (!open || !flows.pinChecked) return;

    // Priority: FaceID > Touch ID > Windows Hello > PIN > Password
    if (hasBiometric) {
      const raw = biometricType || '';
      if (raw === 'faceId') setLoginMethod('faceId');
      else if (raw === 'touchId') setLoginMethod('touchId');
      else if (raw === 'windowsHello') setLoginMethod('windowsHello');
      else setLoginMethod('password');
    } else if (flows.pinAvailable) {
      setLoginMethod('pin');
    } else {
      setLoginMethod('password');
    }
  }, [open, flows.pinChecked, hasBiometric, flows.pinAvailable, biometricType]);

  // 底部图标栏：可用解锁方式列表 + 两阶段悬停状态
  // 选择方式时按原内联行为清对应错误（PIN 清 pinError，其余不清）
  const iconBar = usePasswordVerificationIconBar({
    hasBiometric,
    biometricType,
    pinAvailable: flows.pinAvailable,
    onSelectMethod: (method) => {
      setLoginMethod(method);
      if (method === 'pin') {
        setPinError(null);
      }
    },
  });

  // 从 loginMethod 推导生物识别类型标签（与当前卡片图标保持一致）
  const activeBioType =
    loginMethod === 'faceId' || loginMethod === 'touchId' || loginMethod === 'windowsHello'
      ? loginMethod
      : biometricType || '';
  const biometricLabel =
    BIOMETRIC_LABEL[activeBioType] ||
    activeBioType ||
    t('auth:bio_default', { defaultValue: 'Biometric' });

  const handleClose = () => {
    setPassword('');
    setError(null);
    setLoginMethod(null);
    setPinError(null);
    // handleIconLeave 语义即清除悬停状态（清 hovered/committed + 定时器），复用
    iconBar.handleIconLeave();
    onClose();
  };

  return {
    // 展示层需要
    loginMethod,
    bioLoading: flows.bioLoading,
    biometricLabel,
    handleBiometric: flows.handleBiometric,
    pinUnlocking: flows.pinUnlocking,
    pinError: flows.pinError,
    pinInputKey: flows.pinInputKey,
    handlePinComplete: flows.handlePinComplete,
    password: flows.password,
    handlePasswordChange: flows.handlePasswordChange,
    inputError: errorMessage ?? flows.error,
    loading: flows.loading,
    handleConfirm: flows.handleConfirm,
    handleClose,
    methods: iconBar.methods,
    hoveredIcon: iconBar.hoveredIcon,
    committedIcon: iconBar.committedIcon,
    handleIconEnter: iconBar.handleIconEnter,
    handleIconLeave: iconBar.handleIconLeave,
    handleIconClick: iconBar.handleIconClick,
  };
}
