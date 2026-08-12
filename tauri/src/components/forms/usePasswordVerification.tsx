import { useState, useEffect, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useToastError } from '@/hooks/useToastError';
import { useAutoLockPauseStore } from '@/stores/autoLockPauseStore';
import { Fingerprint, KeyRound, ScanFace, ShieldCheck, Grip } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { supportsHover } from '@/lib/platform';
import type { LoginMethodOption } from '@/pages/auth/LoginIconBar';

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

/** 解锁方式定义（id 与 LoginIconBar 的 LoginMethodOption 对齐，便于复用） */
interface UnlockMethodDef {
  id: 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password';
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}

/**
 * 统一密码验证对话框的全部编排逻辑（P046 拆分：数据 hook）。
 * 密码/生物识别/PIN 三态、可用性探测、解锁方式优先级、三个验证 handler
 * 与底部图标栏状态均收敛于此，PasswordVerificationDialog 组件退化为纯展示组合层。
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

  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [bioLoading, setBioLoading] = useState(false);
  const [pinAvailable, setPinAvailable] = useState(false);
  const [pinChecked, setPinChecked] = useState(false);
  const [pinUnlocking, setPinUnlocking] = useState(false);
  const [pinError, setPinError] = useState<string | null>(null);
  const [pinInputKey, setPinInputKey] = useState(0);
  const [hoveredIcon, setHoveredIcon] = useState<string | null>(null);
  const [committedIcon, setCommittedIcon] = useState<string | null>(null);
  const commitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { onError } = useToastError();
  const { t } = useTranslation(['auth', 'common', 'settings']);

  const hasBiometric = !!biometricType && !!onBiometric;

  // 当前显示的解锁方式（按优先级选择）
  const [loginMethod, setLoginMethod] = useState<
    'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password' | null
  >(null);

  // 从 loginMethod 推导生物识别类型标签（与当前卡片图标保持一致）
  const activeBioType =
    loginMethod === 'faceId' || loginMethod === 'touchId' || loginMethod === 'windowsHello'
      ? loginMethod
      : biometricType || '';
  const biometricLabel =
    BIOMETRIC_LABEL[activeBioType] ||
    activeBioType ||
    t('auth:bio_default', { defaultValue: 'Biometric' });

  // 卸载时清理悬停延迟定时器
  useEffect(() => {
    return () => {
      if (commitTimerRef.current) clearTimeout(commitTimerRef.current);
    };
  }, []);

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
  }, [open, pinAccountId]);

  // PIN 检查完成后按优先级设置默认解锁方式
  useEffect(() => {
    if (!open || !pinChecked) return;

    // Priority: FaceID > Touch ID > Windows Hello > PIN > Password
    if (hasBiometric) {
      const raw = biometricType || '';
      if (raw === 'faceId') setLoginMethod('faceId');
      else if (raw === 'touchId') setLoginMethod('touchId');
      else if (raw === 'windowsHello') setLoginMethod('windowsHello');
      else setLoginMethod('password');
    } else if (pinAvailable) {
      setLoginMethod('pin');
    } else {
      setLoginMethod('password');
    }
  }, [open, pinChecked, hasBiometric, pinAvailable, biometricType]);

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
    [pinAccountId, t, onPinSuccess],
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

  const handleClose = () => {
    setPassword('');
    setError(null);
    setLoginMethod(null);
    setPinError(null);
    setCommittedIcon(null);
    if (commitTimerRef.current) clearTimeout(commitTimerRef.current);
    onClose();
  };

  // 两阶段悬停：边框/颜色立即高亮，文字/展开延迟 200ms 后触发
  const handleIconEnter = (id: string) => {
    // 触屏设备不触发悬停展开（Android WebView hover 会粘住）
    if (!supportsHover()) return;
    setHoveredIcon(id);
    // 清除上一次的定时器
    if (commitTimerRef.current) clearTimeout(commitTimerRef.current);
    // 200ms 后提交展开状态
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

  const handleIconClick = (method: LoginMethodOption) => {
    setHoveredIcon(null);
    setCommittedIcon(null);
    if (commitTimerRef.current) {
      clearTimeout(commitTimerRef.current);
      commitTimerRef.current = null;
    }
    method.onClick();
  };

  // 密码输入变化：清行内错误 + 通知父组件清自定义 errorMessage
  const handlePasswordChange = (v: string) => {
    setPassword(v);
    setError(null);
    onPasswordChange?.();
  };

  // ==== 构建可用解锁方式列表 ====
  // 顺序：主密码 → Face ID → Touch ID → Windows Hello → PIN
  const methods: UnlockMethodDef[] = [];
  // 1. 主密码（始终可用）
  methods.push({
    id: 'password',
    icon: <KeyRound size={ICON_SIZE.xl} />,
    label: t('auth:password_method', { defaultValue: '主密码' }),
    onClick: () => setLoginMethod('password'),
  });

  // 2–4. 生物识别（根据类型显示其中一个）
  if (hasBiometric) {
    if (biometricType === 'faceId') {
      methods.push({
        id: 'faceId',
        icon: <ScanFace size={ICON_SIZE.xl} />,
        label: 'Face ID',
        onClick: () => setLoginMethod('faceId'),
      });
    }
    if (biometricType === 'touchId') {
      methods.push({
        id: 'touchId',
        icon: <Fingerprint size={ICON_SIZE.xl} />,
        label: 'Touch ID',
        onClick: () => setLoginMethod('touchId'),
      });
    }
    if (biometricType === 'windowsHello') {
      methods.push({
        id: 'windowsHello',
        icon: <ShieldCheck size={ICON_SIZE.xl} />,
        label: 'Windows Hello',
        onClick: () => setLoginMethod('windowsHello'),
      });
    }
  }
  // 5. PIN 码
  if (pinAvailable) {
    methods.push({
      id: 'pin',
      icon: <Grip size={ICON_SIZE.xl} />,
      label: t('auth:pin_method', { defaultValue: 'PIN 码' }),
      onClick: () => {
        setLoginMethod('pin');
        setPinError(null);
      },
    });
  }

  return {
    // 展示层需要
    loginMethod,
    bioLoading,
    biometricLabel,
    handleBiometric,
    pinUnlocking,
    pinError,
    pinInputKey,
    handlePinComplete,
    password,
    handlePasswordChange,
    inputError: errorMessage ?? error,
    loading,
    handleConfirm,
    handleClose,
    methods,
    hoveredIcon,
    committedIcon,
    handleIconEnter,
    handleIconLeave,
    handleIconClick,
  };
}
