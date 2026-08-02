import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useNavigate } from 'react-router-dom';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { Fingerprint, Grip, X } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

/** localStorage key for guide dismissal count tracking. */
const LS_GUIDE_DISMISS_COUNT = 'solosoul_setup_guide_dismiss_count';
const MAX_DISMISS_COUNT = 3;

/** Account ID → tracked flag to avoid re-doing lifecycle checks on re-render. */
const _shownForAccount = new Set<string>();

export function PostLoginSetupGuide() {
  const { t } = useTranslation(['common', 'settings', 'auth']);
  const navigate = useNavigate();
  const currentAccount = useAuthStore((s) => s.currentAccount);

  const [showGuide, setShowGuide] = useState(false);
  const [hasBiometric, setHasBiometric] = useState(false);
  const [hasPin, setHasPin] = useState(false);
  const [dismissed, setDismissed] = useState(false);

  // Reset state when account changes
  useEffect(() => {
    setDismissed(false);
    setShowGuide(false);
  }, [currentAccount?.id]);

  // Check account history on mount when currentAccount is available
  useEffect(() => {
    if (!currentAccount?.id) return;
    const accountId = currentAccount.id;

    // Skip if we've already processed this account in the current session
    if (_shownForAccount.has(accountId)) return;

    // 同步标记已处理，防止后续重渲染/解挂时 async 回调被取消导致永久丢失
    _shownForAccount.add(accountId);

    // 持久化 reconciled 检查：跨会话不重复提示
    const reconciledKey = `solosoul_guide_reconciled_${accountId}`;
    try {
      if (localStorage.getItem(reconciledKey) === 'true') {
        return;
      }
    } catch {
      // Ignore localStorage errors
    }

    // Check dismissal count from localStorage
    try {
      const count = parseInt(localStorage.getItem(LS_GUIDE_DISMISS_COUNT) || '0', 10);
      if (count >= MAX_DISMISS_COUNT) return;
    } catch {
      // Ignore localStorage errors
    }

    const hasBioHistory = currentAccount.hasBiometricHistory === true;
    const hasPinHistory = currentAccount.hasPinHistory === true;

    // 无任何解锁方式历史记录 → 已 reconcil，不再检查
    if (!hasBioHistory && !hasPinHistory) {
      try {
        localStorage.setItem(reconciledKey, 'true');
      } catch {
        // Ignore
      }
      return;
    }

    // 历史标志只表示「曾经用过」，不代表「本机未配置」。
    // 以当前设备真实配置状态为准：已在安全设置中重新设置过的
    // 解锁方式不再提醒；只有「有历史但本机未配置」才提示。
    let cancelled = false;
    (async () => {
      let bioConfigured = false;
      let pinConfigured = false;
      if (hasBioHistory) {
        bioConfigured = await invoke<{
          strongConfigured?: boolean;
          weakConfigured?: boolean;
        }>('biometric_check_availability', { accountId: accountId })
          .then((r) => r.strongConfigured === true || r.weakConfigured === true)
          .catch(() => false);
      }
      if (hasPinHistory) {
        pinConfigured = await invoke<{ configured?: boolean }>('pin_check_availability', {
          accountId: accountId,
        })
          .then((r) => r.configured === true)
          .catch(() => false);
      }
      if (cancelled) return;

      const needBio = hasBioHistory && !bioConfigured;
      const needPin = hasPinHistory && !pinConfigured;
      // 全部已配置：不再提醒，标记 reconciled 并跳过
      if (!needBio && !needPin) {
        try {
          localStorage.setItem(reconciledKey, 'true');
        } catch {
          // Ignore
        }
        return;
      }

      setHasBiometric(needBio);
      setHasPin(needPin);
      setShowGuide(true);
    })();
    return () => {
      cancelled = true;
    };
  }, [currentAccount]);

  const handleDismiss = useCallback(() => {
    setShowGuide(false);
    setDismissed(true);
    // Increment dismiss counter
    try {
      const count = parseInt(localStorage.getItem(LS_GUIDE_DISMISS_COUNT) || '0', 10);
      localStorage.setItem(LS_GUIDE_DISMISS_COUNT, String(count + 1));
    } catch {
      // Ignore localStorage errors
    }
  }, []);

  const handleSetupBiometric = useCallback(() => {
    setShowGuide(false);
    navigate('/settings/security');
  }, [navigate]);

  const handleSetupPin = useCallback(() => {
    setShowGuide(false);
    navigate('/settings/security');
  }, [navigate]);

  if (!showGuide || dismissed) return null;

  return (
    <div
      style={{
        position: 'fixed',
        left: '50%',
        bottom: 80,
        transform: 'translateX(-50%)',
        zIndex: 900,
        maxWidth: 380,
        width: 'calc(100% - 40px)',
        background: 'var(--bg-elevated)',
        border: '1px solid var(--border-subtle)',
        borderRadius: 14,
        boxShadow: '0 4px 24px rgba(0,0,0,0.12)',
        padding: 16,
        animation: 'slideUpIn 0.3s ease',
      }}
    >
      <style>{`
        @keyframes slideUpIn {
          from { opacity: 0; transform: translate(-50%, 16px); }
          to   { opacity: 1; transform: translate(-50%, 0); }
        }
      `}</style>

      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: 12,
        }}
      >
        <span
          style={{
            fontSize: 'var(--text-card-title)',
            fontWeight: 600,
            color: 'var(--text-primary)',
          }}
        >
          {t('common:quick_setup_title', { defaultValue: '快速设置' })}
        </span>
        <button
          onClick={handleDismiss}
          aria-label={t('common:close', { defaultValue: '关闭' })}
          style={{
            border: 'none',
            cursor: 'pointer',
            padding: 4,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: 6,
          }}
          className="interactive-ghost"
        >
          <X size={ICON_SIZE.md} />
        </button>
      </div>

      <p
        style={{
          fontSize: 'var(--text-body-sm)',
          color: 'var(--text-secondary)',
          marginBottom: 16,
          lineHeight: 1.5,
        }}
      >
        {t('common:setup_guide_description', {
          defaultValue:
            '检测到您之前使用过快捷解锁方式，是否重新设置？设置后下次登录可免输主密码。',
        })}
      </p>

      {/* Biometric setup button */}
      {hasBiometric && (
        <button
          onClick={handleSetupBiometric}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            width: '100%',
            padding: '10px 14px',
            borderRadius: 10,
            borderWidth: 1,
            borderStyle: 'solid',
            cursor: 'pointer',
            fontFamily: 'inherit',
            fontSize: 'var(--text-body)',
            color: 'var(--text-primary)',
            marginBottom: 8,
          }}
          className="interactive-toolbar"
        >
          <Fingerprint size={ICON_SIZE.lg} color="var(--accent-primary)" />
          <span
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'flex-start',
              gap: 2,
            }}
          >
            <span>{t('settings:biometric_setup_guide', { defaultValue: '设置指纹/面容解锁' })}</span>
            <span
              style={{
                fontSize: 'var(--text-badge)',
                color: 'var(--text-tertiary)',
              }}
            >
              {t('common:biometric_setup_desc', {
                defaultValue: '使用指纹或面容快速解锁保险库',
              })}
            </span>
          </span>
        </button>
      )}

      {/* PIN setup button */}
      {hasPin && (
        <button
          onClick={handleSetupPin}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            width: '100%',
            padding: '10px 14px',
            borderRadius: 10,
            borderWidth: 1,
            borderStyle: 'solid',
            cursor: 'pointer',
            fontFamily: 'inherit',
            fontSize: 'var(--text-body)',
            color: 'var(--text-primary)',
          }}
          className="interactive-toolbar"
        >
          <Grip size={ICON_SIZE.lg} color="var(--accent-primary)" />
          <span
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'flex-start',
              gap: 2,
            }}
          >
            <span>{t('settings:pin_setup_guide', { defaultValue: '设置 PIN 码解锁' })}</span>
            <span
              style={{
                fontSize: 'var(--text-badge)',
                color: 'var(--text-tertiary)',
              }}
            >
              {t('common:pin_setup_desc', { defaultValue: '使用 PIN 码快速解锁保险库' })}
            </span>
          </span>
        </button>
      )}

      {/* Dismiss link */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'flex-end',
          marginTop: 12,
        }}
      >
        <button
          onClick={handleDismiss}
          className="interactive-color-secondary"
          style={{
            background: 'transparent',
            border: 'none',
            fontSize: 'var(--text-badge)',
            cursor: 'pointer',
            fontFamily: 'inherit',
            padding: '4px 8px',
          }}
        >
          {t('common:not_now', { defaultValue: '暂不' })}
        </button>
      </div>
    </div>
  );
}
