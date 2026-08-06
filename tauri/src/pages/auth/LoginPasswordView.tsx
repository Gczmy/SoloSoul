import { useTranslation } from 'react-i18next';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import type { FormEvent } from 'react';

interface LoginPasswordViewProps {
  password: string;
  onPasswordChange: (v: string) => void;
  isLoading: boolean;
  bioError: string | null;
  submitError: string | null;
  pinError: string | null;
  /** 主密码输入框行内错误（空密码 / 后端密码类错误），已 i18n。 */
  passwordFieldError: string | null;
  /** 密码错误抖动重触发计数（同串错误重复抖动）。 */
  passwordErrorTick: number;
  passwordHint: string | null;
  onSubmit: (e?: FormEvent) => void;
  onFocus?: () => void;
}

/** 密码登录表单（登录页最低优先级；初始化或缓存回退时也显示，避免白屏）。 */
export function LoginPasswordView({
  password,
  onPasswordChange,
  isLoading,
  bioError,
  submitError,
  pinError,
  passwordFieldError,
  passwordErrorTick,
  passwordHint,
  onSubmit,
  onFocus,
}: LoginPasswordViewProps) {
  const { t } = useTranslation(['auth', 'common']);

  // 主密码错误已改由输入框行内展示（passwordFieldError，红边 + 抖动 + 行内红字），
  // 独立错误区仅保留非密码类错误：PIN > 提交 > 生物识别
  const displayError = pinError || submitError || bioError || '';

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
          // Windows WebView2 自动填充会显示历史密码明文：SoloSoul 主密码非网站密码，
          // 密码管理器不应接管，禁用自动填充（current-password 会触发填充）。
          autoComplete="off"
          onEnter={onSubmit}
          onFocus={onFocus}
          error={passwordFieldError}
          errorTick={passwordErrorTick}
          reserveErrorSpace
        />
        {/* 非密码错误区：minHeight 固定占位，错误出现/消失不改变表单高度（防闪烁） */}
        <div style={{ color: '#dc2626', fontSize: 'var(--text-body-sm)', minHeight: 20 }}>
          {displayError}
        </div>
        <button
          type="submit"
          disabled={isLoading}
          className="interactive-toolbar"
          style={{
            width: '100%',
            padding: '8px 16px',
            borderRadius: 8,
            borderWidth: 1,
            borderStyle: 'solid',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            fontFamily: 'inherit',
            cursor: isLoading ? 'default' : 'pointer',
            opacity: isLoading ? 0.6 : 1,
            // 加载态保留 accent 底色（disabled 时无 hover，类仅管 idle 态）
            background: isLoading
              ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
              : undefined,
            color: isLoading ? 'var(--accent-primary)' : undefined,
          }}
        >
          {isLoading ? t('common:loading', { defaultValue: '...' }) : t('auth:login_button')}
        </button>
      </form>
    </div>
  );
}
