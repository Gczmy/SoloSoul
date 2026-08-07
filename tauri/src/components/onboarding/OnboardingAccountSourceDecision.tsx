import { useTranslation } from 'react-i18next';
import { Smartphone, UserPlus } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

interface OnboardingAccountSourceDecisionProps {
  onRecovery: () => void;
  onCreateNew: () => void;
  onBack: () => void;
  /** 「返回」按钮文案（已翻译）；缺省用 onboarding_account_source_back（「返回」）。
   *  重开浮层场景在本地已有账户时传「返回登录」，首次启动（无账户）保持「返回」。 */
  backLabel?: string;
}

/** 完成引导后（本地无账户时）询问账户来源：从其它设备恢复 or 创建新账户。 */
export function OnboardingAccountSourceDecision({
  onRecovery,
  onCreateNew,
  onBack,
  backLabel,
}: OnboardingAccountSourceDecisionProps) {
  const { t } = useTranslation('common');

  const actionCard = (onClick: () => void, accent: 'primary' | 'warm') => ({
    style: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 8,
      padding: '14px 16px',
      borderRadius: 12,
      borderWidth: 1,
      borderStyle: 'solid',
      cursor: 'pointer',
      fontFamily: 'inherit',
      fontWeight: 500,
      fontSize: 'var(--text-body-sm)',
    },
    className: accent === 'primary' ? 'interactive-toolbar' : 'interactive-toolbar-warm',
    onClick,
  });

  return (
    <div
      style={{
        position: 'absolute',
        inset: 0,
        zIndex: 'calc(var(--z-onboarding) + 1)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg-overlay)',
        backdropFilter: 'blur(4px)',
        padding: 16,
        borderRadius: 18,
      }}
    >
      <div
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 18,
          padding: '32px 36px',
          maxWidth: 440,
          width: '100%',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
          textAlign: 'center',
        }}
      >
        <div
          style={{
            width: 64,
            height: 64,
            borderRadius: 16,
            background: 'linear-gradient(135deg, var(--accent-primary), var(--accent-warm))',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            margin: '0 auto 20px',
          }}
        >
          <Smartphone size={ICON_SIZE['3xl']} color="white" />
        </div>

        <h2
          style={{
            fontSize: 'var(--text-page-title)',
            fontWeight: 700,
            margin: '0 0 10px',
            color: 'var(--text-primary)',
          }}
        >
          {t('onboarding_account_source_title')}
        </h2>
        <p
          style={{
            fontSize: 'var(--text-body)',
            color: 'var(--text-secondary)',
            lineHeight: 1.6,
            margin: '0 0 24px',
          }}
        >
          {t('onboarding_account_source_desc')}
        </p>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 12, marginBottom: 16 }}>
          <button type="button" {...actionCard(onRecovery, 'primary')}>
            <Smartphone size={ICON_SIZE.md} />
            {t('onboarding_account_source_sync')}
          </button>
          <button type="button" {...actionCard(onCreateNew, 'warm')}>
            <UserPlus size={ICON_SIZE.md} />
            {t('onboarding_account_source_create')}
          </button>
        </div>

        <button
          type="button"
          onClick={onBack}
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-tertiary)',
            background: 'transparent',
            border: 'none',
            padding: '6px 12px',
            cursor: 'pointer',
            fontFamily: 'inherit',
            fontWeight: 500,
          }}
        >
          {backLabel ?? t('onboarding_account_source_back')}
        </button>
      </div>
    </div>
  );
}
