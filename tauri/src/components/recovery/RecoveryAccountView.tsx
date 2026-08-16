import { useTranslation } from 'react-i18next';
import { AlertTriangle } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';
import { Input } from '@/components/ui/Input';
import type { ScannedRecoveryQr } from '@/components/recovery/recoveryReceiveTypes';
import { RecoveryConnectionCard } from './RecoveryConnectionCard';

interface RecoveryAccountViewProps {
  pending: ScannedRecoveryQr;
  loading: boolean;
  statusText: string | null;
  /** 恢复执行进度（recovery-progress 事件）：phase=download/overwrite/create/import/done，percent=0-100 */
  progress: { phase: string; percent: number } | null;
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
  progress,
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
      <RecoveryConnectionCard
        pending={pending}
        loading={loading}
        statusText={statusText}
        progress={progress}
      />

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
            autoComplete="new-password"
            error={masterPasswordError ?? undefined}
          />

          <Input
            label={t('common:confirm_password')}
            type="password"
            value={confirmPassword}
            onChange={(e) => onConfirmPasswordChange(e.target.value)}
            placeholder={t('common:recovery_receive_password_hint')}
            disabled={loading}
            autoComplete="new-password"
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
              ? statusText || t('common:loading')
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
