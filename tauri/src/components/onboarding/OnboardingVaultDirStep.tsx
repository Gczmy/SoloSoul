import { useTranslation } from 'react-i18next';
import { Folder, LogIn, UserPlus } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { IndeterminateProgressBar } from '@/components/ui/IndeterminateProgressBar';
import type { AccountInfo } from '@/lib/ipc';
import type { SyncPhase } from '@/hooks/useOnboarding';
import type { OnboardingStepDef } from '@/hooks/useOnboarding';
import { OnboardingFrame, OnboardingBackButton, OnboardingNextButton } from '@/components/onboarding/OnboardingFrame';

interface OnboardingVaultDirStepProps {
  steps: readonly OnboardingStepDef[];
  step: number;
  vaultDirActing: boolean;
  vaultDirError: string | null;
  selectedSafUri: string | null;
  syncPhase: SyncPhase;
  syncFileName: string;
  syncFileCount: number;
  showAccountDecision: boolean;
  foundAccounts: AccountInfo[];
  foundAccountCount: number;
  onPickLocal: () => void;
  onPickSaf: () => void;
  onLoginExisting: () => void;
  onCreateNewAccount: () => void;
  onClearSafUri: () => void;
  onBack: () => void;
  onNext: () => void;
  onSetVaultDirError: (e: string | null) => void;
}

/** vault_directory 步骤（Android）：本地/SAF 目录选择、SAF 同步进度、已有账户决策。 */
export function OnboardingVaultDirStep({
  steps,
  step,
  vaultDirActing,
  vaultDirError,
  selectedSafUri,
  syncPhase,
  syncFileName,
  syncFileCount,
  showAccountDecision,
  foundAccounts,
  foundAccountCount,
  onPickLocal,
  onPickSaf,
  onLoginExisting,
  onCreateNewAccount,
  onClearSafUri,
  onBack,
  onNext,
  onSetVaultDirError,
}: OnboardingVaultDirStepProps) {
  const { t } = useTranslation('common');

  const actionCard = (onClick: () => void, accent: 'primary' | 'warm') => ({
    style: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 8,
      padding: '12px 16px',
      borderRadius: 10,
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
    <OnboardingFrame
      icon={Folder}
      title={t('onboarding_vault_dir_title')}
      desc={t('onboarding_vault_dir_desc')}
      descStyle={{ margin: '0 0 24px', minHeight: 0 }}
      steps={steps}
      step={step}
      footerLeft={<div />}
      footerRight={
        <>
          {step > 0 && (
            <OnboardingBackButton
              onClick={() => {
                onSetVaultDirError(null);
                onBack();
              }}
            />
          )}
          {selectedSafUri && !showAccountDecision && (
            <OnboardingNextButton label={t('onboarding_next')} onClick={onNext} />
          )}
        </>
      }
    >
      {showAccountDecision ? (
        /* 已有账户：让用户选择登录还是创建新账户 */
        <div
          style={{
            padding: 16,
            borderRadius: 12,
            border: '1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)',
            background: 'color-mix(in srgb, var(--accent-primary) 6%, var(--bg-toolbar))',
            textAlign: 'left',
            marginBottom: 24,
          }}
        >
          <div
            style={{
              fontSize: 'var(--text-body)',
              fontWeight: 600,
              color: 'var(--text-primary)',
              marginBottom: 8,
            }}
          >
            {t('onboarding_existing_accounts_title')}
          </div>
          <div
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              marginBottom: 12,
              lineHeight: 1.5,
            }}
          >
            {t('onboarding_existing_accounts_desc', {
              count: foundAccountCount,
            })}
          </div>
          {foundAccounts.length > 0 && (
            <ul
              style={{
                margin: '0 0 12px 0',
                paddingLeft: 18,
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                lineHeight: 1.6,
              }}
            >
              {foundAccounts.slice(0, 3).map((acc) => (
                <li key={acc.id}>{acc.name || acc.id}</li>
              ))}
              {foundAccounts.length > 3 && (
                <li>
                  {t('onboarding_existing_accounts_more', { count: foundAccounts.length - 3 })}
                </li>
              )}
            </ul>
          )}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            <button type="button" {...actionCard(onLoginExisting, 'primary')}>
              <LogIn size={ICON_SIZE.md} />
              {t('onboarding_action_login')}
            </button>
            <button type="button" {...actionCard(onCreateNewAccount, 'warm')}>
              <UserPlus size={ICON_SIZE.md} />
              {t('onboarding_action_create_new')}
            </button>
          </div>
        </div>
      ) : syncPhase === 'syncing' ? (
        /* SAF 同步中：进度条 + 提示 */
        <>
          <IndeterminateProgressBar height={6} style={{ marginBottom: 20 }} />
          <div
            style={{
              fontSize: 'var(--text-body)',
              fontWeight: 600,
              color: 'var(--text-primary)',
              marginBottom: 8,
              overflow: 'hidden',
              whiteSpace: 'nowrap',
            }}
          >
            {syncFileName ? (
              <span
                style={{
                  display: 'inline-block',
                  animation: 'text-scroll 4s ease-in-out infinite',
                  paddingRight: 8,
                  willChange: 'transform',
                }}
              >
                {t('onboarding_vault_dir_syncing_file', {
                  fileName: syncFileName,
                  count: syncFileCount,
                })}
              </span>
            ) : (
              t('onboarding_vault_dir_syncing')
            )}
          </div>
          <div
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
              marginBottom: syncFileName ? 4 : 24,
            }}
          >
            {syncFileName ? (
              <span
                key={syncFileCount}
                style={{
                  display: 'inline-block',
                  animation: 'count-bounce 0.4s cubic-bezier(0.34, 1.56, 0.64, 1)',
                  willChange: 'transform',
                }}
              >
                {t('onboarding_vault_dir_sync_count', { count: syncFileCount })}
              </span>
            ) : (
              t('onboarding_vault_dir_sync_hint')
            )}
          </div>
        </>
      ) : syncPhase === 'done' ? (
        /* 同步完成：成功提示 */
        <div
          style={{
            padding: 16,
            borderRadius: 12,
            border: '1px solid color-mix(in srgb, var(--color-success, #22c55e) 35%, transparent)',
            background: 'color-mix(in srgb, var(--color-success, #22c55e) 8%, var(--bg-toolbar))',
            textAlign: 'center',
            marginBottom: 24,
          }}
        >
          <div style={{ fontSize: 32, marginBottom: 8 }}>✅</div>
          <div
            style={{
              fontSize: 'var(--text-body)',
              fontWeight: 600,
              color: 'var(--text-primary)',
            }}
          >
            {t('onboarding_vault_dir_sync_done')}
          </div>
        </div>
      ) : selectedSafUri ? (
        /* Selected SAF path summary */
        <div
          style={{
            padding: 16,
            borderRadius: 12,
            border: '1px solid color-mix(in srgb, var(--accent-primary) 35%, transparent)',
            background: 'color-mix(in srgb, var(--accent-primary) 6%, var(--bg-toolbar))',
            textAlign: 'left',
            marginBottom: 24,
          }}
        >
          <div
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-secondary)',
              marginBottom: 6,
            }}
          >
            {t('onboarding_vault_dir_selected_label')}
          </div>
          <div
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-primary)',
              wordBreak: 'break-all',
              lineHeight: 1.5,
            }}
          >
            {selectedSafUri}
          </div>
          <button
            type="button"
            onClick={onClearSafUri}
            style={{
              marginTop: 12,
              fontSize: 'var(--text-caption)',
              color: 'var(--accent-primary)',
              background: 'transparent',
              border: 'none',
              padding: 0,
              cursor: 'pointer',
              fontFamily: 'inherit',
              fontWeight: 500,
            }}
          >
            {t('onboarding_vault_dir_reselect')}
          </button>
        </div>
      ) : (
        <>
          {/* Choice: Local vs SAF */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 24 }}>
            <button
              type="button"
              onClick={onPickLocal}
              style={{
                padding: '14px 16px',
                borderRadius: 12,
                borderWidth: 1,
                borderStyle: 'solid',
                cursor: 'pointer',
                textAlign: 'left',
                fontFamily: 'inherit',
              }}
              className="interactive-toolbar"
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
              onClick={onPickSaf}
              disabled={vaultDirActing}
              className="interactive-accent-soft"
              style={{
                padding: '14px 16px',
                borderRadius: 12,
                borderWidth: 1,
                borderStyle: 'solid',
                borderColor: 'color-mix(in srgb, var(--accent-primary) 35%, transparent)',
                cursor: vaultDirActing ? 'wait' : 'pointer',
                textAlign: 'left',
                fontFamily: 'inherit',
                opacity: vaultDirActing ? 0.6 : 1,
              }}
            >
              <div
                style={{
                  fontWeight: 600,
                  marginBottom: 4,
                  color: 'var(--accent-primary)',
                }}
              >
                {vaultDirActing ? t('common:loading') : t('onboarding_vault_dir_saf_title')}
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
      )}
    </OnboardingFrame>
  );
}
