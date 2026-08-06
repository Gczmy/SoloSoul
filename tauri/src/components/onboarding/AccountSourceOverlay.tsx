import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useUiStore } from '@/stores/uiStore';
import { OnboardingAccountSourceDecision } from '@/components/onboarding/OnboardingAccountSourceDecision';
import { RecoveryReceiveDialog } from '@/components/recovery/RecoveryReceiveDialog';

/**
 * 「返回账户来源选择」独立浮层（从创建新账户页返回时使用）。
 *
 * 重开场景下用户只想重新做「恢复 or 新建」这一个决定，不需要重挂整个引导向导
 * （原先的做法是重挂 OnboardingDialog 并 hack 跳到最后一步，导致「返回」后
 * 露出停在末步、无关闭出口的引导卡片）。
 *
 * 本浮层只承载决策卡片 + 恢复对话框：
 * - 「返回」= 清 reopenAccountSource 关闭浮层，露出底下仍在的 BootstrapPage 创建账户表单
 * - 「创建新账户」= 清标志 + 留在创建账户页（浮层下本就在该页，等于只关浮层）
 * - 「从其它设备恢复」= 打开恢复对话框；恢复成功清标志并跳转登录页
 *
 * 决策卡片（OnboardingAccountSourceDecision）自带 position:absolute; inset:0 遮罩，
 * 直接作为本浮层（position:fixed，构成定位祖先）的子元素渲染，无需改造。
 */
export function AccountSourceOverlay() {
  const navigate = useNavigate();
  const [recoveryOpen, setRecoveryOpen] = useState(false);

  const closeOverlay = () => {
    useUiStore.getState().setReopenAccountSource(false);
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-onboarding)',
        background: 'var(--bg-overlay)',
        backdropFilter: 'blur(4px)',
      }}
    >
      {/* 恢复对话框(z-modal)会被决策卡片(z-onboarding+1)盖住，必须互斥显示 */}
      {!recoveryOpen && (
        <OnboardingAccountSourceDecision
          onRecovery={() => setRecoveryOpen(true)}
          onCreateNew={() => {
            useUiStore.getState().setReopenAccountSource(false);
            navigate('/bootstrap?mode=create', { replace: true });
          }}
          onBack={closeOverlay}
        />
      )}
      {recoveryOpen && (
        <RecoveryReceiveDialog
          isOpen
          onClose={() => setRecoveryOpen(false)}
          onSuccess={() => {
            // 恢复成功后会写入账户，清标志并跳转到登录页解锁新账户
            useUiStore.getState().setReopenAccountSource(false);
            navigate('/login', { replace: true });
          }}
        />
      )}
    </div>
  );
}
