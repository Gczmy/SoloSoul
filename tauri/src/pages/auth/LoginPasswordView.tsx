import { useTranslation } from 'react-i18next';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import type { FormEvent } from 'react';

interface LoginPasswordViewProps {
  password: string;
  onPasswordChange: (v: string) => void;
  isLoading: boolean;
  error: string | null;
  bioError: string | null;
  submitError: string | null;
  pinError: string | null;
  passwordHint: string | null;
  onSubmit: (e?: FormEvent) => void;
  onFocus?: () => void;
}

/** 密码登录表单（登录页最低优先级；初始化或缓存回退时也显示，避免白屏）。 */
export function LoginPasswordView({
  password,
  onPasswordChange,
  isLoading,
  error,
  bioError,
  submitError,
  pinError,
  passwordHint,
  onSubmit,
  onFocus,
}: LoginPasswordViewProps) {
  const { t } = useTranslation(['auth', 'common']);

  // 错误优先级与原件一致：PIN > 提交 > 生物识别 > 主密码（仅主密码需要错误文案转换）
  const displayError =
    pinError ||
    submitError ||
    bioError ||
    (error
      ? error.toLowerCase().includes('8 characters') || error.toLowerCase().includes('至少')
        ? t('auth:password_too_short')
        : error.toLowerCase().includes('password') || error.toLowerCase().includes('invalid')
          ? t('auth:incorrect_password')
          : error.toLowerCase().includes('required')
            ? t('auth:password_required')
            : error
      : '');

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
      <form onSubmit={onSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
        <SecurePasswordInput
          value={password}
          onChange={onPasswordChange}
          placeholder={t('common:password_placeholder')}
          hint={passwordHint}
          autoComplete="current-password"
          onEnter={onSubmit}
          onFocus={onFocus}
        />
        {displayError && (
          <div style={{ color: '#dc2626', fontSize: 'var(--text-body-sm)' }}>{displayError}</div>
        )}
        <button
          type="submit"
          disabled={isLoading}
          style={{
            width: '100%',
            padding: '8px 16px',
            borderRadius: 8,
            border: '1px solid var(--border-subtle)',
            background: isLoading
              ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
              : 'var(--bg-toolbar)',
            color: isLoading ? 'var(--accent-primary)' : 'var(--text-primary)',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            fontFamily: 'inherit',
            cursor: isLoading ? 'default' : 'pointer',
            opacity: isLoading ? 0.6 : 1,
            transition: 'all 0.15s ease',
          }}
          onMouseEnter={(e) => {
            if (!isLoading) {
              e.currentTarget.style.background =
                'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              e.currentTarget.style.borderColor = 'var(--accent-primary)';
              e.currentTarget.style.color = 'var(--accent-primary)';
            }
          }}
          onMouseLeave={(e) => {
            if (!isLoading) {
              e.currentTarget.style.background = 'var(--bg-toolbar)';
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
              e.currentTarget.style.color = 'var(--text-primary)';
            }
          }}
        >
          {isLoading ? t('common:loading', { defaultValue: '...' }) : t('auth:login_button')}
        </button>
      </form>
    </div>
  );
}
