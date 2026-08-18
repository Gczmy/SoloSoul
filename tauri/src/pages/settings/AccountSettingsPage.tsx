import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { useToastError } from '@/hooks/useToastError';
import { Copy } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

/** 设置页「账户管理」：查看账户 ID（不可修改）、修改账户名。 */
export function AccountSettingsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const refreshCurrentAccount = useAuthStore((s) => s.refreshCurrentAccount);
  const showToast = useUiStore((s) => s.showToast);
  const { onError } = useToastError();
  const [name, setName] = useState(currentAccount?.name ?? '');
  const [saving, setSaving] = useState(false);

  // 外部刷新账户名（如重装恢复、其他入口改名）后同步输入框；
  // 本页保存成功后 setName(trimmed) 已同步，effect 不会重复覆盖。
  useEffect(() => {
    if (currentAccount?.name) {
      setName(currentAccount.name);
    }
  }, [currentAccount?.name]);

  const handleCopyId = async () => {
    if (!currentAccount) return;
    try {
      await navigator.clipboard.writeText(currentAccount.id);
      showToast({
        type: 'success',
        message: t('settings:account_id_copied', { defaultValue: 'Account ID copied' }),
      });
    } catch {
      // 剪贴板不可用时静默降级（桌面 WebView 一般可用）
    }
  };

  const handleSave = async () => {
    if (!currentAccount) return;
    const trimmed = name.trim();
    if (!trimmed) {
      showToast({ type: 'error', message: t('common:account_name_required') });
      return;
    }
    setSaving(true);
    try {
      await invoke('vault_rename_account', {
        accountId: currentAccount.id,
        newName: trimmed,
      });
      showToast({
        type: 'success',
        message: t('settings:account_rename_success', { defaultValue: 'Account name updated' }),
      });
      // 刷新 authStore 中的账户名，首页/设置页等处即时生效
      await refreshCurrentAccount();
      setName(trimmed);
    } catch (e) {
      onError(e, t('settings:account_rename_failed', { defaultValue: 'Failed to rename account' }));
    } finally {
      setSaving(false);
    }
  };

  return (
    <PageShell title={t('settings:items.account_management')} onBack={() => navigate('/settings')}>
      <PageContainer variant="form" gap="default">
        <Card>
          <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, marginBottom: 4 }}>
            {t('settings:account_id', { defaultValue: 'Account ID' })}
          </h3>
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              marginBottom: 12,
            }}
          >
            {t('settings:account_id_hint', {
              defaultValue: 'The account ID identifies your vault and cannot be changed after creation.',
            })}
          </p>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <div
              style={{
                flex: 1,
                minWidth: 0,
                fontFamily: 'var(--font-mono, monospace)',
                fontSize: 'var(--text-body-sm)',
                padding: '8px 10px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {currentAccount?.id ?? ''}
            </div>
            <Button
              variant="secondary"
              size="sm"
              onClick={handleCopyId}
              disabled={!currentAccount}
            >
              <Copy size={ICON_SIZE.sm} /> {t('common:copy', { defaultValue: 'Copy' })}
            </Button>
          </div>
        </Card>

        <Card>
          <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, marginBottom: 4 }}>
            {t('settings:account_name', { defaultValue: 'Account Name' })}
          </h3>
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              marginBottom: 12,
            }}
          >
            {t('settings:account_name_hint', {
              defaultValue: 'Rename this account. The name is used on the login screen and in exported documents.',
            })}
          </p>
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <div style={{ flex: 1 }}>
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={t('settings:account_name', { defaultValue: 'Account Name' })}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleSave();
                }}
              />
            </div>
            <Button
              variant="primary"
              size="md"
              loading={saving}
              disabled={!name.trim() || name.trim() === currentAccount?.name}
              onClick={handleSave}
            >
              {t('common:save', { defaultValue: 'Save' })}
            </Button>
          </div>
        </Card>
      </PageContainer>
    </PageShell>
  );
}
