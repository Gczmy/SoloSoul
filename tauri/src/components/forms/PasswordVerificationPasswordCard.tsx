import type { TFunction } from 'i18next';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { Button } from '@/components/ui/Button';

/**
 * PasswordVerificationDialog 的密码输入卡片（P046 拆分：展示子组件）。
 * 纯展示：密码输入 + 取消/确认按钮，行为由 onConfirm/onCancel 转发。
 */
export function PasswordVerificationPasswordCard({
  password,
  onPasswordChange,
  hint,
  error,
  loading,
  confirmLabel,
  onConfirm,
  onCancel,
  t,
}: {
  password: string;
  onPasswordChange: (v: string) => void;
  hint?: string | null;
  error: string | null;
  loading: boolean;
  confirmLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
  t: TFunction;
}) {
  return (
    <div
      style={{
        minHeight: 152,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        marginBottom: 8,
      }}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        <SecurePasswordInput
          value={password}
          onChange={onPasswordChange}
          placeholder={t('common:password_placeholder')}
          error={error}
          // SoloSoul 主密码非网站密码：禁用浏览器/密码管理器自动填充（current-password 会显示历史密码明文）
          autoComplete="off"
          hint={hint}
          onEnter={onConfirm}
        />
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
          <Button variant="secondary" onClick={onCancel}>
            {t('common:cancel')}
          </Button>
          <Button onClick={onConfirm} loading={loading} disabled={!password}>
            {confirmLabel || t('common:confirm')}
          </Button>
        </div>
      </div>
    </div>
  );
}
