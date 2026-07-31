import { useTranslation } from 'react-i18next';
import { ScanFace, Fingerprint, ShieldCheck, AlertTriangle } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import styles from './LoginPage.module.css';

/** 生物识别类型的可读标签映射 */
const BIOMETRIC_LABEL: Record<string, string> = {
  faceId: 'Face ID',
  touchId: 'Touch ID',
  windowsHello: 'Windows Hello',
};

const BIOMETRIC_ICON: Record<string, typeof ScanFace> = {
  faceId: ScanFace,
  touchId: Fingerprint,
  windowsHello: ShieldCheck,
};

interface LoginBiometricViewProps {
  loginMethod: 'faceId' | 'touchId' | 'windowsHello';
  bioLoading: boolean;
  /** 系统生物识别因失败次数过多被临时锁定（Android） */
  bioLockout: boolean;
  onUnlock: () => void;
}

/** 生物识别解锁面板（登录页最高优先级方式）。 */
export function LoginBiometricView({
  loginMethod,
  bioLoading,
  bioLockout,
  onUnlock,
}: LoginBiometricViewProps) {
  const { t } = useTranslation(['auth', 'common', 'settings']);
  const Icon = BIOMETRIC_ICON[loginMethod] || ScanFace;

  return (
    <div
      style={{
        minHeight: 152,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        marginBottom: 16,
      }}
    >
      {/* 系统生物识别临时锁定（失败次数过多）警告条 — 与设置页 BiometricSection 风格一致 */}
      {bioLockout && (
        <div
          role="alert"
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            gap: 8,
            padding: 10,
            borderRadius: 8,
            marginBottom: 10,
            background: 'rgba(212, 133, 10, 0.10)',
            border: '1px solid rgba(212, 133, 10, 0.25)',
            color: '#D4850A',
            fontSize: 'var(--text-caption)',
            lineHeight: 1.4,
          }}
        >
          <AlertTriangle
            size={ICON_SIZE.md}
            style={{ flexShrink: 0, marginTop: 1 }}
          />
          <span>{t('settings:biometric_lockout_desc')}</span>
        </div>
      )}
      <button
        onClick={onUnlock}
        disabled={bioLoading}
        className={styles.loginFloatButton}
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 12,
          padding: '20px 24px',
          borderRadius: 14,
          border: '1px solid var(--border-subtle)',
          background: bioLoading ? 'var(--bg-toolbar)' : 'transparent',
          cursor: bioLoading ? 'wait' : 'pointer',
          width: '100%',
        }}
      >
        <Icon
          size={ICON_SIZE['4xl']}
          color="var(--accent-primary)"
          style={{ opacity: bioLoading ? 0.5 : 1 }}
        />
        <span
          style={{
            fontSize: 'var(--text-card-title)',
            fontWeight: 500,
            color: 'var(--text-primary)',
          }}
        >
          {bioLoading
            ? t('auth:bio_verifying')
            : t('auth:bio_unlock_reason', {
                type: BIOMETRIC_LABEL[loginMethod] || loginMethod,
              })}
        </span>
      </button>
    </div>
  );
}
