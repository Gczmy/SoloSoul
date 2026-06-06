import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';

export function BootstrapPage() {
  const navigate = useNavigate();
  const { bootstrap, isLoading, error } = useAuthStore();
  const [accountName, setAccountName] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const { t } = useTranslation(['auth', 'common']);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (password !== confirm) return;
    await bootstrap(accountName, password);
    navigate('/');
  };

  return (
    <div
      style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh' }}
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
        <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
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
          />
          <SecurePasswordInput
            label={t('auth:confirm_password')}
            value={confirm}
            onChange={(v) => setConfirm(v)}
            placeholder={t('common:password_placeholder')}
          />
          {error && (
            <div style={{ color: '#e74c3c', fontSize: 13 }}>
              {error.toLowerCase().includes('password') || error.toLowerCase().includes('invalid')
                ? t('auth:incorrect_password')
                : error.toLowerCase().includes('required')
                  ? t('auth:password_required')
                  : error}
            </div>
          )}
          <Button type="submit" loading={isLoading} style={{ width: '100%', marginTop: 8 }}>
            {t('auth:create_account')}
          </Button>
        </form>
      </div>
    </div>
  );
}
