import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Fingerprint, KeyRound, ScanFace, ShieldCheck, Grip } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { supportsHover } from '@/lib/platform';

import type { LoginMethodOption } from './LoginIconBar';
import type { LoginMethod } from './useLoginPage';


export interface UseLoginIconBarOptions {
  bioAvailable: boolean;
  biometryTypeRaw: string;
  pinAvailable: boolean;
  /** 选择解锁方式（父 hook 负责 setLoginMethod + 清除对应错误） */
  onSelectMethod: (method: LoginMethod) => void;
}

/**
 * 登录页底部图标栏状态（W001-② 拆分：数据 hook）。
 * 可用解锁方式列表构建（主密码 → Face ID → Touch ID → Windows Hello → PIN）
 * 与两阶段悬停（边框/颜色立即高亮 + 文字/展开延迟 300ms）收敛于此；
 * LoginPage 仅透传给 LoginIconBar 展示。
 */
export function useLoginIconBar({
  bioAvailable,
  biometryTypeRaw,
  pinAvailable,
  onSelectMethod,
}: UseLoginIconBarOptions) {
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
  const iconMethods: LoginMethodOption[] = [];
  // 1. 主密码（始终可用）
  iconMethods.push({
    id: 'password',
    icon: <KeyRound size={ICON_SIZE.xl} />,
    label: t('auth:password_method', { defaultValue: '主密码' }),
    onClick: () => onSelectMethod('password'),
  });
  // 2. Face ID
  if (bioAvailable && biometryTypeRaw === 'faceId') {
    iconMethods.push({
      id: 'faceId',
      icon: <ScanFace size={ICON_SIZE.xl} />,
      label: 'Face ID',
      onClick: () => onSelectMethod('faceId'),
    });
  }
  // 3. Touch ID
  if (bioAvailable && biometryTypeRaw === 'touchId') {
    iconMethods.push({
      id: 'touchId',
      icon: <Fingerprint size={ICON_SIZE.xl} />,
      label: 'Touch ID',
      onClick: () => onSelectMethod('touchId'),
    });
  }
  // 4. Windows Hello
  if (bioAvailable && biometryTypeRaw === 'windowsHello') {
    iconMethods.push({
      id: 'windowsHello',
      icon: <ShieldCheck size={ICON_SIZE.xl} />,
      label: 'Windows Hello',
      onClick: () => onSelectMethod('windowsHello'),
    });
  }
  // 5. PIN 码
  if (pinAvailable) {
    iconMethods.push({
      id: 'pin',
      icon: <Grip size={ICON_SIZE.xl} />,
      label: t('auth:pin_method', { defaultValue: 'PIN 码' }),
      onClick: () => onSelectMethod('pin'),
    });
  }

  // 两阶段悬停：边框/颜色立即高亮，文字/展开延迟 300ms 后触发
  const handleIconEnter = (id: string) => {
    // 触屏设备不触发悬停展开（Android WebView hover 会粘住）
    if (!supportsHover()) return;
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
    iconMethods,
    hoveredIcon,
    committedIcon,
    handleIconEnter,
    handleIconLeave,
    handleIconClick,
  };
}
