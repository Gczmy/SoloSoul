import { useState, type CSSProperties } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import i18next from 'i18next';
import { useAuthStore } from '@/stores/authStore';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { AlertTriangle } from 'lucide-react';

export function BootstrapPage() {
  const navigate = useNavigate();
  const { bootstrap, isLoading, error } = useAuthStore();
  const [accountName, setAccountName] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [passwordHint, setPasswordHint] = useState('');
  const { t } = useTranslation(['auth', 'common']);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (password !== confirm) return;
    // Use the language currently active in i18next (detected via Rust IPC),
    // NOT navigator.language (which is unreliable on Windows WebView2)
    const locale = i18next.language?.startsWith('zh') ? 'zh' : 'en';
    await bootstrap(accountName, password, locale, passwordHint || undefined);
    navigate('/');
  };

  return (
    <div
      style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh' } as CSSProperties}
    >
      <div
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          padding: 32,
          width: 400,
          boxShadow: '0 8px 32px rgba(0,0,0,0.08)',
        }}
      >
        <h1 style={{ fontSize: 24, fontWeight: 600, marginBottom: 8 }}>{t('auth:bootstrap_title')}</h1>
        <p style={{ fontSize: 14, color: 'var(--text-secondary)', marginBottom: 24 }}>
          {t('auth:bootstrap_subtitle')}
        </p>
        <form onSubmit={handleSubmit} autoComplete="off" style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <Input
            label={t('auth:account_name')}
            value={accountName}
            onChange={(e) => setAccountName(e.target.value)}
            placeholder={t('auth:account_name')}
          />
          <SecurePasswordInput
            label={t('auth:master_password')}
            value={password}
            onChange={(v) => setPassword(v)}
            placeholder={t('common:password_placeholder')}
            autoComplete="new-password"
          />
          <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: -12 }}>
            {t('auth:password_rule_hint')}
          </div>
          <SecurePasswordInput
            label={t('auth:confirm_password')}
            value={confirm}
            onChange={(v) => setConfirm(v)}
            placeholder={t('common:password_placeholder')}
            autoComplete="new-password"
          />
          <Input
            label={t('auth:password_hint')}
            value={passwordHint}
            onChange={(e) => setPasswordHint(e.target.value)}
            placeholder={t('auth:password_hint_placeholder')}
          />
          {error && (
            <div style={{ color: '#e74c3c', fontSize: 13 }}>
              {error.toLowerCase().includes('password') || error.toLowerCase().includes('invalid')
                ? t('auth:incorrect_password')
                : error.toLowerCase().includes('required')
                  ? t('auth:password_required')
                  : error.toLowerCase().includes('8 characters')
                    ? t('auth:password_too_short')
                    : error}
            </div>
          )}
          <div style={{
            display: 'flex', alignItems: 'flex-start', gap: 8,
            padding: 10, borderRadius: 8,
            background: 'rgba(212, 133, 10, 0.10)', border: '1px solid rgba(212, 133, 10, 0.25)',
            color: '#D4850A', fontSize: 12, lineHeight: 1.4, textAlign: 'left',
          }}>
            <AlertTriangle size={16} style={{ flexShrink: 0, marginTop: 1 }} />
            {t('auth:master_password_warning')}
          </div>
          <Button type="submit" loading={isLoading} style={{ width: '100%', marginTop: 8 }}>
            {t('auth:create_account')}
          </Button>
        </form>
      </div>
    </div>
  );
}
