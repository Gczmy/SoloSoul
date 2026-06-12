import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import { Sparkles, PlusSquare, LayoutTemplate, ShieldCheck, CheckCircle } from 'lucide-react';

interface OnboardingDialogProps {
  onComplete: () => void;
  onSkip: () => void;
}

const steps = [
  { key: 'welcome', icon: Sparkles },
  { key: 'create_object', icon: PlusSquare },
  { key: 'templates', icon: LayoutTemplate },
  { key: 'security', icon: ShieldCheck },
  { key: 'finish', icon: CheckCircle },
] as const;

export function OnboardingDialog({ onComplete, onSkip }: OnboardingDialogProps) {
  const { t } = useTranslation('common');
  const [step, setStep] = useState(0);

  const current = steps[step];
  const Icon = current.icon;
  const isLast = step === steps.length - 1;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 10000,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.4)',
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
            width: 64, height: 64, borderRadius: 16,
            background: 'linear-gradient(135deg, var(--accent-primary), var(--accent-warm))',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            margin: '0 auto 20px',
          }}
        >
          <Icon size={30} color="white" />
        </div>

        <h2 style={{ fontSize: 20, fontWeight: 700, margin: '0 0 10px', color: 'var(--text-primary)' }}>
          {t(`onboarding_${current.key}_title`)}
        </h2>
        <p style={{ fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.6, margin: '0 0 28px', minHeight: 70 }}>
          {t(`onboarding_${current.key}_desc`)}
        </p>

        <div style={{ display: 'flex', justifyContent: 'center', gap: 6, marginBottom: 28 }}>
          {steps.map((_, i) => (
            <span
              key={i}
              style={{
                width: 8, height: 8, borderRadius: '50%',
                background: i === step ? 'var(--accent-primary)' : 'var(--border-subtle)',
              }}
            />
          ))}
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <button
            onClick={onSkip}
            style={{
              padding: '8px 12px', borderRadius: 8, border: 'none',
              background: 'transparent', color: 'var(--text-tertiary)',
              fontSize: 13, cursor: 'pointer',
            }}
          >
            {t('onboarding_skip')}
          </button>
          <div style={{ display: 'flex', gap: 8 }}>
            {step > 0 && (
              <Button variant="secondary" onClick={() => setStep((s) => s - 1)}>
                {t('onboarding_back')}
              </Button>
            )}
            <Button onClick={() => {
              if (isLast) {
                onComplete();
              } else {
                setStep((s) => s + 1);
              }
            }}>
              {isLast ? t('onboarding_done') : t('onboarding_next')}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
