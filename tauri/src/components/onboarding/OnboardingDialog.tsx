import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';

import {
  Sparkles,
  PlusSquare,
  LayoutTemplate,
  ShieldCheck,
  CheckCircle,
  Folder,
  AlertCircle,
  RefreshCw,
} from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { getPlatform } from '@/lib/platform';
import { pickVaultDirectory, setVaultDirectory } from '@/lib/vaultDirectory';
import { relaunch } from '@tauri-apps/plugin-process';

interface OnboardingDialogProps {
  onComplete: () => void;
  onSkip: () => void;
}

const baseSteps = [
  { key: 'welcome', icon: Sparkles },
  { key: 'vault_directory', icon: Folder },
  { key: 'create_object', icon: PlusSquare },
  { key: 'templates', icon: LayoutTemplate },
  { key: 'security', icon: ShieldCheck },
  { key: 'finish', icon: CheckCircle },
] as const;

export function OnboardingDialog({ onComplete, onSkip: _onSkip }: OnboardingDialogProps) {
  const { t } = useTranslation('common');
  const [step, setStep] = useState(0);
  const [platformName, setPlatformName] = useState<string>('');
  const [vaultDirChoice, setVaultDirChoice] = useState<'local' | 'saf' | null>(null);
  const [vaultDirActing, setVaultDirActing] = useState(false);
  const [vaultDirSuccess, setVaultDirSuccess] = useState(false);
  const [vaultDirError, setVaultDirError] = useState<string | null>(null);
  const [vaultNeedsRestart, setVaultNeedsRestart] = useState(false);

  useEffect(() => {
    getPlatform().then((p) => {
      setPlatformName(p);
    });
  }, []);

  const isAndroid = platformName === 'android';

  // Wait for platform to load before determining steps, avoiding race where
  // the steps array changes length mid-render (vault_directory step only shown on Android).
  const steps =
    platformName === ''
      ? baseSteps
      : isAndroid
        ? baseSteps
        : baseSteps.filter((s) => s.key !== 'vault_directory');

  const current = steps[step];
  const Icon = current?.icon || Sparkles;
  const isLast = step >= steps.length - 1;

  // 每次进入 vault_directory 步骤时重置状态，让用户可以重新选择
  useEffect(() => {
    if (current?.key === 'vault_directory') {
      setVaultDirChoice(null);
      setVaultDirSuccess(false);
      setVaultDirError(null);
      setVaultNeedsRestart(false);
      setVaultDirActing(false);
    }
  }, [step]);

  const handleVaultDirPick = useCallback(async () => {
    const { pause, resume } = await import('@/stores/autoLockPauseStore').then(
      (m) => m.useAutoLockPauseStore.getState(),
    );
    pause();
    try {
      setVaultDirActing(true);
      setVaultDirError(null);
      const uri = await pickVaultDirectory();
      if (!uri) {
        // User cancelled — return to choice
        setVaultDirChoice(null);
        return;
      }
      const result = await setVaultDirectory(uri);
      if (result.success) {
        setVaultDirSuccess(true);
        setVaultNeedsRestart(result.needsRestart);
      } else {
        setVaultDirError(result.message || t('onboarding_vault_dir_set_failed'));
      }
    } catch (e) {
      setVaultDirError(String(e));
      setVaultDirChoice(null);
    } finally {
      resume();
      setVaultDirActing(false);
    }
  }, [t]);

  const handleFinishOnboarding = () => {
    if (vaultNeedsRestart) {
      // Mark onboarding as seen before restart so it doesn't show again
      onComplete();
      relaunch().catch(() => {
        // Fallback: just stay on current page
      });
    } else {
      onComplete();
    }
  };

  // Show only the vault directory step when we need to display it
  if (current.key === 'vault_directory') {
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
            <Folder size={ICON_SIZE['3xl']} color="white" />
          </div>

          <h2
            style={{
              fontSize: 'var(--text-page-title)',
              fontWeight: 700,
              margin: '0 0 10px',
              color: 'var(--text-primary)',
            }}
          >
            {t('onboarding_vault_dir_title')}
          </h2>

          <p
            style={{
              fontSize: 'var(--text-body)',
              color: 'var(--text-secondary)',
              lineHeight: 1.6,
              margin: '0 0 24px',
            }}
          >
            {t('onboarding_vault_dir_desc')}
          </p>

          {vaultNeedsRestart ? (
            <div
              style={{
                padding: 16,
                borderRadius: 12,
                background: 'color-mix(in srgb, var(--accent-primary) 8%, var(--bg-elevated))',
                border: '1px solid color-mix(in srgb, var(--accent-primary) 30%, transparent)',
                marginBottom: 24,
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
                <AlertCircle size={20} style={{ color: 'var(--accent-primary)', flexShrink: 0 }} />
                <div style={{ textAlign: 'left' }}>
                  <div
                    style={{
                      fontWeight: 600,
                      marginBottom: 4,
                      color: 'var(--text-primary)',
                    }}
                  >
                    {t('onboarding_vault_dir_restart_title')}
                  </div>
                  <div
                    style={{
                      fontSize: 'var(--text-body-sm)',
                      color: 'var(--text-secondary)',
                    }}
                  >
                    {t('onboarding_vault_dir_restart_desc')}
                  </div>
                </div>
              </div>
              <button
                type="button"
                onClick={handleFinishOnboarding}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 6,
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: '1px solid var(--accent-primary)',
                  background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
                  color: 'var(--accent-primary)',
                  fontSize: 'var(--text-body-sm)',
                  fontWeight: 500,
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                }}
              >
                <RefreshCw size={14} />
                {t('onboarding_vault_dir_restart_btn')}
              </button>
            </div>
          ) : vaultDirSuccess ? (
            <div
              style={{
                padding: 14,
                borderRadius: 12,
                background: 'color-mix(in srgb, var(--accent-primary) 8%, var(--bg-elevated))',
                marginBottom: 24,
                color: 'var(--accent-primary)',
                fontWeight: 500,
                fontSize: 'var(--text-body)',
              }}
            >
              {t('onboarding_vault_dir_success')}
            </div>
          ) : vaultDirChoice === null ? (
            <>
              {/* Choice: Local vs SAF */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 24 }}>
                <button
                  type="button"
                  onClick={() => {
                    setVaultDirChoice('local');
                    setStep((s) => s + 1);
                  }}
                  style={{
                    padding: '14px 16px',
                    borderRadius: 12,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    cursor: 'pointer',
                    textAlign: 'left',
                    fontFamily: 'inherit',
                    transition: 'all 0.15s ease',
                  }}
                  className="interactive-toolbar"
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 8%, transparent)';
                    e.currentTarget.style.borderColor =
                      'color-mix(in srgb, var(--accent-primary) 40%, var(--border-subtle))';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  }}
                >
                  <div style={{ fontWeight: 600, marginBottom: 4, color: 'var(--text-primary)' }}>
                    {t('onboarding_vault_dir_local_title')}
                  </div>
                  <div
                    style={{
                      fontSize: 'var(--text-caption)',
                      color: 'var(--text-tertiary)',
                      lineHeight: 1.4,
                    }}
                  >
                    {t('onboarding_vault_dir_local_desc')}
                  </div>
                </button>

                <button
                  type="button"
                  onClick={handleVaultDirPick}
                  disabled={vaultDirActing}
                  style={{
                    padding: '14px 16px',
                    borderRadius: 12,
                    border: `1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)`,
                    background:
                      'color-mix(in srgb, var(--accent-primary) 6%, var(--bg-toolbar))',
                    cursor: vaultDirActing ? 'wait' : 'pointer',
                    textAlign: 'left',
                    fontFamily: 'inherit',
                    transition: 'all 0.15s ease',
                    opacity: vaultDirActing ? 0.6 : 1,
                  }}
                  onMouseEnter={(e) => {
                    if (!vaultDirActing) {
                      e.currentTarget.style.background =
                        'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (!vaultDirActing) {
                      e.currentTarget.style.background =
                        'color-mix(in srgb, var(--accent-primary) 6%, var(--bg-toolbar))';
                    }
                  }}
                >
                  <div
                    style={{
                      fontWeight: 600,
                      marginBottom: 4,
                      color: 'var(--accent-primary)',
                    }}
                  >
                    {vaultDirActing
                      ? t('common:loading')
                      : t('onboarding_vault_dir_saf_title')}
                  </div>
                  <div
                    style={{
                      fontSize: 'var(--text-caption)',
                      color: 'var(--text-secondary)',
                      lineHeight: 1.4,
                    }}
                  >
                    {t('onboarding_vault_dir_saf_desc')}
                  </div>
                </button>
              </div>

              {vaultDirError && (
                <div
                  style={{
                    padding: 8,
                    borderRadius: 8,
                    background: 'rgba(220, 38, 38, 0.08)',
                    color: '#dc2626',
                    fontSize: 'var(--text-body-sm)',
                    marginBottom: 16,
                  }}
                >
                  {vaultDirError}
                </div>
              )}
            </>
          ) : (
            // vaultDirChoice === 'saf' and waiting for result (should not reach here normally)
            <div
              style={{
                padding: 14,
                borderRadius: 12,
                background: 'var(--bg-toolbar)',
                marginBottom: 24,
                color: 'var(--text-secondary)',
                fontSize: 'var(--text-body)',
              }}
            >
              {t('common:loading')}
            </div>
          )}

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
            {/* Hide skip button during vault directory step to prevent easy dismiss */}
            <div />
            <div style={{ display: 'flex', gap: 8 }}>
              {step > 0 && (
                <button
                  type="button"
                  onClick={() => {
                    if (vaultDirChoice === 'saf') {
                      setVaultDirChoice(null);
                      setVaultDirError(null);
                    }
                    setStep((s) => s - 1);
                  }}
                  style={{
                    fontSize: 'var(--text-caption)',
                    padding: '6px 12px',
                    borderRadius: 6,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    color: 'var(--text-primary)',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                    fontWeight: 500,
                    transition: 'all 0.15s ease',
                  }}
                  className="interactive-toolbar"
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    e.currentTarget.style.color = 'var(--text-primary)';
                  }}
                >
                  {t('onboarding_back')}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Regular step rendering
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
          {t(`onboarding_${current.key}_title`)}
        </h2>
        <p
          style={{
            fontSize: 'var(--text-body)',
            color: 'var(--text-secondary)',
            lineHeight: 1.6,
            margin: '0 0 28px',
            minHeight: 70,
          }}
        >
          {t(`onboarding_${current.key}_desc`)}
        </p>

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
          <div />
          <div style={{ display: 'flex', gap: 8 }}>
            {step > 0 && (
              <button
                type="button"
                onClick={() => setStep((s) => s - 1)}
                style={{
                  fontSize: 'var(--text-caption)',
                  padding: '6px 12px',
                  borderRadius: 6,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  color: 'var(--text-primary)',
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                  fontWeight: 500,
                  transition: 'all 0.15s ease',
                }}
                className="interactive-toolbar"
                onMouseEnter={(e) => {
                  e.currentTarget.style.background =
                    'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.color = 'var(--text-primary)';
                }}
              >
                {t('onboarding_back')}
              </button>
            )}
            <button
              type="button"
              onClick={() => {
                if (isLast) {
                  onComplete();
                } else {
                  setStep((s) => s + 1);
                }
              }}
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background =
                  'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--border-subtle)';
                e.currentTarget.style.color = 'var(--text-primary)';
              }}
            >
              {isLast ? t('onboarding_done') : t('onboarding_next')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
