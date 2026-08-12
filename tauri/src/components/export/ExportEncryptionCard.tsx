import type { TFunction } from 'i18next';
import { Card } from '@/components/ui/Card';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';

/**
 * ExportSection 的「加密」卡片（P046 拆分：展示子组件）。
 * 敏感数据警告 + 主密码/确认密码/密码提示词输入。
 */
export function ExportEncryptionCard({
  exportPassword,
  exportPasswordConfirm,
  exportHint,
  hasSensitiveData,
  onSetExportPassword,
  onSetExportPasswordConfirm,
  onSetExportHint,
  onExport,
  t,
}: {
  exportPassword: string;
  exportPasswordConfirm: string;
  exportHint: string;
  hasSensitiveData: boolean;
  onSetExportPassword: (v: string) => void;
  onSetExportPasswordConfirm: (v: string) => void;
  onSetExportHint: (v: string) => void;
  onExport: () => void;
  t: TFunction;
}) {
  return (
    <Card>
      <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
        {t('settings:encryption')}
      </h3>

      {hasSensitiveData && (
        <div
          style={{
            marginBottom: 10,
            padding: '8px 12px',
            background: 'var(--warning-subtle)',
            borderRadius: 6,
            fontSize: 'var(--text-caption)',
            color: 'var(--warning)',
            border: '1px solid var(--warning)',
          }}
        >
          {t('settings:sensitive_export_warning')}
        </div>
      )}

      <SecurePasswordInput
        value={exportPassword}
        onChange={onSetExportPassword}
        placeholder={t('common:password_placeholder')}
        showHintButton={false}
        onEnter={onExport}
      />
      <div style={{ marginTop: 8 }}>
        <SecurePasswordInput
          value={exportPasswordConfirm}
          onChange={onSetExportPasswordConfirm}
          placeholder={t('settings:confirm_password')}
          showHintButton={false}
          onEnter={onExport}
        />
      </div>
      {exportPassword && exportPasswordConfirm && exportPassword !== exportPasswordConfirm && (
        <div style={{ marginTop: 4, fontSize: 'var(--text-caption)', color: 'var(--danger)' }}>
          {t('settings:password_mismatch')}
        </div>
      )}
      <div style={{ marginTop: 8 }}>
        <input
          type="text"
          value={exportHint}
          onChange={(e) => onSetExportHint(e.target.value)}
          placeholder={t('common:password_hint')}
          maxLength={200}
          className="interactive-field"
          style={{
            width: '100%',
            padding: '10px 14px',
            fontSize: 'var(--text-body)',
            borderWidth: 1,
            borderStyle: 'solid',
            borderRadius: 8,
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            fontFamily: 'inherit',
            outline: 'none',
          }}
        />
      </div>
    </Card>
  );
}
