import type { TFunction } from 'i18next';
import { WarningCancelButton } from './WarningCancelButton';
import { TransferButton } from '@/components/transfer/TransferButton';

/**
 * ExportSection 的导出风险确认区（P046 拆分：展示子组件）。
 * 弱密码确认 + 密码提示词含密码确认两个内联警告块。
 */
export function ExportWarningDialogs({
  showWeakPasswordWarning,
  showHintWarning,
  onSetShowWeakPasswordWarning,
  onSetShowHintWarning,
  onSetWeakPasswordExport,
  onSetShowHintWarningAndExport,
  t,
}: {
  showWeakPasswordWarning: boolean;
  showHintWarning: boolean;
  onSetShowWeakPasswordWarning: (v: boolean) => void;
  onSetShowHintWarning: (v: boolean) => void;
  onSetWeakPasswordExport: () => void;
  onSetShowHintWarningAndExport: () => void;
  t: TFunction;
}) {
  return (
    <>
      {/* Weak password confirmation dialog */}
      {showWeakPasswordWarning && (
        <div
          style={{
            padding: '12px 16px',
            borderRadius: 8,
            background: 'var(--warning-subtle)',
            border: '1px solid var(--warning)',
            fontSize: 'var(--text-body-sm)',
            color: 'var(--warning)',
          }}
        >
          <p style={{ marginBottom: 8, fontWeight: 600 }}>{t('settings:weak_password_title')}</p>
          <p style={{ marginBottom: 10 }}>{t('settings:weak_password_confirm')}</p>
          <div style={{ display: 'flex', gap: 8 }}>
            <WarningCancelButton onClick={() => onSetShowWeakPasswordWarning(false)}>
              {t('common:cancel')}
            </WarningCancelButton>
            <TransferButton variant="warning" onClick={onSetWeakPasswordExport}>
              {t('settings:export_anyway')}
            </TransferButton>
          </div>
        </div>
      )}

      {/* Password hint risk confirmation dialog */}
      {showHintWarning && (
        <div
          style={{
            padding: '12px 16px',
            borderRadius: 8,
            background: 'var(--warning-subtle)',
            border: '1px solid var(--warning)',
            fontSize: 'var(--text-body-sm)',
            color: 'var(--warning)',
          }}
        >
          <p style={{ marginBottom: 8, fontWeight: 600 }}>
            {t('settings:hint_contains_password_title')}
          </p>
          <p style={{ marginBottom: 10 }}>{t('settings:hint_contains_password_confirm')}</p>
          <div style={{ display: 'flex', gap: 8 }}>
            <WarningCancelButton onClick={() => onSetShowHintWarning(false)}>
              {t('common:cancel')}
            </WarningCancelButton>
            <TransferButton variant="warning" onClick={onSetShowHintWarningAndExport}>
              {t('settings:export_anyway')}
            </TransferButton>
          </div>
        </div>
      )}
    </>
  );
}
