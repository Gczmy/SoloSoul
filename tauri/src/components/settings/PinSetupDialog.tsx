import type { TFunction } from 'i18next';
import { KeyRound } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { PinInput } from '@/components/forms/PinInput';
import { ICON_SIZE } from '@/lib/constants';

const PIN_LENGTH = 6;

export interface PinSetupDialogProps {
  step: 'enter_password' | 'enter_pin' | 'confirm_pin';
  setupError: string | null;
  pinLoading: boolean;
  onPinEntered: (pin: string) => void;
  onPinConfirm: (pin: string) => void;
  onBackToPassword: () => void;
  onBackToEnterPin: () => void;
  t: TFunction;
}

/**
 * PIN 设置向导弹层（PinSection）：enter_password 步骤由父组件叠加
 * 共享 PasswordVerificationDialog，本组件仅渲染 enter_pin / confirm_pin 步骤内容。
 */
export function PinSetupDialog({
  step,
  setupError,
  pinLoading,
  onPinEntered,
  onPinConfirm,
  onBackToPassword,
  onBackToEnterPin,
  t,
}: PinSetupDialogProps) {
  return (
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
        {step === 'enter_pin' && (
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
            <PinInput length={PIN_LENGTH} onComplete={onPinEntered} />
            {/* P004: PIN 为便利解锁，显式提示强度低于主密码（离线爆破残余风险） */}
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                marginTop: 12,
                lineHeight: 1.5,
              }}
            >
              {t('settings:pin_risk_notice')}
            </p>
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
              <Button variant="secondary" onClick={onBackToPassword}>
                {t('common:back')}
              </Button>
            </div>
          </>
        )}

        {step === 'confirm_pin' && (
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
              onComplete={onPinConfirm}
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
              <Button variant="secondary" onClick={onBackToEnterPin}>
                {t('common:back')}
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
