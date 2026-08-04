import { useTranslation } from 'react-i18next';
import { Grip } from 'lucide-react';
import { PinInput, type PinInputHandle } from '@/components/forms/PinInput';
import { IndeterminateProgressBar } from '@/components/ui/IndeterminateProgressBar';
import { ICON_SIZE } from '@/lib/constants';
import type { RefObject } from 'react';

interface LoginPinViewProps {
  pinUnlocking: boolean;
  pinError: string | null;
  pinInputKey: number;
  pinInputRef: RefObject<PinInputHandle | null>;
  onPinComplete: (pin: string) => void;
}

/** PIN 解锁面板（登录页）。 */
export function LoginPinView({
  pinUnlocking,
  pinError,
  pinInputKey,
  pinInputRef,
  onPinComplete,
}: LoginPinViewProps) {
  const { t } = useTranslation(['auth', 'common']);

  return (
    <div
      style={{
        minHeight: 152,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        marginBottom: 16,
      }}
      onClick={() => pinInputRef.current?.focus()}
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
        {/* 验证中动画 — 移至 PIN 码框下方（创建新账户按钮上方），不再遮挡输入框 */}
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
