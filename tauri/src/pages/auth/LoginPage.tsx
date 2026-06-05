import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuthStore } from '@/stores/authStore';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';

export function LoginPage() {
  const navigate = useNavigate();
  const {
    login, checkHasAccount, listAccounts,
    hasAccount, isAuthenticated, isLoading, error,
    accounts, clearError,
  } = useAuthStore();
  const [selectedAccountId, setSelectedAccountId] = useState('');
  const [password, setPassword] = useState('');

  useEffect(() => {
    checkHasAccount().then(() => listAccounts());
  }, []);

  useEffect(() => {
    if (hasAccount === false) navigate('/bootstrap');
    if (isAuthenticated) navigate('/');
  }, [hasAccount, isAuthenticated, navigate]);

  // Auto-select first account
  useEffect(() => {
    if (accounts.length > 0 && !selectedAccountId) {
      setSelectedAccountId(accounts[0].id);
    }
  }, [accounts, selectedAccountId]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedAccountId) return;
    clearError();
    await login(selectedAccountId, password);
  };

  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh' }}>
      <div style={{
        background: 'var(--bg-elevated)', borderRadius: 16, padding: 32,
        width: 360, boxShadow: '0 8px 32px rgba(0,0,0,0.08)', textAlign: 'center',
      }}>
        <div style={{
          width: 48, height: 48, borderRadius: 12,
          background: 'linear-gradient(135deg, var(--accent-primary), var(--accent-warm))',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          color: 'white', fontWeight: 700, fontSize: 22, margin: '0 auto 16px',
        }}>S</div>
        <h1 style={{ fontSize: 20, fontWeight: 600, marginBottom: 4 }}>SoloSoul</h1>
        <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 24 }}>
          Unlock your vault to continue
        </p>
        <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          {accounts.length > 1 && (
            <select
              value={selectedAccountId}
              onChange={(e) => setSelectedAccountId(e.target.value)}
              style={{
                padding: '10px 14px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                background: 'var(--bg-elevated)', color: 'var(--text-primary)',
                fontSize: 14, fontFamily: 'inherit', outline: 'none',
              }}
            >
              {accounts.map((acc) => (
                <option key={acc.id} value={acc.id}>
                  {acc.name}
                </option>
              ))}
            </select>
          )}
          <Input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="Enter master password"
          />
          {error && <div style={{ color: '#e74c3c', fontSize: 13 }}>{error}</div>}
          <Button type="submit" loading={isLoading} style={{ width: '100%' }}>
            Unlock
          </Button>
        </form>
      </div>
    </div>
  );
}
