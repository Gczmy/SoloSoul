import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import { Info, AlertTriangle } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

interface PasswordChangeFormProps {
  accountId?: string;
}

export function PasswordChangeForm({ accountId }: PasswordChangeFormProps) {
  const { t } = useTranslation(['settings', 'common']);
  const { onError, onSuccess } = useToastError();

  const currentAccount = useAuthStore((s) => s.currentAccount);

  const [oldPw, setOldPw] = useState('');
  const [newPw, setNewPw] = useState('');
  const [confirmPw, setConfirmPw] = useState('');
  const [hint, setHint] = useState('');
  const [hintCleared, setHintCleared] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 10.3 — 回填当前账户的密码提示词
  useEffect(() => {
    if (currentAccount?.passwordHint !== undefined) {
      setHint(currentAccount.passwordHint);
    }
  }, [currentAccount?.passwordHint]);

  const handleChangePassword = async () => {
    setError(null);

    if (!oldPw) {
      setError(t('common:password_required', { ns: 'common' }));
      return;
    }

    const isChangingPw = newPw.length > 0 || confirmPw.length > 0;

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
        await invoke('change_password', {
          accountId: accountId || '',
          oldPassword: oldPw,
          newPassword: newPw,
        });
        if (hint.trim() || hintCleared) {
          await invoke('vault_update_hint', {
            accountId: accountId || '',
            password: newPw,
            hint: hint.trim(),
          });
        }
        onSuccess(t('settings:password_updated'));
      } else {
        if (!oldPw.trim()) {
          setError(t('common:password_required'));
          setLoading(false);
          return;
        }
        if (hint.trim() || hintCleared) {
          await invoke('vault_update_hint', {
            accountId: accountId || '',
            password: oldPw,
            hint: hint.trim(),
          });
        }
        onSuccess(t('settings:hint_updated'));
      }

      await useAuthStore.getState().refreshCurrentAccount();

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
    <Card>
      <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, marginBottom: 12 }}>
        {t('settings:change_password')}
      </h3>
      <div
        style={{
          display: 'flex',
          alignItems: 'flex-start',
          gap: 8,
          padding: 10,
          borderRadius: 8,
          marginBottom: 16,
          background: 'rgba(212, 133, 10, 0.10)',
          border: '1px solid rgba(212, 133, 10, 0.25)',
          color: '#D4850A',
          fontSize: 'var(--text-caption)',
          lineHeight: 1.4,
        }}
      >
        <AlertTriangle size={ICON_SIZE.md} style={{ flexShrink: 0, marginTop: 1 }} />
        {t('settings:master_password_warning')}
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
        <SecurePasswordInput
          label={t('common:current_password')}
          value={oldPw}
          onChange={(v) => {
            setOldPw(v);
            setError(null);
          }}
          placeholder={t('common:password_placeholder')}
          autoComplete="current-password"
          hint={currentAccount?.passwordHint || null}
          onEnter={handleChangePassword}
        />

        <SecurePasswordInput
          label={
            <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              {t('common:new_password')}
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 3,
                  fontSize: 'var(--text-badge)',
                  color: 'var(--text-tertiary)',
                  fontWeight: 400,
                }}
              >
                <Info size={ICON_SIZE.xs} />
                {t('common:password_length_requirement')}
              </span>
            </span>
          }
          showHintButton={false}
          value={newPw}
          onChange={(v) => {
            setNewPw(v);
            setError(null);
          }}
          placeholder={t('common:new_password')}
          autoComplete="new-password"
        />

        <SecurePasswordInput
          label={t('common:confirm_password')}
          showHintButton={false}
          value={confirmPw}
          onChange={(v) => {
            setConfirmPw(v);
            setError(null);
          }}
          placeholder={t('common:confirm_password')}
          autoComplete="new-password"
        />

        {/* Password hint input */}
        <div>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginBottom: 4,
            }}
          >
            <label
              style={{
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                color: 'var(--text-secondary)',
              }}
            >
              {t('common:password_hint')}
            </label>
            <button
              type="button"
              onClick={() => {
                if (hintCleared) {
                  setHintCleared(false);
                } else {
                  setHint('');
                  setHintCleared(true);
                  setError(null);
                }
              }}
              className={hintCleared ? 'interactive-danger-solid' : 'interactive-toolbar'}
              style={{
                padding: '3px 10px',
                borderRadius: 6,
                borderWidth: 1,
                borderStyle: 'solid',
                cursor: 'pointer',
                fontSize: 'var(--text-badge)',
                fontWeight: 500,
              }}
            >
              {hintCleared ? t('common:undo') : t('common:clear_hint')}
            </button>
          </div>
          <input
            type="text"
            value={hint}
            onChange={(e) => {
              setHint(e.target.value);
              setHintCleared(false);
              setError(null);
            }}
            disabled={hintCleared}
            placeholder={t('common:optional')}
            style={{
              width: '100%',
              padding: '10px 14px',
              fontSize: 'var(--text-body)',
              border: '1px solid',
              borderRadius: 8,
              borderColor: hintCleared ? '#e74c3c' : 'var(--border-subtle)',
              background: hintCleared ? 'rgba(231,76,60,0.05)' : 'transparent',
              color: hintCleared ? '#e74c3c' : 'var(--text-primary)',
              fontFamily: 'inherit',
              outline: 'none',
              opacity: hintCleared ? 0.6 : 1,
            }}
          />
          {hintCleared && (
            <div
              style={{
                marginTop: 6,
                fontSize: 'var(--text-caption)',
                color: '#e74c3c',
                lineHeight: 1.4,
              }}
            >
              {t('common:clear_hint_warning')}
            </div>
          )}
        </div>

        {error && (
          <div style={{ color: '#dc2626', fontSize: 'var(--text-body-sm)', padding: '4px 0' }}>
            {error}
          </div>
        )}

        <button
          onClick={handleChangePassword}
          disabled={shouldDisableSave}
          className="interactive-toolbar"
          style={{
            padding: '8px 16px',
            borderRadius: 8,
            borderWidth: 1,
            borderStyle: 'solid',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            cursor: shouldDisableSave ? 'default' : 'pointer',
            opacity: shouldDisableSave ? 0.5 : 1,
            fontFamily: 'inherit',
            alignSelf: 'flex-end',
          }}
        >
          {loading ? t('common:loading', { defaultValue: '...' }) : t('common:save')}
        </button>
      </div>
    </Card>
  );
}
