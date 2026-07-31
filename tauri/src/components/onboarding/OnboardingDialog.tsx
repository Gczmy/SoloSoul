import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useOnboarding, type OnboardingDialogProps } from '@/hooks/useOnboarding';
import { OnboardingFrame, OnboardingBackButton, OnboardingNextButton } from '@/components/onboarding/OnboardingFrame';
import { OnboardingVaultDirStep } from '@/components/onboarding/OnboardingVaultDirStep';
import { OnboardingAccountSourceDecision } from '@/components/onboarding/OnboardingAccountSourceDecision';
import { RecoveryReceiveDialog } from '@/components/recovery/RecoveryReceiveDialog';

/**
 * 首次启动引导向导（多步骤）：
 * - welcome / create_object / templates / security / finish：通用步骤框架
 * - vault_directory（仅 Android）：本地/SAF 目录选择 + 已有账户决策
 * - 完成引导后若本地无账户，弹「账户来源」浮层询问恢复或新建
 * 状态机与业务逻辑收敛在 useOnboarding hook，本组件仅编排视图。
 */
export function OnboardingDialog({ onComplete, onSkip: _onSkip }: OnboardingDialogProps) {
  const { t } = useTranslation('common');
  const navigate = useNavigate();
  const ob = useOnboarding({ onComplete, onSkip: _onSkip });

  // Show only the vault directory step when we need to display it
  if (ob.current.key === 'vault_directory') {
    return (
      <OnboardingVaultDirStep
        steps={ob.steps}
        step={ob.step}
        vaultDirActing={ob.vaultDirActing}
        vaultDirError={ob.vaultDirError}
        selectedSafUri={ob.selectedSafUri}
        syncPhase={ob.syncPhase}
        syncFileName={ob.syncFileName}
        syncFileCount={ob.syncFileCount}
        showAccountDecision={ob.showAccountDecision}
        foundAccounts={ob.foundAccounts}
        foundAccountCount={ob.foundAccountCount}
        onPickLocal={ob.handleLocalDirPick}
        onPickSaf={ob.handleVaultDirPick}
        onLoginExisting={ob.handleLoginExisting}
        onCreateNewAccount={ob.handleCreateNewAccount}
        onClearSafUri={ob.clearSelectedSafUri}
        onBack={() => ob.setStep((s) => s - 1)}
        onNext={() => ob.setStep((s) => s + 1)}
        onSetVaultDirError={ob.setVaultDirError}
      />
    );
  }

  // Regular step rendering
  return (
    <OnboardingFrame
      icon={ob.Icon}
      title={t(`onboarding_${ob.current.key}_title`)}
      desc={t(`onboarding_${ob.current.key}_desc`)}
      steps={ob.steps}
      step={ob.step}
      footerLeft={<div />}
      footerRight={
        <>
          {ob.step > 0 && (
            <OnboardingBackButton onClick={() => ob.setStep((s) => s - 1)} />
          )}
          <OnboardingNextButton
            label={ob.isLast ? t('onboarding_done') : t('onboarding_next')}
            onClick={ob.handleFinishClick}
          />
        </>
      }
    >
      {/* 账户来源决策：新设备或从其它设备同步恢复 */}
      {ob.showAccountSourceDecision && (
        <OnboardingAccountSourceDecision
          onRecovery={() => ob.setRecoveryOpen(true)}
          onCreateNew={() => {
            onComplete();
            navigate('/bootstrap?mode=create', { replace: true });
          }}
          onBack={() => ob.setShowAccountSourceDecision(false)}
        />
      )}

      <RecoveryReceiveDialog
        isOpen={ob.recoveryOpen}
        onClose={() => {
          ob.setRecoveryOpen(false);
        }}
        onSuccess={() => {
          // 恢复成功后会写入账户，直接结束引导并跳转到登录页解锁新账户
          onComplete();
          navigate('/login', { replace: true });
        }}
      />
    </OnboardingFrame>
  );
}
