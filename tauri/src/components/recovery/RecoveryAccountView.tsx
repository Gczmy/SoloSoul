import { useTranslation } from 'react-i18next';
import { AlertTriangle, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';
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
  /** 本设备已存在相同 account_id（账户 ID 冲突）→ 展示覆盖恢复警示框 */
  idConflict: boolean;
  /** 用户已确认覆盖恢复 → 进入覆盖模式密码输入 */
  overwriteApproved: boolean;
  /** 二次确认覆盖弹窗是否打开 */
  confirmingOverwrite: boolean;
  onRequestOverwrite: () => void;
  onCancelConflict: () => void;
  onCancelOverwriteConfirm: () => void;
  onConfirmOverwrite: () => void;
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
  idConflict,
  overwriteApproved,
  confirmingOverwrite,
  onRequestOverwrite,
  onCancelConflict,
  onCancelOverwriteConfirm,
  onConfirmOverwrite,
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

      {idConflict && !overwriteApproved ? (
        <>
          {/* 账户 ID 冲突警示框（密码输入之前）：本设备已有相同 account_id */}
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 10,
              padding: '12px 14px',
              borderRadius: 8,
              background: 'rgba(231,76,60,0.08)',
              border: '1px solid rgba(231,76,60,0.35)',
              width: '100%',
              boxSizing: 'border-box',
            }}
          >
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                color: '#e74c3c',
                fontWeight: 700,
                fontSize: 'var(--text-body)',
              }}
            >
              <AlertTriangle size={16} style={{ flexShrink: 0 }} />
              {t('common:recovery_id_conflict_title')}
            </div>
            <p
              style={{
                margin: 0,
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
              }}
            >
              {t('common:recovery_id_conflict_desc')}
            </p>
            <div style={{ display: 'flex', gap: 8, marginTop: 2 }}>
              <Button
                variant="danger"
                onClick={onRequestOverwrite}
                disabled={loading}
                style={{ flex: 1 }}
              >
                {t('common:recovery_overwrite_confirm')}
              </Button>
              <Button
                variant="secondary"
                onClick={onCancelConflict}
                disabled={loading}
                style={{ flex: 1 }}
              >
                {t('common:cancel')}
              </Button>
            </div>
          </div>
        </>
      ) : (
        <>
          {/* 覆盖模式提示：已确认覆盖，将用旧设备数据替换本端账户 */}
          {idConflict && (
            <div
              style={{
                padding: '10px 12px',
                borderRadius: 8,
                background: 'rgba(243,156,18,0.08)',
                border: '1px solid rgba(243,156,18,0.3)',
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                lineHeight: 1.5,
              }}
            >
              {t('common:recovery_overwrite_mode_note', {
                defaultValue:
                  'Overwrite confirmed: the existing account data on this device will be replaced by the old device data after restore.',
              })}
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
              : idConflict
                ? t('common:recovery_overwrite_start', { defaultValue: 'Overwrite & Restore' })
                : t('common:recovery_receive_start')}
          </Button>
        </>
      )}

      {/* 冲突未确认（警示框阶段）由警示框内「取消」返回扫码页；覆盖模式与普通模式展示「返回」 */}
      {(!idConflict || overwriteApproved) && (
        <Button
          variant="secondary"
          onClick={onBackToCollect}
          disabled={loading}
          style={{ width: '100%' }}
        >
          {t('common:back')}
        </Button>
      )}

      {error && !idConflict && (
        <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)' }}>{error}</div>
      )}

      {/* 二次确认覆盖弹窗：确认后进入覆盖模式密码输入 */}
      <ConfirmDialog
        isOpen={confirmingOverwrite}
        title={t('common:recovery_overwrite_confirm_title')}
        message={t('common:recovery_overwrite_confirm_desc')}
        confirmLabel={t('common:recovery_overwrite_confirm_ok')}
        cancelLabel={t('common:cancel')}
        priority="important"
        onConfirm={onConfirmOverwrite}
        onCancel={onCancelOverwriteConfirm}
      />
    </div>
  );
}
