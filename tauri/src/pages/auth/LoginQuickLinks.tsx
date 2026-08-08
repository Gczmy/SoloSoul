import { useTranslation } from 'react-i18next';

/**
 * P013/2: 登录页快捷入口 — 创建新账户 / 从其他设备恢复。
 */
export function LoginQuickLinks({
  onCreateAccount,
  onRestore,
}: {
  onCreateAccount: () => void;
  onRestore: () => void;
}) {
  const { t } = useTranslation();

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: 8,
        marginTop: 16,
      }}
    >
      <button
        type="button"
        onClick={onCreateAccount}
        className="interactive-accent-link"
        style={{
          fontSize: 'var(--text-body-sm)',
          background: 'transparent',
          border: 'none',
          padding: '6px 12px',
          cursor: 'pointer',
          fontFamily: 'inherit',
        }}
      >
        {t('common:create_new_account_link')}
      </button>
      <button
        type="button"
        onClick={onRestore}
        className="interactive-accent-link"
        style={{
          fontSize: 'var(--text-body-sm)',
          background: 'transparent',
          border: 'none',
          padding: '6px 12px',
          cursor: 'pointer',
          fontFamily: 'inherit',
        }}
      >
        {t('common:restore_from_device_link')}
      </button>
    </div>
  );
}
