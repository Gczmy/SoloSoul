import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Fingerprint, KeyRound, ScanFace, ShieldCheck, Grip } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { supportsHover } from '@/lib/platform';
import type { LoginMethodOption } from '@/pages/auth/LoginIconBar';

import type { LoginMethod } from './usePasswordVerification';

export interface UsePasswordVerificationIconBarOptions {
  /** 是否启用生物识别按钮（biometricType + onBiometric 均提供）。 */
  hasBiometric: boolean;
  /** Biometric type name (e.g. "Touch ID", "Face ID") */
  biometricType?: string;
  /** PIN 是否可用（flows hook 探测结果）。 */
  pinAvailable: boolean;
  /** 选择解锁方式（父 hook 负责 setLoginMethod + 清对应错误）。 */
  onSelectMethod: (method: LoginMethod) => void;
}

/**
 * 统一密码验证对话框的底部图标栏状态（W001-⑤ 拆分：数据 hook）。
 * 可用解锁方式列表构建（主密码 → Face ID → Touch ID → Windows Hello → PIN）
 * 与两阶段悬停（边框/颜色立即高亮 + 文字/展开延迟 300ms）收敛于此；
 * PasswordVerificationDialog 仅透传给 LoginIconBar 展示。
 */
export function usePasswordVerificationIconBar({
  hasBiometric,
  biometricType,
  pinAvailable,
  onSelectMethod,
}: UsePasswordVerificationIconBarOptions) {
  const { t } = useTranslation(['auth', 'common', 'settings']);
  const [hoveredIcon, setHoveredIcon] = useState<string | null>(null);
  const [committedIcon, setCommittedIcon] = useState<string | null>(null);
  const commitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 卸载时清理悬停延迟定时器
  useEffect(() => {
    return () => {
      if (commitTimerRef.current) clearTimeout(commitTimerRef.current);
    };
  }, []);

  // ==== 构建可用解锁方式列表 ====
  // 顺序：主密码 → Face ID → Touch ID → Windows Hello → PIN
  const methods: LoginMethodOption[] = [];
  // 1. 主密码（始终可用）
  methods.push({
    id: 'password',
    icon: <KeyRound size={ICON_SIZE.xl} />,
    label: t('auth:password_method', { defaultValue: '主密码' }),
    onClick: () => onSelectMethod('password'),
  });

  // 2–4. 生物识别（根据类型显示其中一个）
  if (hasBiometric) {
    if (biometricType === 'faceId') {
      methods.push({
        id: 'faceId',
        icon: <ScanFace size={ICON_SIZE.xl} />,
        label: 'Face ID',
        onClick: () => onSelectMethod('faceId'),
      });
    }
    if (biometricType === 'touchId') {
      methods.push({
        id: 'touchId',
        icon: <Fingerprint size={ICON_SIZE.xl} />,
        label: 'Touch ID',
        onClick: () => onSelectMethod('touchId'),
      });
    }
    if (biometricType === 'windowsHello') {
      methods.push({
        id: 'windowsHello',
        icon: <ShieldCheck size={ICON_SIZE.xl} />,
        label: 'Windows Hello',
        onClick: () => onSelectMethod('windowsHello'),
      });
    }
  }
  // 5. PIN 码
  if (pinAvailable) {
    methods.push({
      id: 'pin',
      icon: <Grip size={ICON_SIZE.xl} />,
      label: t('auth:pin_method', { defaultValue: 'PIN 码' }),
      onClick: () => onSelectMethod('pin'),
    });
  }

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

  return {
    methods,
    hoveredIcon,
    committedIcon,
    handleIconEnter,
    handleIconLeave,
    handleIconClick,
  };
}
