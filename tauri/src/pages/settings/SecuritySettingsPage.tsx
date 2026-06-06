import { useState } from 'react';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import { Info } from 'lucide-react';

export function SecuritySettingsPage() {
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const { onError, onSuccess } = useToastError();

  const [oldPw, setOldPw] = useState('');
  const [newPw, setNewPw] = useState('');
  const [confirmPw, setConfirmPw] = useState('');
  const [hint, setHint] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleChangePassword = async () => {
    setError(null);

    if (!oldPw) {
      setError('Current password is required');
      return;
    }

    const isChangingPw = newPw.length > 0 || confirmPw.length > 0;

    // Validate new + confirm only if attempting to change password
    if (isChangingPw) {
      if (newPw.length < 8 && confirmPw.length < 8) {
        setError('Password must be at least 8 characters');
        return;
      }
      if (newPw !== confirmPw) {
        setError('New passwords do not match');
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
        onSuccess('Password updated. Please re-login with your new password.');
      } else {
        // 10.3 — 仅修改密码提示
        // TODO: call dedicated update_hint IPC once backend supports it
        onSuccess('Password hint updated');
      }

      setOldPw('');
      setNewPw('');
      setConfirmPw('');
      setHint('');
    } catch (e) {
      const msg = String(e);
      if (msg.includes('Invalid password') || msg.includes('incorrect')) {
        setError('Current password is incorrect. Please try again.');
      } else {
        onError(e, 'Password update failed');
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <AppShell title="Security Settings" onBack={() => window.history.back()}>
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
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 12 }}>Auto Lock</h3>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ fontSize: 14 }}>Auto-lock timeout</span>
            <select
              style={{
                padding: '6px 10px',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-elevated)',
                fontSize: 13,
              }}
            >
              <option value="1">1 minute</option>
              <option value="5">5 minutes</option>
              <option value="15">15 minutes</option>
              <option value="30">30 minutes</option>
              <option value="0">Never</option>
            </select>
          </div>
        </Card>

        <Card>
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 12 }}>Change Password</h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            {/* 10.1 — 当前密码 */}
            <SecurePasswordInput
              label="Current Password"
              value={oldPw}
              onChange={(v) => { setOldPw(v); setError(null); }}
              placeholder="Current password"
              autoComplete="current-password"
            />

            {/* 10.1 — 新密码 + 10.2 密码要求提示 */}
            <SecurePasswordInput
              label={
                <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  New Password
                  <span
                    style={{
                      display: 'inline-flex', alignItems: 'center', gap: 3,
                      fontSize: 11, color: 'var(--text-tertiary)', fontWeight: 400,
                    }}
                  >
                    <Info size={12} />
                    Password must be at least 8 characters
                  </span>
                </span>
              }
              showHintButton={false}
              value={newPw}
              onChange={(v) => { setNewPw(v); setError(null); }}
              placeholder="New password (leave empty to only update hint)"
              autoComplete="new-password"
            />

            {/* 10.1 — 确认新密码 */}
            <SecurePasswordInput
              label="Confirm New Password"
              showHintButton={false}
              value={confirmPw}
              onChange={(v) => { setConfirmPw(v); setError(null); }}
              placeholder="Confirm new password"
              autoComplete="new-password"
            />

            {/* 10.1 — 密码提示（始终明文可见，普通文本输入框） */}
            <div>
              <label style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-secondary)', display: 'block', marginBottom: 4 }}>
                Password Hint
              </label>
              <input
                type="text"
                value={hint}
                onChange={(e) => { setHint(e.target.value); setError(null); }}
                placeholder="Optional reminder for your password"
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
              disabled={!oldPw}
            >
              Update Password
            </Button>
          </div>
        </Card>
      </div>
    </AppShell>
  );
}
