import { useTranslation } from 'react-i18next';
import { Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import type { ScannedRecoveryQr } from '@/components/recovery/recoveryReceiveTypes';

interface RecoveryAccountViewProps {
  pending: ScannedRecoveryQr;
  loading: boolean;
  statusText: string | null;
  masterPassword: string;
  confirmPassword: string;
  passwordHint: string;
  masterPasswordError: string | null;
  confirmPasswordError: string | null;
  error: string | null;
  onMasterPasswordChange: (v: string) => void;
  onConfirmPasswordChange: (v: string) => void;
  onPasswordHintChange: (v: string) => void;
  onStartRecovery: () => void;
  onBackToCollect: () => void;
}

/** 账户卡：确认账户/连接信息 + 设置主密码（连接前）。 */
export function RecoveryAccountView({
  pending,
  loading,
  statusText,
  masterPassword,
  confirmPassword,
  passwordHint,
  masterPasswordError,
  confirmPasswordError,
  error,
  onMasterPasswordChange,
  onConfirmPasswordChange,
  onPasswordHintChange,
  onStartRecovery,
  onBackToCollect,
}: RecoveryAccountViewProps) {
  const { t } = useTranslation(['common']);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <p
        style={{
          fontSize: 'var(--text-body-sm)',
          color: 'var(--text-secondary)',
          margin: '0 0 4px',
          lineHeight: 1.5,
        }}
      >
        {pending.accountName
          ? t('common:recovery_account_card_desc_scan', {
              defaultValue: 'Account detected. Set a new master password for this device, then start recovery.',
            })
          : t('common:recovery_account_card_desc_manual', {
              defaultValue: 'Connection details ready. Set a new master password for this device, then start recovery.',
            })}
      </p>

      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
          padding: '12px 14px',
          borderRadius: 8,
          background: 'var(--bg-toolbar)',
        }}
      >
        {pending.accountName && (
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
            }}
          >
            <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
              {t('common:recovery_account_name_label', { defaultValue: 'Account Name' })}
            </span>
            <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
              {pending.accountName}
            </span>
          </div>
        )}
        {pending.accountId && (
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
            }}
          >
            <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
              {t('common:recovery_account_id_label', { defaultValue: 'Account ID' })}
            </span>
            <span
              style={{
                fontFamily: 'monospace',
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-primary)',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                maxWidth: '60%',
              }}
            >
              {pending.accountId}
            </span>
          </div>
        )}
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
            {t('common:recovery_host_addr_label')}
          </span>
          <span
            style={{
              fontFamily: 'monospace',
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-primary)',
            }}
          >
            {pending.addr}
          </span>
        </div>
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <span style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
            {t('common:recovery_host_pin_label')}
          </span>
          <span
            style={{
              fontFamily: 'monospace',
              fontSize: 'var(--text-body-sm)',
              fontWeight: 700,
              letterSpacing: 4,
              color: 'var(--accent-primary)',
            }}
          >
            {pending.pin}
          </span>
        </div>
      </div>

      {/* 传输中的状态提示 */}
      {loading && statusText && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '10px 12px',
            borderRadius: 8,
            background: 'rgba(52,152,219,0.08)',
            border: '1px solid rgba(52,152,219,0.2)',
            width: '100%',
            boxSizing: 'border-box',
          }}
        >
          <Loader2
            size={16}
            style={{ animation: 'spin 1s linear infinite', flexShrink: 0 }}
          />
          <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
            {statusText}
          </span>
        </div>
      )}

      <Input
        label={t('common:recovery_receive_password_label')}
        type="password"
        value={masterPassword}
        onChange={(e) => onMasterPasswordChange(e.target.value)}
        placeholder={t('common:recovery_receive_password_hint')}
        disabled={loading}
        autoFocus
        error={masterPasswordError ?? undefined}
      />

      <Input
        label={t('common:confirm_password')}
        type="password"
        value={confirmPassword}
        onChange={(e) => onConfirmPasswordChange(e.target.value)}
        placeholder={t('common:recovery_receive_password_hint')}
        disabled={loading}
        error={confirmPasswordError ?? undefined}
      />

      <Input
        label={t('common:password_hint')}
        type="text"
        value={passwordHint}
        onChange={(e) => onPasswordHintChange(e.target.value)}
        placeholder={t('common:password_hint_placeholder')}
        disabled={loading}
      />

      <Button
        onClick={onStartRecovery}
        disabled={loading}
        loading={loading}
        style={{ width: '100%', marginTop: 4 }}
      >
        {loading
          ? (statusText || t('common:loading'))
          : t('common:recovery_receive_start')}
      </Button>

      <Button
        variant="secondary"
        onClick={onBackToCollect}
        disabled={loading}
        style={{ width: '100%' }}
      >
        {t('common:back')}
      </Button>

      {error && (
        <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)' }}>{error}</div>
      )}
    </div>
  );
}
