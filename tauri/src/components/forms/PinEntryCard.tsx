import { useTranslation } from 'react-i18next';
import { Grip } from 'lucide-react';
import { PinInput, type PinInputHandle } from '@/components/forms/PinInput';
import { IndeterminateProgressBar } from '@/components/ui/IndeterminateProgressBar';
import { ICON_SIZE } from '@/lib/constants';
import type { RefObject } from 'react';

interface PinEntryCardProps {
  /** PIN 解锁进行中：禁用输入 + 显示进度条。 */
  pinUnlocking: boolean;
  /** 错误文案（null 无错误）。 */
  pinError: string | null;
  /** 强制重置输入框的 key（错误后置 +1 清空）。 */
  pinInputKey: number;
  /** 可选 ref：登录页用于点击卡片聚焦输入框。 */
  pinInputRef?: RefObject<PinInputHandle | null>;
  /** PIN 完成回调。 */
  onPinComplete: (pin: string) => void;
  /** 外层容器点击回调（如聚焦）。 */
  onCardClick?: () => void;
  /** 外层容器 marginBottom。 */
  marginBottom?: number;
}

/**
 * PIN 码输入卡片（P040：PasswordVerificationDialog 与 LoginPinView 的 PIN 卡片
 * 布局一致，收敛为共享组件）。
 */
export function PinEntryCard({
  pinUnlocking,
  pinError,
  pinInputKey,
  pinInputRef,
  onPinComplete,
  onCardClick,
  marginBottom = 8,
}: PinEntryCardProps) {
  const { t } = useTranslation(['auth']);

  return (
    <div
      style={{
        minHeight: 152,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        marginBottom,
      }}
      onClick={onCardClick}
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 12,
          padding: '16px 24px 20px',
          borderRadius: 14,
          border: '1px solid var(--border-subtle)',
          background: 'transparent',
          width: '100%',
        }}
      >
        <Grip size={ICON_SIZE['2xl']} color="var(--accent-primary)" />
        <span
          style={{
            fontSize: 'var(--text-card-title)',
            fontWeight: 500,
            color: 'var(--text-primary)',
          }}
        >
          {t('auth:pin_enter_title')}
        </span>
        <PinInput
          ref={pinInputRef}
          key={pinInputKey}
          length={6}
          onComplete={onPinComplete}
          disabled={pinUnlocking}
          error={!!pinError}
          verifying={pinUnlocking}
        />
        {/* 验证中动画 — 移至 PIN 码框下方，不再遮挡输入框 */}
        {pinUnlocking && (
          <div style={{ width: '100%', maxWidth: 240, marginTop: 2 }}>
            <IndeterminateProgressBar height={4} />
          </div>
        )}
        {pinError && (
          <div style={{ color: '#dc2626', fontSize: 'var(--text-body-sm)' }}>{pinError}</div>
        )}
      </div>
    </div>
  );
}
