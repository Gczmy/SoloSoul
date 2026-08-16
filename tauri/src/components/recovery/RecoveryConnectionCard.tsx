import { useTranslation } from 'react-i18next';
import { Loader2 } from 'lucide-react';
import type { ScannedRecoveryQr } from '@/components/recovery/recoveryReceiveTypes';

interface RecoveryConnectionCardProps {
  pending: ScannedRecoveryQr;
  loading: boolean;
  statusText: string | null;
  /** 恢复执行进度（recovery-progress 事件）：phase=download/overwrite/create/import/done，percent=0-100 */
  progress: { phase: string; percent: number } | null;
}

/**
 * 恢复账户卡：账户/连接信息（名称/ID/地址/PIN）+ 传输中进度提示。
 * 从 RecoveryAccountView 抽出，保持渲染结构逐字等价。
 */
export function RecoveryConnectionCard({
  pending,
  loading,
  statusText,
  progress,
}: RecoveryConnectionCardProps) {
  const { t } = useTranslation(['common']);

  const phaseLabel = (phase: string): string => {
    switch (phase) {
      case 'download':
        return t('common:recovery_progress_download', {
          defaultValue: 'Downloading recovery data…',
        });
      case 'overwrite':
        return t('common:recovery_progress_overwrite', {
          defaultValue: 'Deleting the existing account…',
        });
      case 'create':
        return t('common:recovery_progress_create', {
          defaultValue: 'Creating the account…',
        });
      case 'import':
        return t('common:recovery_progress_import', {
          defaultValue: 'Importing data…',
        });
      case 'done':
        return t('common:recovery_progress_done', { defaultValue: 'Recovery complete' });
      default:
        return statusText || t('common:loading');
    }
  };

  return (
    <>
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
              defaultValue:
                'Account detected. Set a new master password for this device, then start recovery.',
            })
          : t('common:recovery_account_card_desc_manual', {
              defaultValue:
                'Connection details ready. Set a new master password for this device, then start recovery.',
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

      {/* 传输中的进度提示：有进度事件时显示确定进度条，否则显示连接中的不确定动画 */}
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
          {progress ? (
            <div style={{ width: '100%' }}>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  marginBottom: 6,
                }}
              >
                <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                  {phaseLabel(progress.phase)}
                </span>
                <span
                  style={{
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-secondary)',
                    fontVariantNumeric: 'tabular-nums',
                  }}
                >
                  {progress.percent}%
                </span>
              </div>
              <div
                style={{
                  width: '100%',
                  height: 6,
                  borderRadius: 3,
                  background: 'var(--border-subtle)',
                  overflow: 'hidden',
                }}
              >
                <div
                  style={{
                    width: `${progress.percent}%`,
                    height: '100%',
                    borderRadius: 3,
                    background: 'linear-gradient(90deg, var(--accent-primary), var(--accent-warm))',
                    transition: 'width 0.3s ease',
                  }}
                />
              </div>
            </div>
          ) : (
            <>
              <Loader2 size={16} style={{ animation: 'spin 1s linear infinite', flexShrink: 0 }} />
              <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                {statusText}
              </span>
            </>
          )}
        </div>
      )}
    </>
  );
}
