import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { BiometricSection } from '@/components/settings/BiometricSection';
import { PinSection } from '@/components/settings/PinSection';
import { PasswordChangeForm } from '@/components/settings/PasswordChangeForm';
import { AlertTriangle } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

export function SecuritySettingsPage() {
  const navigate = useNavigate();
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const { t } = useTranslation(['settings', 'common']);

  const { settings, updateSetting } = useSettingsStore();

  return (
    <AppShell title={t('settings:items.security_settings')} onBack={() => navigate('/settings')}>
      <PageContainer variant="form" gap="default">
        <Card>
          <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, marginBottom: 4 }}>
            {t('settings:auto_lock')}
          </h3>
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              marginBottom: 12,
            }}
          >
            {t('settings:auto_lock_description')}
          </p>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ fontSize: 'var(--text-sm)' }}>{t('settings:auto_lock')}</span>
            <select
              value={settings.autoLockTimeoutMinutes}
              onChange={(e) => {
                const value = parseInt(e.target.value, 10);
                if (currentAccount?.id) {
                  updateSetting(currentAccount.id, 'autoLockTimeoutMinutes', value);
                }
              }}
              style={{
                padding: '6px 10px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                fontFamily: 'inherit',
                fontSize: 'var(--text-body-sm)',
                transition: 'border-color 0.15s ease',
              }}
            >
              <option value="1">
                1 {t('common:minute', { ns: 'common', defaultValue: 'minute' })}
              </option>
              <option value="5">
                5 {t('common:minutes', { ns: 'common', defaultValue: 'minutes' })}
              </option>
              <option value="15">
                15 {t('common:minutes', { ns: 'common', defaultValue: 'minutes' })}
              </option>
              <option value="30">
                30 {t('common:minutes', { ns: 'common', defaultValue: 'minutes' })}
              </option>
              <option value="0">
                {t('common:never', { ns: 'common', defaultValue: 'Never' })}
              </option>
            </select>
          </div>
          {settings.autoLockTimeoutMinutes === 0 && (
            <div
              style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: 8,
                padding: 10,
                borderRadius: 8,
                marginTop: 12,
                background: 'rgba(212, 133, 10, 0.10)',
                border: '1px solid rgba(212, 133, 10, 0.25)',
                color: '#D4850A',
                fontSize: 'var(--text-caption)',
                lineHeight: 1.4,
              }}
            >
              <AlertTriangle size={ICON_SIZE.md} style={{ flexShrink: 0, marginTop: 1 }} />
              {t('settings:auto_lock_never_warning')}
            </div>
          )}
        </Card>

        <BiometricSection accountId={currentAccount?.id || ''} />

        <PinSection accountId={currentAccount?.id || ''} />

        <PasswordChangeForm accountId={currentAccount?.id} />
      </PageContainer>
    </AppShell>
  );
}
