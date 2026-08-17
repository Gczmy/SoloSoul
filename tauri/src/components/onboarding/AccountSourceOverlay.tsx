import { lazy, Suspense, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useUiStore } from '@/stores/uiStore';
import { useAuthStore } from '@/stores/authStore';
import { OnboardingAccountSourceDecision } from '@/components/onboarding/OnboardingAccountSourceDecision';
// P015-R2: 恢复接收对话框(内部 RecoveryQrScanner→html5-qrcode 375K)由入口静态导入
// 改为懒加载——仅真正打开恢复对话框时拉取，html5-qrcode 移出启动链。
const RecoveryReceiveDialog = lazy(() =>
  import('@/components/recovery/RecoveryReceiveDialog').then((m) => ({
    default: m.RecoveryReceiveDialog,
  })),
);

/**
 * 「返回账户来源选择」独立浮层（从创建新账户页返回时使用）。
 *
 * 重开场景下用户只想重新做「恢复 or 新建」这一个决定，不需要重挂整个引导向导
 * （原先的做法是重挂 OnboardingDialog 并 hack 跳到最后一步，导致「返回」后
 * 露出停在末步、无关闭出口的引导卡片）。
 *
 * 本浮层只承载决策卡片 + 恢复对话框：
 * - 本地已有账户（hasAccount !== false）：「返回登录」= 清标志并跳转 /login
 * - 首次启动（hasAccount === false，无登录页可回）：按钮保持「返回」= 仅关闭
 *   浮层露出创建账户表单（/login 在无账户时会重定向回 /bootstrap，导航过去
 *   会形成回跳死循环——与 BootstrapPage 返回登录守卫同一逻辑）
 * - 「创建新账户」= 清标志 + 留在创建账户页（浮层下本就在该页，等于只关浮层）
 * - 「从其它设备恢复」= 打开恢复对话框；恢复成功清标志并跳转登录页
 *
 * 决策卡片（OnboardingAccountSourceDecision）自带 position:absolute; inset:0 遮罩，
 * 直接作为本浮层（position:fixed，构成定位祖先）的子元素渲染，无需改造。
 */
export function AccountSourceOverlay() {
  const navigate = useNavigate();
  const { t } = useTranslation('common');
  const [recoveryOpen, setRecoveryOpen] = useState(false);
  // 首次启动（本地无任何账户）时无登录页可回：hasAccount === false 时
  // 「返回」仅关闭浮层（hasAccount 为 null/true 时均可安全跳登录页）。
  const hasAccount = useAuthStore((s) => s.hasAccount);
  const canGoBackToLogin = hasAccount !== false;

  const closeOverlay = () => {
    useUiStore.getState().setReopenAccountSource(false);
  };

  const handleBack = () => {
    closeOverlay();
    if (canGoBackToLogin) {
      navigate('/login', { replace: true });
    }
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
          onBack={handleBack}
          backLabel={
            canGoBackToLogin ? t('back_to_login_link', { defaultValue: 'Back to login' }) : undefined
          }
        />
      )}
      {recoveryOpen && (
        <Suspense fallback={null}>
          <RecoveryReceiveDialog
            isOpen
            onClose={() => setRecoveryOpen(false)}
            onSuccess={() => {
              // 恢复成功后会写入账户，清标志并跳转到登录页解锁新账户
              useUiStore.getState().setReopenAccountSource(false);
              navigate('/login', { replace: true });
            }}
          />
        </Suspense>
      )}
    </div>
  );
}
