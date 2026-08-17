import { useTranslation } from 'react-i18next';

import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { Suspense } from 'react';
import { ICON_SIZE } from '@/lib/constants';

import { useLoginPage } from './useLoginPage';
import { LoginBiometricView } from './LoginBiometricView';
import { LoginPinView } from './LoginPinView';
import { LoginPasswordView } from './LoginPasswordView';
import { LoginAccountSelector } from './LoginAccountSelector';
import { LoginQuickLinks } from './LoginQuickLinks';
import { LoginIconBar } from './LoginIconBar';
import styles from './LoginPage.module.css';
import { LazyRecoveryReceiveDialog } from '@/components/recovery/LazyRecoveryReceiveDialog';
import { RecoveryDialogSkeleton } from '@/components/recovery/RecoveryDialogSkeleton';

/**
 * 登录页 — P046 拆分后为纯展示组合层：
 * 全部编排逻辑（账户选择、生物识别/PIN 可用性、解锁 handler、图标栏状态）收敛于
 * useLoginPage 数据 hook；本组件仅负责 JSX 组合与子组件装配。
 */
export function LoginPage() {
  const { t } = useTranslation(['auth', 'common', 'settings']);
  const {
    // account
    accounts,
    selectedAccountId,
    setSelectedAccountId,
    selectedAccount,
    // password input
    password,
    setPassword,
    passwordFieldError,
    setPasswordFieldError,
    passwordErrorTick,
    // store-driven
    isLoading,
    // method & availability
    loginMethod,
    // biometric view
    bioLoading,
    bioLockout,
    bioError,
    // pin view
    pinUnlocking,
    pinError,
    pinInputKey,
    pinInputRef,
    // password view extras
    submitError,
    // handlers
    handleBiometricUnlock,
    handlePinComplete,
    handleSubmit,
    // icon bar
    iconMethods,
    hoveredIcon,
    committedIcon,
    handleIconEnter,
    handleIconLeave,
    handleIconClick,
    // links & recovery
    recoveryOpen,
    setRecoveryOpen,
    listAccounts,
    navigate,
    // performance probe
    t1FiredRef,
  } = useLoginPage();

  const isBiometricMethod =
    loginMethod === 'faceId' || loginMethod === 'touchId' || loginMethod === 'windowsHello';

  return (
    <div className={styles.loginWrapper}>
      <div className={styles.loginCard}>
        <ShieldLogo size={ICON_SIZE['5xl']} style={{ margin: '0 auto 16px' }} />
        <h1 style={{ fontSize: 'var(--text-page-title)', fontWeight: 600, marginBottom: 4 }}>
          {t('auth:login_title')}
        </h1>
        <p
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            marginBottom: 4,
          }}
        >
          {t('auth:login_subtitle')}
        </p>

        {/* Account selector / name — 始终预留空间，避免切换登录方式时下方内容位移 */}
        <LoginAccountSelector
          accounts={accounts}
          selectedAccountId={selectedAccountId}
          onSelect={setSelectedAccountId}
        />

        {/* Biometric unlock — highest-priority method */}
        {isBiometricMethod && loginMethod && (
          <LoginBiometricView
            loginMethod={loginMethod}
            bioLoading={bioLoading}
            bioLockout={bioLockout}
            onUnlock={handleBiometricUnlock}
          />
        )}

        {/* PIN unlock — shown when PIN is the highest available method or user chose it */}
        {loginMethod === 'pin' && (
          <LoginPinView
            pinUnlocking={pinUnlocking}
            pinError={pinError}
            pinInputKey={pinInputKey}
            pinInputRef={pinInputRef}
            onPinComplete={handlePinComplete}
          />
        )}

        {/* Password input — 最低优先级；初始化或缓存回退时也显示，避免白屏 */}
        {(loginMethod === 'password' || loginMethod === null) && (
          <LoginPasswordView
            password={password}
            onPasswordChange={(v) => {
              setPassword(v);
              // 用户修改密码时清除行内错误，避免旧错误残留
              if (passwordFieldError) setPasswordFieldError(null);
            }}
            isLoading={isLoading}
            bioError={bioError}
            submitError={submitError}
            pinError={pinError}
            passwordFieldError={passwordFieldError}
            passwordErrorTick={passwordErrorTick}
            passwordHint={selectedAccount?.passwordHint || null}
            onSubmit={handleSubmit}
            onFocus={() => {
              // T1：首个输入框获焦时记录，仅一次
              if (t1FiredRef.current) return;
              t1FiredRef.current = true;
              const start = (
                window as typeof window & { __SOLOSOUL_APP_START_TIME?: number }
              ).__SOLOSOUL_APP_START_TIME;
              if (typeof start === 'number') {
                // T1 timing is captured internally; no console output in production
              }
            }}
          />
        )}

        {/* 在已有账户的登录页提供创建新账户与从其他设备恢复入口 */}
        <LoginQuickLinks
          onCreateAccount={() => navigate('/bootstrap?mode=create')}
          onRestore={() => setRecoveryOpen(true)}
        />

        {/* ===== 底部图标栏 — 切换解锁方式 ===== */}
        {loginMethod !== null && (
          <LoginIconBar
            loginMethod={loginMethod}
            iconMethods={iconMethods}
            hoveredIcon={hoveredIcon}
            committedIcon={committedIcon}
            onIconEnter={handleIconEnter}
            onIconLeave={handleIconLeave}
            onIconClick={handleIconClick}
          />
        )}

        {recoveryOpen && (
          <Suspense fallback={<RecoveryDialogSkeleton />}>
            <LazyRecoveryReceiveDialog
              isOpen
              onClose={() => setRecoveryOpen(false)}
              onSuccess={() => {
                // 恢复成功后刷新账户列表，让登录页立即显示新恢复的账户
                listAccounts();
              }}
            />
          </Suspense>
        )}
      </div>
    </div>
  );
}
