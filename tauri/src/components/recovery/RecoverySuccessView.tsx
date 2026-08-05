import { useTranslation } from 'react-i18next';
import { CheckCircle2 } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import type { RecoveryResultSummary } from '@/components/recovery/recoveryReceiveTypes';

interface RecoverySuccessViewProps {
  success: RecoveryResultSummary;
  /** 点击「完成」：打开「恢复完成」确认框（确认后返回登录页）。 */
  onComplete: () => void;
}

/** 成功卡片：显示账户信息 + 导入统计。 */
export function RecoverySuccessView({ success, onComplete }: RecoverySuccessViewProps) {
  const { t } = useTranslation(['common']);

  return (
    <div style={{ textAlign: 'center', padding: '12px 0' }}>
      <div
        style={{
          width: 56,
          height: 56,
          borderRadius: '50%',
          background: 'rgba(39,174,96,0.12)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          margin: '0 auto 16px',
        }}
      >
        <CheckCircle2 size={32} color="#27ae60" />
      </div>
      <h3
        style={{
          fontSize: 'var(--text-body)',
          fontWeight: 600,
          margin: '0 0 8px',
          color: 'var(--text-primary)',
        }}
      >
        {t('common:recovery_receive_success')}
      </h3>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
          textAlign: 'left',
          margin: '16px 0',
          padding: '12px 14px',
          borderRadius: 8,
          background: 'var(--bg-toolbar)',
        }}
      >
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
            {success.accountName}
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
            {success.accountId}
          </span>
        </div>
      </div>
      <p
        style={{
          fontSize: 'var(--text-body-sm)',
          color: 'var(--text-secondary)',
          margin: '0 0 24px',
        }}
      >
        {t('common:recovery_receive_success_desc', {
          objects: success.objectCount,
          attachments: success.attachmentCount,
        })}
      </p>
      <Button onClick={onComplete} style={{ width: '100%' }}>
        {t('common:onboarding_done')}
      </Button>
    </div>
  );
}
