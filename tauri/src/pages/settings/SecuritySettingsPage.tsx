import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import { Info } from 'lucide-react';

export function SecuritySettingsPage() {
  const navigate = useNavigate();
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const { onError, onSuccess } = useToastError();
  const { t } = useTranslation(['settings', 'common']);

  const [oldPw, setOldPw] = useState('');
  const [newPw, setNewPw] = useState('');
  const [confirmPw, setConfirmPw] = useState('');
  const [hint, setHint] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleChangePassword = async () => {
    setError(null);

    if (!oldPw) {
      setError(t('common:password_required', { ns: 'common' }));
      return;
    }

    const isChangingPw = newPw.length > 0 || confirmPw.length > 0;

    // Validate new + confirm only if attempting to change password
    if (isChangingPw) {
      if (newPw.length < 8 && confirmPw.length < 8) {
        setError(t('settings:password_too_short'));
        return;
      }
      if (newPw !== confirmPw) {
        setError(t('settings:password_mismatch'));
        return;
      }
    }

    setLoading(true);
    try {
      if (isChangingPw) {
        // 10.3 — 同时修改密码和密码提示
        await invoke('vault_change_password', {
          accountId: currentAccount?.id || '',
          oldPassword: oldPw,
          newPassword: newPw,
        });
        // TODO: update password_hint via separate IPC after backend support
        onSuccess(t('settings:password_updated'));
      } else {
        // 10.3 — 仅修改密码提示
        // TODO: call dedicated update_hint IPC once backend supports it
        onSuccess(t('settings:hint_updated'));
      }

      setOldPw('');
      setNewPw('');
      setConfirmPw('');
      setHint('');
    } catch (e) {
      const msg = String(e);
      if (msg.includes('Invalid password') || msg.includes('incorrect')) {
        setError(t('settings:current_password_incorrect'));
      } else {
        onError(e, t('common:error'));
      }
    } finally {
      setLoading(false);
    }
  };

  const allEmpty = !newPw && !confirmPw && !hint;
  const shouldDisableSave = !oldPw || loading;

  return (
    <AppShell title={t('settings:change_password')} onBack={() => navigate('/settings')}>
      <div
        style={{
          maxWidth: 480,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
        }}
      >
        <Card>
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 12 }}>
            {t('settings:auto_lock')}
          </h3>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ fontSize: 14 }}>{t('settings:auto_lock')}</span>
            <select
              style={{
                padding: '6px 10px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-elevated)',
                fontSize: 13,
              }}
            >
              <option value="1">1 {t('common:minute', { ns: 'common', defaultValue: 'minute' })}</option>
              <option value="5">5 {t('common:minutes', { ns: 'common', defaultValue: 'minutes' })}</option>
              <option value="15">15 {t('common:minutes', { ns: 'common', defaultValue: 'minutes' })}</option>
              <option value="30">30 {t('common:minutes', { ns: 'common', defaultValue: 'minutes' })}</option>
              <option value="0">{t('common:never', { ns: 'common', defaultValue: 'Never' })}</option>
            </select>
          </div>
        </Card>

        <Card>
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 12 }}>
            {t('settings:change_password')}
          </h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            {/* 10.1 — 当前密码 */}
            <SecurePasswordInput
              label={t('common:current_password')}
              value={oldPw}
              onChange={(v) => { setOldPw(v); setError(null); }}
              placeholder="common:password_placeholder"
              autoComplete="current-password"
            />

            {/* 10.1 — 新密码 + 10.2 密码要求提示 */}
            <SecurePasswordInput
              label={
                <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  {t('common:new_password')}
                  <span
                    style={{
                      display: 'inline-flex', alignItems: 'center', gap: 3,
                      fontSize: 11, color: 'var(--text-tertiary)', fontWeight: 400,
                    }}
                  >
                    <Info size={12} />
                    {t('common:password_length_requirement')}
                  </span>
                </span>
              }
              showHintButton={false}
              value={newPw}
              onChange={(v) => { setNewPw(v); setError(null); }}
              placeholder="common:new_password"
              autoComplete="new-password"
            />

            {/* 10.1 — 确认新密码 */}
            <SecurePasswordInput
              label={t('common:confirm_password')}
              showHintButton={false}
              value={confirmPw}
              onChange={(v) => { setConfirmPw(v); setError(null); }}
              placeholder="common:confirm_password"
              autoComplete="new-password"
            />

            {/* 10.1 — 密码提示（始终明文可见，普通文本输入框） */}
            <div>
              <label style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-secondary)', display: 'block', marginBottom: 4 }}>
                {t('common:password_hint')}
              </label>
              <input
                type="text"
                value={hint}
                onChange={(e) => { setHint(e.target.value); setError(null); }}
                placeholder={t('common:optional')}
                style={{
                  width: '100%', padding: '10px 14px', fontSize: 14,
                  border: '1px solid var(--border-subtle)', borderRadius: 8,
                  background: 'transparent', color: 'var(--text-primary)',
                  fontFamily: 'inherit', outline: 'none',
                }}
              />
            </div>

            {error && (
              <div style={{ color: '#dc2626', fontSize: 13, padding: '4px 0' }}>{error}</div>
            )}

            <Button
              size="sm"
              style={{ alignSelf: 'flex-start' }}
              onClick={handleChangePassword}
              loading={loading}
              disabled={shouldDisableSave}
            >
              {t('common:save')}
            </Button>
          </div>
        </Card>
      </div>
    </AppShell>
  );
}
