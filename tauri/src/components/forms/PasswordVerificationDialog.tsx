import { useState, useEffect, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { Dialog } from '@/components/ui/Dialog';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { PinInput } from '@/components/forms/PinInput';
import { Button } from '@/components/ui/Button';
import { useToastError } from '@/hooks/useToastError';
import { useAutoLockPauseStore } from '@/stores/autoLockPauseStore';
import { Fingerprint, KeyRound, ScanFace, ShieldCheck, Grip } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

interface PasswordVerificationDialogProps {
  open: boolean;
  onClose: () => void;
  /** Called with the password. Return true to confirm, false to reject */
  onVerify: (password: string) => Promise<boolean>;
  /** Customizable text overrides — all i18n-able via parent */
  title?: string;
  description?: string;
  confirmLabel?: string;
  /** Optional password hint to display */
  hint?: string | null;
  /** Biometric type name (e.g. "Touch ID", "Face ID") — enables biometric button */
  biometricType?: string;
  /** Called when user clicks biometric button. Return true on success */
  onBiometric?: () => Promise<boolean>;
  /** If provided, enables PIN verification mode */
  pinAccountId?: string;
  /** Called when PIN unlock succeeds (instead of onClose, which always reports ok=false) */
  onPinSuccess?: () => void;
}

/** 生物识别类型的可读标签映射 */
const BIOMETRIC_LABEL: Record<string, string> = {
  faceId: 'Face ID',
  touchId: 'Touch ID',
  windowsHello: 'Windows Hello',
};

/** DEBUG: 设为 true 时，底部图标栏始终显示全部 5 种解锁方式按钮 */
const __DEBUG_SHOW_ALL_ICONS = false;

/** 解锁方式定义 */
interface UnlockMethodDef {
  id: string;
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}

/**
 * Unified password verification dialog — single source of truth for all
 * password-gated operations across the app.
 *
 * 按优先级显示解锁方式卡片：Face ID > Touch ID > Windows Hello > PIN > 密码
 * 底部统一图标栏切换方式（主密码 · Face ID · Touch ID · Windows Hello · PIN）。
 * 悬停图标时展开文字，左侧按钮不动右侧按钮被推向右。
 */
export function PasswordVerificationDialog({
  open,
  onClose,
  onVerify,
  title,
  description,
  confirmLabel,
  hint,
  biometricType,
  onBiometric,
  pinAccountId,
  onPinSuccess,
}: PasswordVerificationDialogProps) {
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [bioLoading, setBioLoading] = useState(false);
  const [pinAvailable, setPinAvailable] = useState(false);
  const [pinChecked, setPinChecked] = useState(false);
  const [pinUnlocking, setPinUnlocking] = useState(false);
  const [pinError, setPinError] = useState<string | null>(null);
  const [pinInputKey, setPinInputKey] = useState(0);
  const [hoveredIcon, setHoveredIcon] = useState<string | null>(null);
  const [committedIcon, setCommittedIcon] = useState<string | null>(null);
  const commitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { onError } = useToastError();
  const { t } = useTranslation(['auth', 'common', 'settings']);

  const hasBiometric = !!biometricType && !!onBiometric;

  // 当前显示的解锁方式（按优先级选择）
  const [loginMethod, setLoginMethod] = useState<
    'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password' | null
  >(null);

  // 从 loginMethod 推导生物识别类型标签（与当前卡片图标保持一致）
  const activeBioType =
    loginMethod === 'faceId' || loginMethod === 'touchId' || loginMethod === 'windowsHello'
      ? loginMethod
      : biometricType || '';
  const biometricLabel =
    BIOMETRIC_LABEL[activeBioType] ||
    activeBioType ||
    t('auth:bio_default', { defaultValue: 'Biometric' });

  // 卸载时清理悬停延迟定时器
  useEffect(() => {
    return () => {
      if (commitTimerRef.current) clearTimeout(commitTimerRef.current);
    };
  }, []);

  // 对话框打开期间暂停自动锁定计时（与 CLI 的 auto_lock_paused 语义一致），
  // 避免用户长时间未输入时验证框被锁定流程变成孤儿状态
  useEffect(() => {
    if (!open) return;
    const { pause, resume } = useAutoLockPauseStore.getState();
    pause();
    return () => resume();
  }, [open]);

  // 对话框打开时重置状态、检查可用性
  useEffect(() => {
    if (!open) {
      setPinChecked(false);
      setPinAvailable(false);
      setLoginMethod(null);
      return;
    }

    setPassword('');
    setError(null);
    setPinError(null);
    setPinUnlocking(false);
    setBioLoading(false);
    setLoginMethod(null);
    setPinChecked(false);
    setPinAvailable(false);

    if (pinAccountId) {
      invoke<{ configured: boolean; locked: boolean }>('pin_check_availability', {
        accountId: pinAccountId,
      })
        .then((r) => setPinAvailable(r.configured && !r.locked))
        .catch(() => setPinAvailable(false))
        .finally(() => setPinChecked(true));
    } else {
      setPinChecked(true);
    }
  }, [open, pinAccountId]);

  // PIN 检查完成后按优先级设置默认解锁方式
  useEffect(() => {
    if (!open || !pinChecked) return;

    // Priority: FaceID > Touch ID > Windows Hello > PIN > Password
    if (hasBiometric) {
      const raw = biometricType || '';
      if (raw === 'faceId') setLoginMethod('faceId');
      else if (raw === 'touchId') setLoginMethod('touchId');
      else if (raw === 'windowsHello') setLoginMethod('windowsHello');
      else setLoginMethod('password');
    } else if (pinAvailable) {
      setLoginMethod('pin');
    } else {
      setLoginMethod('password');
    }
  }, [open, pinChecked, hasBiometric, pinAvailable, biometricType]);

  const handlePinComplete = useCallback(
    async (pin: string) => {
      if (!pinAccountId) return;
      setPinUnlocking(true);
      setPinError(null);
      try {
        await invoke('pin_unlock', {
          accountId: pinAccountId,
          pin,
          location: 'critical_data_access',
          action: 'unlock',
        });
        setPassword('');
        setPinError(null);
        onPinSuccess?.();
      } catch (e) {
        const msg = String(e);
        if (msg.includes('__PIN_ERR__:locked')) {
          setPinError(t('auth:pin_locked'));
          setPinAvailable(false);
          setLoginMethod('password');
        } else if (msg.includes('__PIN_ERR__:incorrect')) {
          setPinError(t('auth:pin_incorrect'));
        } else {
          setPinError(t('auth:pin_error'));
        }
        setPinInputKey((k) => k + 1);
      } finally {
        setPinUnlocking(false);
      }
    },
    [pinAccountId, t, onPinSuccess],
  );

  const handleConfirm = async () => {
    if (!password) {
      setError(t('auth:password_required'));
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const ok = await onVerify(password);
      if (ok) {
        setPassword('');
        onClose();
      } else {
        setError(t('auth:incorrect_password'));
      }
    } catch (e) {
      onError(e, t('common:error'));
    } finally {
      setLoading(false);
    }
  };

  const handleBiometric = async () => {
    if (!onBiometric) return;
    setBioLoading(true);
    setError(null);
    try {
      const ok = await onBiometric();
      if (ok) {
        setPassword('');
        onClose();
      }
    } catch {
      // User cancelled or failed — silently stay on screen
    } finally {
      setBioLoading(false);
    }
  };

  const handleClose = () => {
    setPassword('');
    setError(null);
    setLoginMethod(null);
    setPinError(null);
    setCommittedIcon(null);
    if (commitTimerRef.current) clearTimeout(commitTimerRef.current);
    onClose();
  };

  // 两阶段悬停：边框/颜色立即高亮，文字/展开延迟 200ms 后触发
  const handleIconEnter = (id: string) => {
    setHoveredIcon(id);
    // 清除上一次的定时器
    if (commitTimerRef.current) clearTimeout(commitTimerRef.current);
    // 200ms 后提交展开状态
    commitTimerRef.current = setTimeout(() => {
      setCommittedIcon(id);
      commitTimerRef.current = null;
    }, 300);
  };

  const handleIconLeave = () => {
    setHoveredIcon(null);
    setCommittedIcon(null);
    if (commitTimerRef.current) {
      clearTimeout(commitTimerRef.current);
      commitTimerRef.current = null;
    }
  };

  // ==== 构建可用解锁方式列表 ====
  // 顺序：主密码 → Face ID → Touch ID → Windows Hello → PIN
  const methods: UnlockMethodDef[] = [];
  // 1. 主密码（始终可用）
  methods.push({
    id: 'password',
    icon: <KeyRound size={ICON_SIZE.xl} />,
    label: t('auth:password_method', { defaultValue: '主密码' }),
    onClick: () => setLoginMethod('password'),
  });

  // DEBUG 模式：忽略实际可用性，强制显示全部生物识别图标
  const showAll = __DEBUG_SHOW_ALL_ICONS;
  const effectiveHasBiometric = showAll || hasBiometric;
  const effectivePinAvailable = showAll || pinAvailable;

  // 2–4. 生物识别（根据类型显示其中一个；DEBUG 时全部显示）
  if (effectiveHasBiometric) {
    if (showAll || biometricType === 'faceId') {
      methods.push({
        id: 'faceId',
        icon: <ScanFace size={ICON_SIZE.xl} />,
        label: 'Face ID',
        onClick: () => setLoginMethod('faceId'),
      });
    }
    if (showAll || biometricType === 'touchId') {
      methods.push({
        id: 'touchId',
        icon: <Fingerprint size={ICON_SIZE.xl} />,
        label: 'Touch ID',
        onClick: () => setLoginMethod('touchId'),
      });
    }
    if (showAll || biometricType === 'windowsHello') {
      methods.push({
        id: 'windowsHello',
        icon: <ShieldCheck size={ICON_SIZE.xl} />,
        label: 'Windows Hello',
        onClick: () => setLoginMethod('windowsHello'),
      });
    }
  }
  // 5. PIN 码
  if (effectivePinAvailable) {
    methods.push({
      id: 'pin',
      icon: <Grip size={ICON_SIZE.xl} />,
      label: t('auth:pin_method', { defaultValue: 'PIN 码' }),
      onClick: () => {
        setLoginMethod('pin');
        setPinError(null);
      },
    });
  }

  // 判断某方式是否为当前活跃方式
  const isActiveMethod = (methodId: string): boolean => {
    return loginMethod === methodId;
  };

  return (
    <Dialog isOpen={open} onClose={handleClose} dialogStyle={{ maxWidth: 360 }} priority="auth">
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16, minWidth: 320 }}>
        <h2 style={{ fontSize: 'var(--text-section-title)', fontWeight: 600, margin: 0 }}>
          {title || t('auth:verification_title')}
        </h2>
        {description && (
          <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', margin: 0 }}>
            {description}
          </p>
        )}

        {/* ===== 生物识别卡片 ===== */}
        {(loginMethod === 'faceId' ||
          loginMethod === 'touchId' ||
          loginMethod === 'windowsHello') && (
          <div
            style={{
              minHeight: 152,
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'center',
              marginBottom: 8,
            }}
          >
            <button
              onClick={handleBiometric}
              disabled={bioLoading}
              className="interactive-card-lift"
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 12,
                padding: '20px 24px',
                borderRadius: 14,
                borderWidth: 1,
                borderStyle: 'solid',
                background: bioLoading ? 'var(--bg-toolbar)' : 'transparent',
                cursor: bioLoading ? 'wait' : 'pointer',
                width: '100%',
                fontFamily: 'inherit',
              }}
            >
              {loginMethod === 'faceId' && (
                <ScanFace
                  size={ICON_SIZE['4xl']}
                  color="var(--accent-primary)"
                  style={{ opacity: bioLoading ? 0.5 : 1 }}
                />
              )}
              {loginMethod === 'touchId' && (
                <Fingerprint
                  size={ICON_SIZE['4xl']}
                  color="var(--accent-primary)"
                  style={{ opacity: bioLoading ? 0.5 : 1 }}
                />
              )}
              {loginMethod === 'windowsHello' && (
                <ShieldCheck
                  size={ICON_SIZE['4xl']}
                  color="var(--accent-primary)"
                  style={{ opacity: bioLoading ? 0.5 : 1 }}
                />
              )}
              <span
                style={{
                  fontSize: 'var(--text-card-title)',
                  fontWeight: 500,
                  color: 'var(--text-primary)',
                }}
              >
                {bioLoading
                  ? t('auth:bio_verifying')
                  : t('auth:bio_unlock_reason', { type: biometricLabel })}
              </span>
            </button>
          </div>
        )}

        {/* ===== PIN 码卡片 ===== */}
        {loginMethod === 'pin' && (
          <div
            style={{
              minHeight: 152,
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'center',
              marginBottom: 8,
            }}
          >
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 12,
                padding: '16px 24px 20px',
                borderRadius: 14,
                border: '1px solid var(--border-subtle)',
                background: 'transparent',
                width: '100%',
              }}
            >
              <Grip size={ICON_SIZE['2xl']} color="var(--accent-primary)" />
              <span
                style={{
                  fontSize: 'var(--text-card-title)',
                  fontWeight: 500,
                  color: 'var(--text-primary)',
                }}
              >
                {t('auth:pin_enter_title')}
              </span>
              <PinInput
                key={pinInputKey}
                length={6}
                onComplete={handlePinComplete}
                disabled={pinUnlocking}
                error={!!pinError}
                verifying={pinUnlocking}
              />
              {pinError && (
                <div style={{ color: '#dc2626', fontSize: 'var(--text-body-sm)' }}>{pinError}</div>
              )}
            </div>
          </div>
        )}

        {/* ===== 密码卡片 ===== */}
        {(loginMethod === 'password' || loginMethod === null) && (
          <div
            style={{
              minHeight: 152,
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'center',
              marginBottom: 8,
            }}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <SecurePasswordInput
                value={password}
                onChange={(v) => {
                  setPassword(v);
                  setError(null);
                }}
                placeholder={t('common:password_placeholder')}
                error={error}
                autoComplete="current-password"
                hint={hint}
                onEnter={handleConfirm}
              />
              <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
                <Button variant="secondary" onClick={handleClose}>
                  {t('common:cancel')}
                </Button>
                <Button onClick={handleConfirm} loading={loading} disabled={!password}>
                  {confirmLabel || t('common:confirm')}
                </Button>
              </div>
            </div>
          </div>
        )}

        {/* loginMethod === null（正在检测可用性）时显示轻量 loading */}
        {loginMethod === null && (
          <div
            style={{
              display: 'flex',
              justifyContent: 'center',
              padding: '24px 0',
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-tertiary)',
            }}
          >
            {t('common:loading', { defaultValue: '...' })}
          </div>
        )}

        {/* ===== 底部图标栏 — 切换解锁方式 ===== */}
        {loginMethod !== null && methods.length > 1 && (
          <div
            style={{
              display: 'flex',
              gap: 6,
              paddingTop: 12,
              borderTop: '1px solid var(--border-subtle)',
              justifyContent: 'flex-start',
              overflow: 'hidden',
              maxWidth: '100%',
            }}
          >
            {methods.map((method) => {
              const isActive = isActiveMethod(method.id);
              const isHovered = hoveredIcon === method.id;
              const isExpanded = committedIcon === method.id;

              return (
                <button
                  key={method.id}
                  aria-label={method.label}
                  onClick={() => {
                    setHoveredIcon(null);
                    setCommittedIcon(null);
                    if (commitTimerRef.current) {
                      clearTimeout(commitTimerRef.current);
                      commitTimerRef.current = null;
                    }
                    method.onClick();
                  }}
                  onMouseEnter={() => handleIconEnter(method.id)}
                  onMouseLeave={handleIconLeave}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                    padding: '6px 10px',
                    borderRadius: 8,
                    border: `1px solid ${
                      isHovered
                        ? 'var(--accent-primary)'
                        : isActive
                          ? 'color-mix(in srgb, var(--accent-primary) 40%, transparent)'
                          : 'transparent'
                    }`,
                    background: isActive
                      ? 'color-mix(in srgb, var(--accent-primary) 6%, transparent)'
                      : 'transparent',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                    fontSize: 'var(--text-body-sm)',
                    color: isHovered
                      ? 'var(--accent-primary)'
                      : isActive
                        ? 'var(--text-primary)'
                        : 'var(--text-tertiary)',
                    whiteSpace: 'nowrap',
                    overflow: 'hidden',
                    maxWidth: isExpanded ? 200 : 40,
                    transition:
                      isExpanded || (!isHovered && !isExpanded)
                        ? 'all 0.25s ease'
                        : 'all 0.25s ease, max-width 0.01s linear 0.2s',
                    flexShrink: 0,
                    outline: 'none',
                  }}
                >
                  {/* 图标始终可见 */}
                  <span style={{ flexShrink: 0, display: 'flex', alignItems: 'center' }}>
                    {method.icon}
                  </span>
                  {/* 文字：延迟 200ms 后才显示（与 committedIcon 同步） */}
                  <span
                    style={{
                      opacity: isExpanded ? 1 : 0,
                      transition: 'opacity 0.2s ease 0.05s',
                      overflow: 'hidden',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {method.label}
                  </span>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </Dialog>
  );
}
