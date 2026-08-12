import { useTranslation } from 'react-i18next';
import { Dialog } from '@/components/ui/Dialog';
import { PinEntryCard } from '@/components/forms/PinEntryCard';
import { LoginIconBar } from '@/pages/auth/LoginIconBar';

import { usePasswordVerification, type PasswordVerificationDialogProps } from './usePasswordVerification';
import { PasswordVerificationBiometricCard } from './PasswordVerificationBiometricCard';
import { PasswordVerificationPasswordCard } from './PasswordVerificationPasswordCard';

export type { PasswordVerificationDialogProps } from './usePasswordVerification';

/**
 * Unified password verification dialog — single source of truth for all
 * password-gated operations across the app.
 *
 * 按优先级显示解锁方式卡片：Face ID > Touch ID > Windows Hello > PIN > 密码
 * 底部统一图标栏切换方式（主密码 · Face ID · Touch ID · Windows Hello · PIN）。
 * 悬停图标时展开文字，左侧按钮不动右侧按钮被推向右。
 *
 * P046 拆分后为纯展示组合层：状态与 handler 收敛于 usePasswordVerification 数据 hook，
 * 生物识别/密码卡片为独立展示子组件，PIN 卡片复用共享 PinEntryCard。
 */
export function PasswordVerificationDialog(props: PasswordVerificationDialogProps) {
  const {
    open,
    title,
    description,
    confirmLabel,
    hint,
  } = props;
  const { t } = useTranslation(['auth', 'common', 'settings']);
  const {
    loginMethod,
    bioLoading,
    biometricLabel,
    handleBiometric,
    pinUnlocking,
    pinError,
    pinInputKey,
    handlePinComplete,
    password,
    handlePasswordChange,
    inputError,
    loading,
    handleConfirm,
    handleClose,
    methods,
    hoveredIcon,
    committedIcon,
    handleIconEnter,
    handleIconLeave,
    handleIconClick,
  } = usePasswordVerification(props);

  return (
    <Dialog isOpen={open} onClose={handleClose} dialogStyle={{ maxWidth: 360 }} priority="auth">
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16, minWidth: 320 }}>
        <h2 style={{ fontSize: 'var(--text-section-title)', fontWeight: 600, margin: 0 }}>
          {title || t('auth:verification_title')}
        </h2>
        {description && (
          <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', margin: 0 }}>
            {description}
          </p>
        )}

        {/* ===== 生物识别卡片 ===== */}
        {(loginMethod === 'faceId' ||
          loginMethod === 'touchId' ||
          loginMethod === 'windowsHello') && (
          <PasswordVerificationBiometricCard
            loginMethod={loginMethod}
            bioLoading={bioLoading}
            biometricLabel={biometricLabel}
            onUnlock={handleBiometric}
            t={t}
          />
        )}

        {/* ===== PIN 码卡片（P040: 共享 PinEntryCard）===== */}
        {loginMethod === 'pin' && (
          <PinEntryCard
            pinUnlocking={pinUnlocking}
            pinError={pinError}
            pinInputKey={pinInputKey}
            onPinComplete={handlePinComplete}
            marginBottom={8}
          />
        )}

        {/* ===== 密码卡片 ===== */}
        {(loginMethod === 'password' || loginMethod === null) && (
          <PasswordVerificationPasswordCard
            password={password}
            onPasswordChange={handlePasswordChange}
            hint={hint}
            error={inputError}
            loading={loading}
            confirmLabel={confirmLabel}
            onConfirm={handleConfirm}
            onCancel={handleClose}
            t={t}
          />
        )}

        {/* loginMethod === null（正在检测可用性）时显示轻量 loading */}
        {loginMethod === null && (
          <div
            style={{
              display: 'flex',
              justifyContent: 'center',
              padding: '24px 0',
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-tertiary)',
            }}
          >
            {t('common:loading', { defaultValue: '...' })}
          </div>
        )}

        {/* ===== 底部图标栏 — 切换解锁方式（复用 LoginIconBar，P013/6） ===== */}
        {loginMethod !== null && methods.length > 1 && (
          <LoginIconBar
            loginMethod={loginMethod}
            iconMethods={methods}
            hoveredIcon={hoveredIcon}
            committedIcon={committedIcon}
            onIconEnter={handleIconEnter}
            onIconLeave={handleIconLeave}
            onIconClick={handleIconClick}
          />
        )}
      </div>
    </Dialog>
  );
}
