import { useTranslation } from 'react-i18next';
import type { LucideIcon } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import type { CSSProperties, ReactNode } from 'react';
import type { OnboardingStepDef } from '@/hooks/useOnboarding';

interface OnboardingFrameProps {
  icon: LucideIcon;
  title: string;
  /** 描述文案；为 null 时不渲染（由 children 自行提供内容） */
  desc?: string | null;
  /** desc 额外样式（默认常规步骤：minHeight 70 防布局跳动；vault_directory 步骤可覆盖） */
  descStyle?: CSSProperties;
  steps: readonly OnboardingStepDef[];
  step: number;
  /** 底部左侧（隐藏 skip 按钮时的占位） */
  footerLeft?: ReactNode;
  footerRight: ReactNode;
  children?: ReactNode;
}

/** Onboarding 步骤的共享弹窗骨架：图标 + 标题 + 描述 + 内容 + 步骤点 + 底部导航。 */
export function OnboardingFrame({
  icon: Icon,
  title,
  desc,
  descStyle,
  steps,
  step,
  footerLeft,
  footerRight,
  children,
}: OnboardingFrameProps) {
  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-onboarding)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg-overlay)',
        backdropFilter: 'blur(4px)',
      }}
    >
      <div
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 18,
          padding: '32px 36px',
          maxWidth: 440,
          width: '90%',
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
          <Icon size={ICON_SIZE['3xl']} color="white" />
        </div>

        <h2
          style={{
            fontSize: 'var(--text-page-title)',
            fontWeight: 700,
            margin: '0 0 10px',
            color: 'var(--text-primary)',
          }}
        >
          {title}
        </h2>
        {desc && (
          <p
            style={{
              fontSize: 'var(--text-body)',
              color: 'var(--text-secondary)',
              lineHeight: 1.6,
              margin: '0 0 28px',
              minHeight: 70,
              ...descStyle,
            }}
          >
            {desc}
          </p>
        )}

        {children}

        {/* Step dots */}
        <div style={{ display: 'flex', justifyContent: 'center', gap: 6, marginBottom: 28 }}>
          {steps.map((_, i) => (
            <span
              key={i}
              style={{
                width: 8,
                height: 8,
                borderRadius: '50%',
                background: i === step ? 'var(--accent-primary)' : 'var(--border-subtle)',
              }}
            />
          ))}
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>{footerLeft}</div>
          <div style={{ display: 'flex', gap: 8 }}>{footerRight}</div>
        </div>
      </div>
    </div>
  );
}

/** 底部「返回」按钮。 */
export function OnboardingBackButton({ onClick }: { onClick: () => void }) {
  const { t } = useTranslation('common');
  return (
    <button
      type="button"
      onClick={onClick}
      style={{
        fontSize: 'var(--text-caption)',
        padding: '6px 12px',
        borderRadius: 6,
        borderWidth: 1,
        borderStyle: 'solid',
        cursor: 'pointer',
        fontFamily: 'inherit',
        fontWeight: 500,
      }}
      className="interactive-toolbar"
    >
      {t('onboarding_back')}
    </button>
  );
}

/** 底部「下一步/完成」按钮。 */
export function OnboardingNextButton({
  label,
  onClick,
}: {
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      style={{
        fontSize: 'var(--text-caption)',
        padding: '6px 12px',
        borderRadius: 6,
        borderWidth: 1,
        borderStyle: 'solid',
        cursor: 'pointer',
        fontFamily: 'inherit',
        fontWeight: 500,
      }}
      className="interactive-toolbar"
    >
      {label}
    </button>
  );
}
