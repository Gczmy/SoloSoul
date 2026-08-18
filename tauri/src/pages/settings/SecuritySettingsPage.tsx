import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { PageShell } from '@/components/layout/PageShell';
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

  // P022: useShallow 字段级选择——避免 store 无关字段（isLoading/customPages 等）翻转时整页重渲染
  const { settings, updateSetting } = useSettingsStore(
    useShallow((s) => ({ settings: s.settings, updateSetting: s.updateSetting })),
  );

  return (
    <PageShell title={t('settings:items.security_settings')} onBack={() => navigate('/settings')}>
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

          {/* 切后台锁定开关 */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginTop: 16,
              paddingTop: 16,
              borderTop: '1px solid var(--border-subtle)',
            }}
          >
            <div>
              <span style={{ fontSize: 'var(--text-sm)', display: 'block' }}>
                {t('settings:auto_lock_on_background')}
              </span>
              <span
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-secondary)',
                  display: 'block',
                  marginTop: 2,
                }}
              >
                {t('settings:auto_lock_on_background_desc')}
              </span>
            </div>
            <label
              style={{
                position: 'relative',
                display: 'inline-flex',
                alignItems: 'center',
                cursor: 'pointer',
              }}
            >
              <input
                type="checkbox"
                checked={settings.autoLockOnBackground}
                onChange={(e) => {
                  const value = e.target.checked;
                  if (currentAccount?.id) {
                    updateSetting(currentAccount.id, 'autoLockOnBackground', value);
                  }
                }}
                style={{ position: 'absolute', opacity: 0, width: 0, height: 0 }}
              />
              <span
                style={{
                  width: 40,
                  height: 22,
                  borderRadius: 11,
                  background: settings.autoLockOnBackground
                    ? 'var(--accent-primary)'
                    : 'var(--border-subtle)',
                  transition: 'background 0.2s ease',
                  position: 'relative',
                }}
              >
                <span
                  style={{
                    position: 'absolute',
                    top: 2,
                    left: settings.autoLockOnBackground ? 20 : 2,
                    width: 18,
                    height: 18,
                    borderRadius: '50%',
                    background: '#fff',
                    transition: 'left 0.2s ease',
                    boxShadow: '0 1px 3px rgba(0,0,0,0.15)',
                  }}
                />
              </span>
            </label>
          </div>

          {/* 自动锁定通知开关 */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginTop: 16,
              paddingTop: 16,
              borderTop: '1px solid var(--border-subtle)',
            }}
          >
            <div>
              <span style={{ fontSize: 'var(--text-sm)', display: 'block' }}>
                {t('settings:auto_lock_notification')}
              </span>
              <span
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-secondary)',
                  display: 'block',
                  marginTop: 2,
                }}
              >
                {t('settings:auto_lock_notification_desc')}
              </span>
            </div>
            <label
              style={{
                position: 'relative',
                display: 'inline-flex',
                alignItems: 'center',
                cursor: 'pointer',
              }}
            >
              <input
                type="checkbox"
                checked={settings.autoLockNotificationEnabled}
                onChange={(e) => {
                  const value = e.target.checked;
                  if (currentAccount?.id) {
                    updateSetting(currentAccount.id, 'autoLockNotificationEnabled', value);
                  }
                }}
                style={{ position: 'absolute', opacity: 0, width: 0, height: 0 }}
              />
              <span
                style={{
                  width: 40,
                  height: 22,
                  borderRadius: 11,
                  background: settings.autoLockNotificationEnabled
                    ? 'var(--accent-primary)'
                    : 'var(--border-subtle)',
                  transition: 'background 0.2s ease',
                  position: 'relative',
                }}
              >
                <span
                  style={{
                    position: 'absolute',
                    top: 2,
                    left: settings.autoLockNotificationEnabled ? 20 : 2,
                    width: 18,
                    height: 18,
                    borderRadius: '50%',
                    background: '#fff',
                    transition: 'left 0.2s ease',
                    boxShadow: '0 1px 3px rgba(0,0,0,0.15)',
                  }}
                />
              </span>
            </label>
          </div>
        </Card>

        <BiometricSection accountId={currentAccount?.id || ''} />

        <PinSection accountId={currentAccount?.id || ''} />

        <PasswordChangeForm accountId={currentAccount?.id} />
      </PageContainer>
    </PageShell>
  );
}
