import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { usePinSetup } from '@/hooks/usePinSetup';
import { usePinDisable } from '@/hooks/usePinDisable';
import { PinSetupDialog } from './PinSetupDialog';
import { Grip, AlertTriangle } from 'lucide-react';
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

  // 子 hook：设置向导 / 禁用确认（共享 pinLoading 与 refreshStatus）
  const setup = usePinSetup({ accountId, t, onSuccess, refreshStatus, setPinLoading });
  const disable = usePinDisable({ accountId, t, onSuccess, refreshStatus, setPinLoading });

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
                <Button variant="secondary" size="sm" onClick={setup.handleSetupStart}>
                  {t('settings:pin_change_button')}
                </Button>
                <Button variant="secondary" size="sm" onClick={disable.handleDisableStart}>
                  {t('settings:pin_disable_button')}
                </Button>
              </>
            ) : (
              <Button variant="primary" size="sm" onClick={setup.handleSetupStart}>
                {t('settings:pin_setup_button')}
              </Button>
            )}
          </div>
        </div>
      </Card>

      {/* ── Setup Dialog ── */}
      {setup.showSetup && (
        <PinSetupDialog
          step={setup.setupStep}
          setupError={setup.setupError}
          pinLoading={pinLoading}
          onPinEntered={setup.handlePinEntered}
          onPinConfirm={setup.handlePinConfirm}
          onBackToPassword={setup.goToPasswordStep}
          onBackToEnterPin={setup.backToEnterPin}
          t={t}
        />
      )}

      {/* ── Disable Dialog（P012: 统一走共享 PasswordVerificationDialog） ── */}
      <PasswordVerificationDialog
        open={disable.showDisableConfirm}
        onClose={disable.closeDisable}
        onVerify={disable.handleDisableVerify}
        title={t('settings:pin_disable_title')}
        description={t('settings:pin_disable_desc')}
        hint={passwordHint}
        errorMessage={disable.disablePasswordError}
        onPasswordChange={disable.clearDisablePasswordError}
      />

      {/* ── Setup 向导密码验证段（P012: 统一走共享 PasswordVerificationDialog） ── */}
      <PasswordVerificationDialog
        open={setup.showSetup && setup.setupStep === 'enter_password'}
        onClose={setup.handleSetupCancel}
        onVerify={setup.handleSetupVerify}
        onVerifySuccess={setup.handleSetupPasswordVerified}
        title={t('settings:pin_verify_password_title')}
        hint={passwordHint}
        errorMessage={setup.setupPasswordError}
        onPasswordChange={setup.clearSetupPasswordError}
      />
    </>
  );
}
