import type { TFunction } from 'i18next';
import { Fingerprint, ScanFace, ShieldCheck } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

/**
 * PasswordVerificationDialog 的生物识别解锁卡片（P046 拆分：展示子组件）。
 * 纯展示：图标/文案随 loginMethod 变化，解锁动作由 onUnlock 转发。
 */
export function PasswordVerificationBiometricCard({
  loginMethod,
  bioLoading,
  biometricLabel,
  onUnlock,
  t,
}: {
  loginMethod: 'faceId' | 'touchId' | 'windowsHello';
  bioLoading: boolean;
  biometricLabel: string;
  onUnlock: () => void;
  t: TFunction;
}) {
  return (
    <div
      style={{
        minHeight: 152,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        marginBottom: 8,
      }}
    >
      <button
        onClick={onUnlock}
        disabled={bioLoading}
        className="interactive-card-lift"
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 12,
          padding: '20px 24px',
          borderRadius: 14,
          borderWidth: 1,
          borderStyle: 'solid',
          background: bioLoading ? 'var(--bg-toolbar)' : 'transparent',
          cursor: bioLoading ? 'wait' : 'pointer',
          width: '100%',
          fontFamily: 'inherit',
        }}
      >
        {loginMethod === 'faceId' && (
          <ScanFace
            size={ICON_SIZE['4xl']}
            color="var(--accent-primary)"
            style={{ opacity: bioLoading ? 0.5 : 1 }}
          />
        )}
        {loginMethod === 'touchId' && (
          <Fingerprint
            size={ICON_SIZE['4xl']}
            color="var(--accent-primary)"
            style={{ opacity: bioLoading ? 0.5 : 1 }}
          />
        )}
        {loginMethod === 'windowsHello' && (
          <ShieldCheck
            size={ICON_SIZE['4xl']}
            color="var(--accent-primary)"
            style={{ opacity: bioLoading ? 0.5 : 1 }}
          />
        )}
        <span
          style={{
            fontSize: 'var(--text-card-title)',
            fontWeight: 500,
            color: 'var(--text-primary)',
          }}
        >
          {bioLoading ? t('auth:bio_verifying') : t('auth:bio_unlock_reason', { type: biometricLabel })}
        </span>
      </button>
    </div>
  );
}
