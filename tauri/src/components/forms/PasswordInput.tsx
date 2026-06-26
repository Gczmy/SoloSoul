import { useState, useRef, useEffect, useCallback, useId } from 'react';
import type { ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Lock, Eye, EyeOff, HelpCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ICON_SIZE } from '@/lib/iconSizes';


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
  const { t } = useTranslation(['common', 'auth']);

  // Focus change handler — sync to state
  const handleFocus = useCallback(() => setIsFocused(true), []);
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
      setHintCardPos({ top: rect.top + rect.height / 2, left: rect.right + 8 });
    }
  }, []);

  const handleHintEnter = useCallback(() => {
    setIsHintHovered(true);
    updateHintCardPos();
  }, [updateHintCardPos]);

  const handleHintLeave = useCallback(() => {
    setIsHintHovered(false);
  }, []);

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
          style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500, color: 'var(--text-secondary)' }}
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
              style={{
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                padding: 4,
                borderRadius: 4,
                display: 'flex',
                alignItems: 'center',
                color: 'var(--text-tertiary)',
                transition: 'all 0.15s',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'none';
                e.currentTarget.style.color = 'var(--text-tertiary)';
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
            >
              <button
                ref={hintBtnRef}
                type="button"
                aria-label={t('common:password_hint_tooltip')}
                tabIndex={-1}
                style={{
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  padding: 4,
                  borderRadius: 4,
                  display: 'flex',
                  alignItems: 'center',
                  color: isHintHovered ? 'var(--accent-primary)' : 'var(--text-tertiary)',
                  transition: 'all 0.15s',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'none';
                  e.currentTarget.style.color = 'var(--text-tertiary)';
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
                      zIndex: 5000,
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
