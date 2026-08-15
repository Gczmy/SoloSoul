import { useState, useEffect, type CSSProperties } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useShallow } from 'zustand/react/shallow';
import i18next from 'i18next';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { useApplyThemeFromSettings } from '@/hooks/useApplyThemeFromSettings';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { AlertTriangle } from 'lucide-react';
import { ICON_SIZE, MIN_PASSWORD_LENGTH } from '@/lib/constants';
import { translateRustError } from '@/lib/rustErrors';

export function BootstrapPage() {
  useApplyThemeFromSettings();
  const navigate = useNavigate();
  // P022: useShallow 字段级选择——避免 store 无关字段翻转时整页重渲染
  const { bootstrap, isLoading, error } = useAuthStore(
    useShallow((s) => ({
      bootstrap: s.bootstrap,
      isLoading: s.isLoading,
      error: s.error,
    })),
  );
  const [accountName, setAccountName] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [passwordHint, setPasswordHint] = useState('');
  // 空字段/长度/一致性校验错误（按优先级：账户名称 > 主密码未输入 > 主密码不符合要求 > 确认密码未输入 > 两次密码不一致）
  const [accountNameError, setAccountNameError] = useState<string | null>(null);
  /** 账户名重名失败自增计数：同串错误（Account name already taken）重复提交时也重新抖动。 */
  const [nameErrorTick, setNameErrorTick] = useState(0);
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [confirmError, setConfirmError] = useState<string | null>(null);

  // P034: 组件卸载时清空密码 state（向导完成导航离开 / 返回账户来源选择时缩短驻留）
  useEffect(() => {
    return () => {
      setPassword('');
      setConfirm('');
      setPasswordHint('');
    };
  }, []);
  const [searchParams] = useSearchParams();
  const isCreateMode = searchParams.get('mode') === 'create';
  const hasAccount = useAuthStore((s) => s.hasAccount);
  const { t } = useTranslation(['auth', 'common', 'settings']);

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    // 在途保护：Button disabled 挡不住 Enter 键/form submit，防止并发 bootstrap 导致 error/isLoading 高频交替闪烁
    if (useAuthStore.getState().isLoading) return;
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
      // P034: 创建成功后立即清空密码 state（JS 堆不可清零，尽早缩短驻留窗口）
      setPassword('');
      setConfirm('');
      setPasswordHint('');
      navigate('/');
      return;
    }
    // 重名错误：i18n 后挂到账户名输入框（红边 + 抖动），不再走独立错误 div
    if (translateRustError(state.error) === 'common:account_name_taken') {
      setAccountNameError(t('common:account_name_taken'));
      setNameErrorTick((n) => n + 1);
    }
  };

  // 后端错误展示文本：translateRustError 映射优先（返回 i18n key，需 t() 转译），未命中保留启发式兜底；
  // 重名错误已挂到账户名输入框行内（accountNameError），不再在此重复展示
  const translatedError = error ? translateRustError(error) : null;
  const isNameTakenError = translatedError === 'common:account_name_taken';
  const backendErrorText =
    error && !isNameTakenError
      ? (translatedError
          ? t(translatedError)
          : error.toLowerCase().includes('8 characters') || error.toLowerCase().includes('至少')
            ? t('auth:password_too_short')
            : error.toLowerCase().includes('password') || error.toLowerCase().includes('invalid')
              ? t('auth:incorrect_password')
              : error.toLowerCase().includes('required')
                ? t('auth:password_required')
                : error)
      : null;

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
            errorTick={nameErrorTick}
            reserveErrorSpace
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
          {/* 后端错误区：minHeight 固定占位，错误出现/消失不改变卡片高度（防闪烁） */}
          <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)', minHeight: 20 }}>
            {backendErrorText}
          </div>
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

        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: 4,
            marginTop: 16,
          }}
        >
          {/* 始终提供「返回账户来源选择」：切换 SAF 目录后若账户未被检测到，
              用户可从这里回到引导，重新选择目录或从其他设备恢复。 */}
          <button
            type="button"
            onClick={() => useUiStore.getState().setReopenAccountSource(true)}
            className="interactive-accent-link"
            style={{
              fontSize: 'var(--text-body-sm)',
              background: 'transparent',
              border: 'none',
              padding: '6px 12px',
              cursor: 'pointer',
              fontFamily: 'inherit',
            }}
          >
            {t('common:back_to_account_source_link')}
          </button>
          {/* 仅当确实存在账户时才显示「返回登录页」（否则 hasAccount=false 时
              /login 会再重定向回 /bootstrap，形成死循环） */}
          {(isCreateMode || hasAccount === true) && (
            <button
              type="button"
              onClick={() => navigate('/login')}
              className="interactive-accent-link"
              style={{
                fontSize: 'var(--text-body-sm)',
                background: 'transparent',
                border: 'none',
                padding: '6px 12px',
                cursor: 'pointer',
                fontFamily: 'inherit',
              }}
            >
              {t('common:back_to_login_link')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
