import { useState, type CSSProperties } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import i18next from 'i18next';
import { useAuthStore } from '@/stores/authStore';
import { useApplyThemeFromSettings } from '@/hooks/useApplyThemeFromSettings';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { AlertTriangle } from 'lucide-react';
import { ICON_SIZE, MIN_PASSWORD_LENGTH } from '@/lib/constants';

export function BootstrapPage() {
  useApplyThemeFromSettings();
  const navigate = useNavigate();
  const { bootstrap, isLoading, error } = useAuthStore();
  const [accountName, setAccountName] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [passwordHint, setPasswordHint] = useState('');
  // 空字段/长度/一致性校验错误（按优先级：账户名称 > 主密码未输入 > 主密码不符合要求 > 确认密码未输入 > 两次密码不一致）
  const [accountNameError, setAccountNameError] = useState<string | null>(null);
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [confirmError, setConfirmError] = useState<string | null>(null);
  const [searchParams] = useSearchParams();
  const isCreateMode = searchParams.get('mode') === 'create';
  const { t } = useTranslation(['auth', 'common', 'settings']);

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    // 校验优先级：账户名称未输入 > 主密码未输入 > 主密码不符合要求 > 确认密码未输入 > 两次密码不一致
    if (!accountName.trim()) {
      setAccountNameError(t('auth:account_name_required'));
      return;
    }
    if (!password) {
      setPasswordError(t('auth:master_password_required'));
      return;
    }
    if (password.length < MIN_PASSWORD_LENGTH) {
      // 密码长度不足：抖动主密码输入框 + 红边 + 红字提示，不跳转
      setPasswordError(t('auth:password_too_short'));
      return;
    }
    if (!confirm) {
      setConfirmError(t('auth:confirm_password_required'));
      return;
    }
    if (password !== confirm) {
      // 两次密码不一致：抖动确认密码输入框 + 红边 + 红字提示，不跳转
      setConfirmError(t('settings:password_mismatch'));
      return;
    }
    // Use the language currently active in i18next (detected via Rust IPC),
    // NOT navigator.language (which is unreliable on Windows WebView2)
    const locale = i18next.language?.startsWith('zh') ? 'zh' : 'en';
    await bootstrap(accountName.trim(), password, locale, passwordHint || undefined);
    // 仅创建成功（store 无错误）时才跳转，失败时停留在卡片展示后端错误
    const state = useAuthStore.getState();
    if (!state.error) {
      navigate('/');
    }
  };

  return (
    <div
      style={
        {
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100vh',
        } as CSSProperties
      }
    >
      <div
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          padding: 32,
          width: '100%',
          maxWidth: 400,
          boxShadow: '0 8px 32px rgba(0,0,0,0.08)',
          margin: '0 16px',
        }}
      >
        <h1 style={{ fontSize: 'var(--text-xl)', fontWeight: 600, marginBottom: 8 }}>
          {t('auth:bootstrap_title')}
        </h1>
        <p
          style={{ fontSize: 'var(--text-body)', color: 'var(--text-secondary)', marginBottom: 24 }}
        >
          {t('auth:bootstrap_subtitle')}
        </p>
        <form
          onSubmit={handleSubmit}
          autoComplete="off"
          style={{ display: 'flex', flexDirection: 'column', gap: 16 }}
        >
          <Input
            label={t('auth:account_name')}
            value={accountName}
            onChange={(e) => {
              setAccountName(e.target.value);
              if (accountNameError) setAccountNameError(null);
            }}
            placeholder={t('auth:account_name')}
            error={accountNameError ?? undefined}
          />
          <SecurePasswordInput
            label={t('auth:master_password')}
            value={password}
            onChange={(v) => {
              setPassword(v);
              if (passwordError) setPasswordError(null);
            }}
            placeholder={t('common:password_placeholder')}
            autoComplete="new-password"
            onEnter={handleSubmit}
            error={passwordError}
          />
          <div
            style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)', marginTop: -12 }}
          >
            {t('auth:password_rule_hint')}
          </div>
          <SecurePasswordInput
            label={t('auth:confirm_password')}
            value={confirm}
            onChange={(v) => {
              setConfirm(v);
              if (confirmError) setConfirmError(null);
            }}
            placeholder={t('common:password_placeholder')}
            autoComplete="new-password"
            onEnter={handleSubmit}
            error={confirmError}
          />
          <Input
            label={t('auth:password_hint')}
            value={passwordHint}
            onChange={(e) => setPasswordHint(e.target.value)}
            placeholder={t('auth:password_hint_placeholder')}
          />
          {error && (
            <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)' }}>
              {error.toLowerCase().includes('8 characters') || error.toLowerCase().includes('至少')
                ? t('auth:password_too_short')
                : error.toLowerCase().includes('password') ||
                    error.toLowerCase().includes('invalid')
                  ? t('auth:incorrect_password')
                  : error.toLowerCase().includes('required')
                    ? t('auth:password_required')
                    : error}
            </div>
          )}
          <div
            style={{
              display: 'flex',
              alignItems: 'flex-start',
              gap: 8,
              padding: 10,
              borderRadius: 8,
              background: 'rgba(212, 133, 10, 0.10)',
              border: '1px solid rgba(212, 133, 10, 0.25)',
              color: '#D4850A',
              fontSize: 'var(--text-caption)',
              lineHeight: 1.4,
              textAlign: 'left',
            }}
          >
            <AlertTriangle size={ICON_SIZE.md} style={{ flexShrink: 0, marginTop: 1 }} />
            {t('auth:master_password_warning')}
          </div>
          <Button type="submit" loading={isLoading} style={{ width: '100%', marginTop: 8 }}>
            {t('auth:create_account')}
          </Button>
        </form>

        {isCreateMode && (
          <div
            style={{
              display: 'flex',
              justifyContent: 'center',
              marginTop: 16,
            }}
          >
            <button
              type="button"
              onClick={() => navigate('/login')}
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-tertiary)',
                background: 'transparent',
                border: 'none',
                padding: '6px 12px',
                cursor: 'pointer',
                fontFamily: 'inherit',
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = 'var(--text-tertiary)';
              }}
            >
              {t('common:back_to_login_link')}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
