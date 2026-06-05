import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
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
        <h1 style={{ fontSize: 24, fontWeight: 600, marginBottom: 8 }}>Welcome to SoloSoul</h1>
        <p style={{ fontSize: 14, color: 'var(--text-secondary)', marginBottom: 24 }}>
          First time setup — create a master password to protect your data.
        </p>
        <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <Input
            label="Account Name"
            value={accountName}
            onChange={(e) => setAccountName(e.target.value)}
            placeholder="e.g. Personal Vault"
          />
          <SecurePasswordInput
            label="Master Password"
            value={password}
            onChange={(v) => setPassword(v)}
            placeholder="At least 8 characters"
          />
          <SecurePasswordInput
            label="Confirm Password"
            value={confirm}
            onChange={(v) => setConfirm(v)}
            placeholder="Repeat password"
          />
          {error && <div style={{ color: '#e74c3c', fontSize: 13 }}>{error}</div>}
          <Button type="submit" loading={isLoading} style={{ width: '100%', marginTop: 8 }}>
            Create Account
          </Button>
        </form>
      </div>
    </div>
  );
}
