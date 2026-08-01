import { useState, useRef, useEffect, useCallback, useId } from 'react';
import type { ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Lock, Eye, EyeOff, HelpCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ICON_SIZE } from '@/lib/constants';

interface SecurePasswordInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  label?: string | ReactNode;
  error?: string | null;
  autoComplete?: string;
  /** Enter key callback for form submission without mouse */
  onEnter?: () => void;
  /** Focus callback for the underlying input element */
  onFocus?: () => void;
  /** Password hint shown via the hint button tooltip */
  hint?: string | null;
  /** Whether to show the hint button. Defaults to true.
   *  Set false for new/confirm password fields (8.3). */
  showHintButton?: boolean;
}

export function SecurePasswordInput({
  value,
  onChange,
  placeholder = 'common:password_placeholder',
  disabled = false,
  className = '',
  label,
  error,
  autoComplete,
  hint,
  showHintButton = true,
  onEnter,
  onFocus,
}: SecurePasswordInputProps) {
  const inputId = useId();
  const [visible, setVisible] = useState(false);
  const [isHintHovered, setIsHintHovered] = useState(false);
  const [hintCardPos, setHintCardPos] = useState<{ top: number; left: number } | null>(null);
  const [isFocused, setIsFocused] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const [shouldShake, setShouldShake] = useState(false);
  const prevErrorRef = useRef(error);
  const hintBtnRef = useRef<HTMLButtonElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const touchActiveRef = useRef(false);
  
  // 触屏点击标记：用于阻止触屏操作后浏览器合成的 mouseenter 事件重新打开卡片
  const clearTouchFlag = useCallback(() => {
    touchActiveRef.current = false;
  }, []);
  const { t } = useTranslation(['common', 'auth']);

  // Focus change handler — sync to state and fire external callback
  const handleFocus = useCallback(() => {
    setIsFocused(true);
    onFocus?.();
  }, [onFocus]);
  const handleBlur = useCallback(() => {
    setVisible(false);
    setIsFocused(false);
  }, []);

  // Shake on new error
  useEffect(() => {
    if (error && error !== prevErrorRef.current) {
      setShouldShake(true);
      const timer = setTimeout(() => setShouldShake(false), 300);
      prevErrorRef.current = error;
      return () => clearTimeout(timer);
    }
    prevErrorRef.current = error;
  }, [error]);

  const updateHintCardPos = useCallback(() => {
    if (hintBtnRef.current) {
      const rect = hintBtnRef.current.getBoundingClientRect();
      const left = rect.right + 8;
      const cardWidth = 240;
      const viewportWidth = window.innerWidth;
      // 确保提示卡片不溢出视口右侧
      const clampedLeft = left + cardWidth > viewportWidth - 16
        ? Math.max(8, viewportWidth - cardWidth - 16)
        : left;
      setHintCardPos({ top: rect.top + rect.height / 2, left: clampedLeft });
    }
  }, []);

  const handleHintEnter = useCallback(() => {
    // 触屏操作后浏览器可能合成 mouseenter，跳过以避免重新打开已关闭的卡片
    if (touchActiveRef.current) return;
    setIsHintHovered(true);
    updateHintCardPos();
  }, [updateHintCardPos]);

  const handleHintLeave = useCallback(() => {
    setIsHintHovered(false);
  }, []);

  // 触屏设备：点击提示词按钮时切换卡片显示
  // 注意：直接操作 state，不委托给 handleHintEnter/handleHintLeave，
  // 避免 touchActiveRef 阻止自身打开/关闭。
  const handleHintTouch = useCallback(
    (e: React.TouchEvent) => {
      e.stopPropagation();
      touchActiveRef.current = true;
      const nowShowing = !isHintHovered;
      setIsHintHovered(nowShowing);
      if (nowShowing) updateHintCardPos();
      setTimeout(clearTouchFlag, 300);
    },
    [isHintHovered, updateHintCardPos, clearTouchFlag],
  );

  // 触屏/鼠标设备：点击卡片外部区域时关闭卡片
  useEffect(() => {
    if (!isHintHovered) return;
    const closeOnOutside = (e: MouseEvent | TouchEvent) => {
      if (hintBtnRef.current && !hintBtnRef.current.contains(e.target as Node)) {
        setIsHintHovered(false);
      }
    };
    // 使用 setTimeout 避免与当前触发事件冲突
    const timer = setTimeout(() => {
      document.addEventListener('touchstart', closeOnOutside);
      document.addEventListener('mousedown', closeOnOutside);
    }, 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener('touchstart', closeOnOutside);
      document.removeEventListener('mousedown', closeOnOutside);
    };
  }, [isHintHovered]);

  useEffect(() => {
    if (!isHintHovered) return;
    window.addEventListener('scroll', updateHintCardPos, true);
    window.addEventListener('resize', updateHintCardPos);
    return () => {
      window.removeEventListener('scroll', updateHintCardPos, true);
      window.removeEventListener('resize', updateHintCardPos);
    };
  }, [isHintHovered, updateHintCardPos]);

  const hasHint = hint && hint.trim().length > 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      {label && (
        <label
          htmlFor={inputId}
          style={{
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            color: 'var(--text-secondary)',
          }}
        >
          {label}
        </label>
      )}
      <style>{`
        @keyframes fadeInSlideDown {
          from { opacity: 0; transform: translateY(-4px); }
          to   { opacity: 1; transform: translateY(0); }
        }
        @keyframes shake {
          0%, 100% { transform: translateX(0); }
          20%      { transform: translateX(-4px); }
          40%      { transform: translateX(4px); }
          60%      { transform: translateX(-4px); }
          80%      { transform: translateX(4px); }
        }
      `}</style>
      <div
        className={className}
        onMouseEnter={() => !disabled && setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        style={{
          position: 'relative',
          display: 'flex',
          alignItems: 'center',
          border: error
            ? '1px solid var(--accent-danger, #dc2626)'
            : isFocused
              ? '1px solid var(--accent-primary)'
              : isHovered && !disabled
                ? '1px solid var(--accent-primary)'
                : '1px solid var(--border-subtle)',
          borderRadius: 8,
          boxShadow: isFocused
            ? '0 0 0 2px color-mix(in srgb, var(--accent-primary) 15%, transparent)'
            : isHovered && !disabled
              ? '0 0 0 2px color-mix(in srgb, var(--accent-primary) 10%, transparent)'
              : 'none',
          transition: 'border-color 0.2s, box-shadow 0.2s',
          backgroundColor: 'var(--bg-input)',
          cursor: disabled ? 'not-allowed' : undefined,
          animation: shouldShake ? 'shake 0.3s ease-in-out' : 'none',
        }}
      >
        <Lock
          size={ICON_SIZE.sm}
          style={{
            position: 'absolute',
            left: 12,
            color: 'var(--text-tertiary)',
            pointerEvents: 'none',
          }}
        />
        <input
          id={inputId}
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && onEnter) {
              e.preventDefault();
              onEnter();
            }
          }}
          onFocus={handleFocus}
          onBlur={handleBlur}
          placeholder={t(placeholder)}
          disabled={disabled}
          autoComplete={autoComplete}
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          data-1p-ignore
          data-lpignore="true"
          aria-live="polite"
          style={{
            flex: 1,
            border: 'none',
            outline: 'none',
            padding: showHintButton ? '10px 72px 10px 32px' : '10px 48px 10px 32px',
            fontSize: 'var(--text-body)',
            background: 'transparent',
            color: 'var(--text-primary)',
            fontFamily: 'inherit',
            // Replace type="password" masking with CSS text-security
            ...(visible ? {} : { WebkitTextSecurity: 'disc' as unknown as string }),
          }}
        />

        {/* Button area (absolute positioned on the right) */}
        <div
          style={{
            position: 'absolute',
            right: 8,
            top: 0,
            bottom: 0,
            display: 'flex',
            alignItems: 'center',
            gap: 2,
          }}
        >
          {/* Visibility toggle button */}
          {value.length > 0 && (
            <button
              type="button"
              onClick={() => setVisible((prev) => !prev)}
              aria-label={visible ? t('common:hide_password') : t('common:show_password')}
              aria-pressed={visible}
              tabIndex={-1}
              className="interactive-icon"
              style={{
                border: 'none',
                cursor: 'pointer',
                padding: 4,
                borderRadius: 4,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              {visible ? <EyeOff size={ICON_SIZE.md} /> : <Eye size={ICON_SIZE.md} />}
            </button>
          )}

          {/* Hint button — hover to show via Portal (no close button) */}
          {showHintButton && (
            <div
              style={{ position: 'relative', display: 'flex' }}
              onMouseEnter={handleHintEnter}
              onMouseLeave={handleHintLeave}
              onTouchStart={handleHintTouch}
            >
              <button
                ref={hintBtnRef}
                type="button"
                aria-label={t('common:password_hint_tooltip')}
                tabIndex={-1}
                className="interactive-icon"
                style={{
                  border: 'none',
                  cursor: 'pointer',
                  padding: 4,
                  borderRadius: 4,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: isHintHovered ? 'var(--accent-primary)' : 'var(--text-tertiary)',
                }}
              >
                <HelpCircle size={ICON_SIZE.md} />
              </button>

              {/* Card via Portal (not clipped by overflow) */}
              {isHintHovered &&
                createPortal(
                  <div
                    data-testid="password-hint-tooltip"
                    style={{
                      position: 'fixed',
                      top: hintCardPos?.top ?? 0,
                      left: hintCardPos?.left ?? 0,
                      transform: 'translateY(-50%)',
                      zIndex: 'var(--z-tooltip)',
                      whiteSpace: 'normal',
                      wordBreak: 'keep-all',
                      overflowWrap: 'break-word',
                      maxWidth: 240,
                      padding: '8px 10px',
                      borderRadius: 6,
                      fontSize: 'var(--text-badge)',
                      lineHeight: 1.4,
                      color: 'var(--text-secondary)',
                      background: 'var(--bg-elevated)',
                      boxShadow: 'var(--shadow-md)',
                    }}
                  >
                    {hasHint ? hint : t('common:no_hint_available')}
                  </div>,
                  document.body,
                )}
            </div>
          )}
        </div>
      </div>
      {error && (
        <span
          role="alert"
          key={error}
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--accent-danger, #dc2626)',
            animation: 'fadeInSlideDown 0.25s ease-out',
          }}
        >
          {error}
        </span>
      )}
    </div>
  );
}
