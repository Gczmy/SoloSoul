import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import { Info, Fingerprint, ShieldCheck, AlertTriangle } from 'lucide-react';

export function SecuritySettingsPage() {
  const navigate = useNavigate();
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const { onError, onSuccess } = useToastError();
  const { t } = useTranslation(['settings', 'common']);

  const [oldPw, setOldPw] = useState('');
  const [newPw, setNewPw] = useState('');
  const [confirmPw, setConfirmPw] = useState('');
  const [hint, setHint] = useState('');
  const [hintCleared, setHintCleared] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Biometric state
  const biometricEnabled = useSettingsStore((s) => s.settings.biometricEnabled);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string } | null>(null);
  const [bioLoading, setBioLoading] = useState(false);
  const [showBioPwDialog, setShowBioPwDialog] = useState(false);
  const [bioPw, setBioPw] = useState('');
  const [bioAction, setBioAction] = useState<'enable' | 'disable' | null>(null);

  useEffect(() => {
    invoke<{ available: boolean; biometryType?: string }>('biometric_check_availability', { accountId: currentAccount?.id })
      .then(setBioAvailable).catch(() => setBioAvailable({ available: false }));
  }, [currentAccount?.id]);

  const biometryType = bioAvailable?.biometryType === 'touchId' ? 'Touch ID' : bioAvailable?.biometryType === 'faceId' ? 'Face ID' : 'Touch ID';

  const handleBioToggle = () => {
    if (biometricEnabled) {
      setBioAction('disable');
    } else {
      setBioAction('enable');
    }
    setBioPw('');
    setShowBioPwDialog(true);
  };

  const handleBioConfirm = async () => {
    if (!bioPw || !currentAccount) return;
    setBioLoading(true);
    try {
      if (bioAction === 'enable') {
        await invoke('biometric_save_credential', { accountId: currentAccount.id, password: bioPw, silent: false, location: 'settings_page', action: 'enable' });
        await useSettingsStore.getState().updateSetting(currentAccount.id, 'biometricEnabled', true);
        onSuccess(t('settings:biometric_enabled_toast', { type: biometryType }));
      } else {
        await invoke('biometric_delete_credential', { accountId: currentAccount.id, password: bioPw, location: 'settings_page', action: 'disable' });
        await useSettingsStore.getState().updateSetting(currentAccount.id, 'biometricEnabled', false);
        onSuccess(t('settings:biometric_disabled_toast', { type: biometryType }));
      }
      setShowBioPwDialog(false);
    } catch (e) {
      const msg = String(e);
      if (msg.includes('Invalid password') || msg.includes('incorrect')) {
        setError('密码错误');
      } else {
        onError(e, t('common:error'));
      }
    } finally {
      setBioLoading(false);
    }
  };

  const handleBioTest = async () => {
    if (!currentAccount) return;
    try {
      await invoke('biometric_test', { accountId: currentAccount.id });
      onSuccess(t('settings:biometric_test_success', { type: biometryType }));
    } catch (e) {
      onError(e, t('settings:biometric_test_failed', { type: biometryType }));
    }
  };

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
        await invoke('change_password', {
          accountId: currentAccount?.id || '',
          oldPassword: oldPw,
          newPassword: newPw,
        });
        if (hint.trim() || hintCleared) {
          // Password already changed — use the new password for vault_update_hint
          await invoke('vault_update_hint', { accountId: currentAccount?.id || '', password: newPw, hint: hint.trim() });
        }
        onSuccess(t('settings:password_updated'));
      } else {
        // 10.3 — 仅修改密码提示（需要验证当前密码）
        if (!oldPw.trim()) {
          setError(t('common:password_required'));
          setLoading(false);
          return;
        }
        if (hint.trim() || hintCleared) {
          // hint.trim() 空但 hintCleared=true → 用户显式点击清除按钮，清除提示词
          await invoke('vault_update_hint', { accountId: currentAccount?.id || '', password: oldPw, hint: hint.trim() });
        }
        onSuccess(t('settings:hint_updated'));
      }

      // Refresh accounts to sync updated hint to LoginPage
      await useAuthStore.getState().listAccounts();

      setOldPw('');
      setNewPw('');
      setConfirmPw('');
      setHint('');
      setHintCleared(false);
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

  const shouldDisableSave = !oldPw || loading;

  return (
    <AppShell title={t('settings:items.security_settings')} onBack={() => navigate('/settings')}>
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
                color: 'var(--text-primary)',
                fontFamily: 'inherit',
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

        {bioAvailable !== null && bioAvailable.available && (
          <Card>
            <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 4, display: 'flex', alignItems: 'center', gap: 8 }}>
              <Fingerprint size={18} />
              {t('settings:biometric_title')}
            </h3>
            <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 12 }}>
              {t('settings:biometric_desc', { type: biometryType })}
            </p>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 14 }}>{t('settings:biometric_toggle_label', { type: biometryType })}</span>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                {biometricEnabled && (
                  <Button variant="secondary" size="sm" onClick={handleBioTest}>
                    <ShieldCheck size={14} style={{ marginRight: 4 }} />
                    {t('settings:biometric_test_button', { type: biometryType })}
                  </Button>
                )}
                <label style={{ position: 'relative', display: 'inline-block', width: 44, height: 24, cursor: 'pointer' }}>
                  <input type="checkbox" checked={biometricEnabled} onChange={handleBioToggle} style={{ opacity: 0, width: 0, height: 0 }} />
                  <span style={{
                    position: 'absolute', inset: 0,
                    background: biometricEnabled ? 'var(--accent-primary)' : 'var(--border-subtle)',
                    borderRadius: 12, transition: '0.2s',
                  }} />
                  <span style={{
                    position: 'absolute', top: 2, left: biometricEnabled ? 22 : 2,
                    width: 20, height: 20, borderRadius: '50%',
                    background: 'white', transition: '0.2s', boxShadow: '0 1px 3px rgba(0,0,0,0.2)',
                  }} />
                </label>
              </div>
            </div>
          </Card>
        )}

        <Card>
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 12 }}>
            {t('settings:change_password')}
          </h3>
          <div style={{
            display: 'flex', alignItems: 'flex-start', gap: 8,
            padding: 10, borderRadius: 8, marginBottom: 16,
            background: 'rgba(212, 133, 10, 0.10)', border: '1px solid rgba(212, 133, 10, 0.25)',
            color: '#D4850A', fontSize: 12, lineHeight: 1.4,
          }}>
            <AlertTriangle size={16} style={{ flexShrink: 0, marginTop: 1 }} />
            {t('settings:master_password_warning')}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            {/* 10.1 — 当前密码 */}
            <SecurePasswordInput
              label={t('common:current_password')}
              value={oldPw}
              onChange={(v) => { setOldPw(v); setError(null); }}
              placeholder={t('common:password_placeholder')}
              autoComplete="current-password"
              hint={(currentAccount as { passwordHint?: string } | null)?.passwordHint || null}
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
              placeholder={t('common:new_password')}
              autoComplete="new-password"
            />

            {/* 10.1 — 确认新密码 */}
            <SecurePasswordInput
              label={t('common:confirm_password')}
              showHintButton={false}
              value={confirmPw}
              onChange={(v) => { setConfirmPw(v); setError(null); }}
              placeholder={t('common:confirm_password')}
              autoComplete="new-password"
            />

            {/* 10.1 — 密码提示（始终明文可见，普通文本输入框） */}
            <div>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
                <label style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-secondary)' }}>
                  {t('common:password_hint')}
                </label>
                <button
                  type="button"
                  onClick={() => {
                    if (hintCleared) {
                      setHintCleared(false); // undo clear
                    } else {
                      setHint('');
                      setHintCleared(true);
                      setError(null);
                    }
                  }}
                  style={{
                    padding: '3px 10px', borderRadius: 4, border: '1px solid',
                    borderColor: hintCleared ? '#e74c3c' : 'var(--border-subtle)',
                    background: hintCleared ? '#e74c3c' : 'transparent',
                    cursor: 'pointer', fontSize: 11, fontWeight: 500,
                    color: hintCleared ? 'white' : 'var(--text-tertiary)',
                    transition: 'all 0.15s ease',
                  }}
                >
                  {hintCleared ? t('common:undo') : t('common:clear_hint')}
                </button>
              </div>
              <input
                type="text"
                value={hint}
                onChange={(e) => { setHint(e.target.value); setHintCleared(false); setError(null); }}
                disabled={hintCleared}
                placeholder={t('common:optional')}
                style={{
                  width: '100%', padding: '10px 14px', fontSize: 14,
                  border: '1px solid', borderRadius: 8,
                  borderColor: hintCleared ? '#e74c3c' : 'var(--border-subtle)',
                  background: hintCleared ? 'rgba(231,76,60,0.05)' : 'transparent',
                  color: hintCleared ? '#e74c3c' : 'var(--text-primary)',
                  fontFamily: 'inherit', outline: 'none',
                  opacity: hintCleared ? 0.6 : 1,
                }}
              />
              {hintCleared && (
                <div style={{ marginTop: 6, fontSize: 12, color: '#e74c3c', lineHeight: 1.4 }}>
                  {t('common:clear_hint_warning')}
                </div>
              )}
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

      {/* Biometric password verification dialog */}
      {showBioPwDialog && (
        <div style={{ position: 'fixed', inset: 0, zIndex: 2000, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(0,0,0,0.45)', backdropFilter: 'blur(6px)' }}>
          <div style={{ background: 'var(--bg-elevated)',
                color: 'var(--text-primary)',
                fontFamily: 'inherit', borderRadius: 16, padding: '28px 32px', maxWidth: 380, width: '90%', boxShadow: 'var(--shadow-lg)', border: '1px solid var(--border-subtle)' }}>
            <h3 style={{ fontSize: 17, fontWeight: 600, marginBottom: 12, display: 'flex', alignItems: 'center', gap: 8 }}>
              <Fingerprint size={20} />
              {bioAction === 'enable'
                ? t('settings:biometric_enable_prompt', { type: biometryType })
                : t('settings:biometric_disable_prompt', { type: biometryType })}
            </h3>
            <SecurePasswordInput
              label={t('common:current_password')}
              value={bioPw}
              onChange={(v) => { setBioPw(v); setError(null); }}
              placeholder={t('common:password_placeholder')}
              autoComplete="current-password"
              showHintButton={true}
              hint={(currentAccount as { passwordHint?: string } | null)?.passwordHint || null}
            />
            {error && (
              <div style={{ color: '#dc2626', fontSize: 13, padding: '4px 0', marginTop: 8 }}>{error}</div>
            )}
            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 16 }}>
              <Button variant="secondary" onClick={() => { setShowBioPwDialog(false); setError(null); }}>{t('common:cancel')}</Button>
              <Button onClick={handleBioConfirm} loading={bioLoading} disabled={!bioPw}>{t('common:confirm')}</Button>
            </div>
          </div>
        </div>
      )}
    </AppShell>
  );
}
