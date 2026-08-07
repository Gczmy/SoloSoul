import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { PinInput } from '@/components/forms/PinInput';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { logger } from '@/lib/logger';
import { Grip, KeyRound, AlertTriangle } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';

interface PinSectionProps {
  accountId: string;
}

const PIN_LENGTH = 6;

export function PinSection({ accountId }: PinSectionProps) {
  const { t } = useTranslation(['settings', 'common']);
  const { onSuccess } = useToastError();

  const [pinStatus, setPinStatus] = useState<{
    configured: boolean;
    locked: boolean;
    remainingAttempts: number;
    lockedUntil: string | null;
  } | null>(null);
  const [pinLoading, setPinLoading] = useState(false);

  // Setup flow
  const [showSetup, setShowSetup] = useState(false);
  const [setupStep, setSetupStep] = useState<'enter_password' | 'enter_pin' | 'confirm_pin'>(
    'enter_password',
  );
  const [setupPassword, setSetupPassword] = useState('');
  const [setupPin1, setSetupPin1] = useState('');
  const [setupError, setSetupError] = useState<string | null>(null);
  /** 共享对话框的自定义错误文案（P012：密码验证段统一走共享组件） */
  const [setupPasswordError, setSetupPasswordError] = useState<string | null>(null);

  // Disable flow（P012：禁用确认统一走共享 PasswordVerificationDialog）
  const [showDisableConfirm, setShowDisableConfirm] = useState(false);
  const [disablePasswordError, setDisablePasswordError] = useState<string | null>(null);

  const currentAccount = useAuthStore((s) => s.currentAccount);
  const passwordHint = currentAccount?.passwordHint || null;

  const refreshStatus = useCallback(async () => {
    if (!accountId) return;
    try {
      const status = await invoke<{
        configured: boolean;
        locked: boolean;
        remainingAttempts: number;
        lockedUntil: string | null;
      }>('pin_check_availability', { accountId: accountId });
      setPinStatus(status);
    } catch {
      setPinStatus(null);
    }
  }, [accountId]);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  // ── Setup Flow ──

  const handleSetupStart = () => {
    setSetupStep('enter_password');
    setSetupPassword('');
    setSetupPin1('');
    setSetupError(null);
    setSetupPasswordError(null);
    setShowSetup(true);
  };

  /** 共享 PasswordVerificationDialog 的验证回调（P012）：成功推进到 enter_pin 步骤 */
  const handleSetupVerify = async (password: string): Promise<boolean> => {
    setSetupPasswordError(null);
    try {
      const ok = await invoke<boolean>('verify_password', {
        accountId: accountId,
        password,
      });
      if (ok) {
        setSetupPassword(password);
        return true;
      }
      setSetupPasswordError(t('settings:current_password_incorrect'));
      return false;
    } catch (e) {
      // P123: 后端异常≠密码错误——verify_password 对错误密码返回 false（不抛异常），
      // 走到 catch 的是真实后端故障（锁定/崩溃等），统一报「密码不正确」会误导用户。
      logger.warn('[PinSection] verify_password failed:', e);
      setSetupPasswordError(t('settings:pin_error_setup_failed'));
      return false;
    }
  };

  /** 密码验证通过后由共享对话框回调：推进向导到 PIN 输入步骤 */
  const handleSetupPasswordVerified = () => {
    setSetupStep('enter_pin');
  };

  const handlePinEntered = (pin: string) => {
    setSetupPin1(pin);
    setSetupStep('confirm_pin');
  };

  const handlePinConfirm = async (pin: string) => {
    if (pin !== setupPin1) {
      setSetupError(t('settings:pin_mismatch'));
      setSetupStep('enter_pin');
      setSetupPin1('');
      return;
    }

    setPinLoading(true);
    setSetupError(null);
    try {
      await invoke('pin_setup', {
        accountId: accountId,
        password: setupPassword,
        pin,
      });
      onSuccess(t('settings:pin_setup_success'));
      setShowSetup(false);
      refreshStatus();
    } catch (e) {
      const msg = String(e);
      if (msg.includes('__PIN_ERR__:too_short')) {
        setSetupError(t('settings:pin_error_too_short'));
      } else if (msg.includes('__PIN_ERR__:too_long')) {
        setSetupError(t('settings:pin_error_too_long'));
      } else {
        setSetupError(t('settings:pin_error_setup_failed'));
      }
    } finally {
      setPinLoading(false);
    }
  };

  const handleSetupCancel = () => {
    setShowSetup(false);
    setSetupError(null);
    setSetupPasswordError(null);
  };

  // ── Disable Flow ──

  const handleDisableStart = () => {
    setDisablePasswordError(null);
    setShowDisableConfirm(true);
  };

  /** 共享 PasswordVerificationDialog 的验证回调（P012）：成功即执行 pin_disable */
  const handleDisableVerify = async (password: string): Promise<boolean> => {
    setDisablePasswordError(null);
    setPinLoading(true);
    try {
      await invoke('pin_disable', { accountId: accountId, password });
      onSuccess(t('settings:pin_disabled_toast'));
      setShowDisableConfirm(false);
      refreshStatus();
      return true;
    } catch (e) {
      const msg = String(e);
      if (msg.includes('__PIN_ERR__:invalid_password')) {
        setDisablePasswordError(t('settings:current_password_incorrect'));
      } else {
        setDisablePasswordError(t('settings:pin_error_disable_failed'));
      }
      return false;
    } finally {
      setPinLoading(false);
    }
  };

  // ── Render ──

  if (pinStatus === null) return null;

  return (
    <>
      <Card>
        <h3
          style={{
            fontSize: 'var(--text-card-title)',
            fontWeight: 600,
            marginBottom: 4,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          <Grip size={ICON_SIZE.lg} />
          {t('settings:pin_title')}
        </h3>

        {pinStatus.locked && (
          <div
            style={{
              display: 'flex',
              alignItems: 'flex-start',
              gap: 8,
              padding: '10px 14px',
              borderRadius: 8,
              marginBottom: 12,
              background: 'rgba(212, 133, 10, 0.10)',
              border: '1px solid rgba(212, 133, 10, 0.25)',
              color: '#D4850A',
              fontSize: 'var(--text-caption)',
              lineHeight: 1.4,
            }}
          >
            <AlertTriangle size={ICON_SIZE.md} style={{ flexShrink: 0, marginTop: 1 }} />
            <span>{t('settings:pin_locked_warning')}</span>
          </div>
        )}

        <p
          style={{
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            marginBottom: 12,
          }}
        >
          {pinStatus.configured
            ? t('settings:pin_desc', { length: PIN_LENGTH })
            : t('settings:pin_not_configured_desc')}
        </p>

        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span style={{ fontSize: 'var(--text-body)' }}>
            {pinStatus.configured
              ? t('settings:pin_status_enabled')
              : t('settings:pin_status_disabled')}
          </span>
          <div style={{ display: 'flex', gap: 8 }}>
            {pinStatus.configured ? (
              <>
                <Button variant="secondary" size="sm" onClick={handleSetupStart}>
                  {t('settings:pin_change_button')}
                </Button>
                <Button variant="secondary" size="sm" onClick={handleDisableStart}>
                  {t('settings:pin_disable_button')}
                </Button>
              </>
            ) : (
              <Button variant="primary" size="sm" onClick={handleSetupStart}>
                {t('settings:pin_setup_button')}
              </Button>
            )}
          </div>
        </div>
      </Card>

      {/* ── Setup Dialog ── */}
      {showSetup && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 'var(--z-modal-important)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'rgba(0,0,0,0.45)',
            backdropFilter: 'blur(6px)',
          }}
        >
          <div
            style={{
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
              fontFamily: 'inherit',
              borderRadius: 16,
              padding: '28px 32px',
              maxWidth: 400,
              width: '90%',
              boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)',
              textAlign: 'center',
            }}
          >
            {/* P012: 密码验证段由共享 PasswordVerificationDialog 承载（showSetup 为 true 且处于
                enter_password 步骤时打开），不再手写密码浮层 */}
            {setupStep === 'enter_pin' && (
              <>
                <h3
                  style={{
                    fontSize: 'var(--text-md)',
                    fontWeight: 600,
                    marginBottom: 12,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: 8,
                  }}
                >
                  <KeyRound size={ICON_SIZE.xl} />
                  {t('settings:pin_enter_title')}
                </h3>
                <p
                  style={{
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-secondary)',
                    marginBottom: 20,
                  }}
                >
                  {t('settings:pin_enter_desc', { length: PIN_LENGTH })}
                </p>
                <PinInput length={PIN_LENGTH} onComplete={handlePinEntered} />
                {setupError && (
                  <div
                    style={{
                      color: '#dc2626',
                      fontSize: 'var(--text-body-sm)',
                      padding: '4px 0',
                      marginTop: 12,
                    }}
                  >
                    {setupError}
                  </div>
                )}
                <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 16 }}>
                  <Button variant="secondary" onClick={() => setSetupStep('enter_password')}>
                    {t('common:back')}
                  </Button>
                </div>
              </>
            )}

            {setupStep === 'confirm_pin' && (
              <>
                <h3
                  style={{
                    fontSize: 'var(--text-md)',
                    fontWeight: 600,
                    marginBottom: 12,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: 8,
                  }}
                >
                  <KeyRound size={ICON_SIZE.xl} />
                  {t('settings:pin_confirm_title')}
                </h3>
                <p
                  style={{
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-secondary)',
                    marginBottom: 20,
                  }}
                >
                  {t('settings:pin_confirm_desc')}
                </p>
                <PinInput
                  length={PIN_LENGTH}
                  onComplete={handlePinConfirm}
                  disabled={pinLoading}
                  error={!!setupError}
                />
                {setupError && (
                  <div
                    style={{
                      color: '#dc2626',
                      fontSize: 'var(--text-body-sm)',
                      padding: '4px 0',
                      marginTop: 12,
                    }}
                  >
                    {setupError}
                  </div>
                )}
                <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 16 }}>
                  <Button
                    variant="secondary"
                    onClick={() => {
                      setSetupStep('enter_pin');
                      setSetupPin1('');
                      setSetupError(null);
                    }}
                  >
                    {t('common:back')}
                  </Button>
                </div>
              </>
            )}
          </div>
        </div>
      )}

      {/* ── Disable Dialog（P012: 统一走共享 PasswordVerificationDialog） ── */}
      <PasswordVerificationDialog
        open={showDisableConfirm}
        onClose={() => {
          setShowDisableConfirm(false);
          setDisablePasswordError(null);
        }}
        onVerify={handleDisableVerify}
        title={t('settings:pin_disable_title')}
        description={t('settings:pin_disable_desc')}
        hint={passwordHint}
        errorMessage={disablePasswordError}
        onPasswordChange={() => setDisablePasswordError(null)}
      />

      {/* ── Setup 向导密码验证段（P012: 统一走共享 PasswordVerificationDialog） ── */}
      <PasswordVerificationDialog
        open={showSetup && setupStep === 'enter_password'}
        onClose={handleSetupCancel}
        onVerify={handleSetupVerify}
        onVerifySuccess={handleSetupPasswordVerified}
        title={t('settings:pin_verify_password_title')}
        hint={passwordHint}
        errorMessage={setupPasswordError}
        onPasswordChange={() => setSetupPasswordError(null)}
      />
    </>
  );
}
