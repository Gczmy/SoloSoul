import { useState, useRef, useEffect, useCallback } from 'react';
import type { ReactNode } from 'react';
import { Lock, Eye, EyeOff, HelpCircle, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface SecurePasswordInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  label?: string | ReactNode;
  error?: string | null;
  autoComplete?: string;
  /** Password hint shown via the hint button tooltip */
  hint?: string | null;
  /** Whether to show the hint button. Defaults to true.
   *  Set false for new/confirm password fields (8.3). */
  showHintButton?: boolean;
}

const TOOLTIP_CLOSE_DELAY = 300;

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
}: SecurePasswordInputProps) {
  const [visible, setVisible] = useState(false);
  const [tooltipOpen, setTooltipOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const tooltipTimeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);
  const { t } = useTranslation(['common', 'auth']);

  // Blur resets visibility
  const handleBlur = useCallback(() => {
    setVisible(false);
  }, []);

  // Close tooltip on outside click
  useEffect(() => {
    if (!tooltipOpen) return;
    const handler = (e: MouseEvent) => {
      if (
        tooltipRef.current &&
        !tooltipRef.current.contains(e.target as Node)
      ) {
        setTooltipOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [tooltipOpen]);

  const handleTooltipToggle = () => {
    if (tooltipTimeoutRef.current) {
      clearTimeout(tooltipTimeoutRef.current);
      tooltipTimeoutRef.current = undefined;
    }
    setTooltipOpen((prev) => !prev);
  };

  const handleTooltipClose = () => {
    tooltipTimeoutRef.current = setTimeout(() => {
      setTooltipOpen(false);
    }, TOOLTIP_CLOSE_DELAY);
  };

  const hasHint = hint && hint.trim().length > 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      {label && (
        <label
          htmlFor="secure-password-input"
          style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-secondary)' }}
        >
          {label}
        </label>
      )}
      <div
        className={className}
        style={{
          position: 'relative',
          display: 'flex', alignItems: 'center',
          border: error
            ? '1px solid var(--accent-danger, #dc2626)'
            : '1px solid var(--border-subtle)',
          borderRadius: 8,
          transition: 'border-color 0.2s',
          backgroundColor: 'var(--bg-input)',
        }}
      >
        <Lock size={14} style={{
          position: 'absolute', left: 12,
          color: 'var(--text-tertiary)', pointerEvents: 'none',
        }} />
        <input
          id="secure-password-input"
          ref={inputRef}
          type={visible ? 'text' : 'password'}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onBlur={handleBlur}
          placeholder={t(placeholder)}
          disabled={disabled}
          autoComplete={autoComplete}
          aria-live="polite"
          style={{
            flex: 1,
            border: 'none', outline: 'none',
            padding: showHintButton ? '10px 72px 10px 32px' : '10px 48px 10px 32px',
            fontSize: 14,
            background: 'transparent',
            color: 'var(--text-primary)',
            fontFamily: 'inherit',
          }}
        />

        {/* Button area (absolute positioned on the right) */}
        <div style={{
          position: 'absolute', right: 8, top: 0, bottom: 0,
          display: 'flex', alignItems: 'center', gap: 2,
        }}>
          {/* Visibility toggle button */}
          {value.length > 0 && (
            <button
              type="button"
              onClick={() => setVisible((prev) => !prev)}
              aria-label={visible ? t('common:hide_password') : t('common:show_password')}
              aria-pressed={visible}
              tabIndex={-1}
              style={{
                background: 'none', border: 'none', cursor: 'pointer',
                padding: 4, borderRadius: 4,
                display: 'flex', alignItems: 'center',
                color: 'var(--text-tertiary)',
                transition: 'color 0.15s',
              }}
              onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--text-secondary)'; }}
              onMouseLeave={(e) => { e.currentTarget.style.color = 'var(--text-tertiary)'; }}
            >
              {visible ? <EyeOff size={16} /> : <Eye size={16} />}
            </button>
          )}

          {/* Hint button (optional, controlled by showHintButton prop) */}
          {showHintButton && (
            <div style={{ position: 'relative', display: 'flex' }}>
              <button
                type="button"
                onClick={handleTooltipToggle}
                aria-label={t('common:password_hint_tooltip')}
                aria-pressed={tooltipOpen}
                tabIndex={-1}
                style={{
                  background: 'none', border: 'none', cursor: 'pointer',
                  padding: 4, borderRadius: 4,
                  display: 'flex', alignItems: 'center',
                  color: tooltipOpen ? 'var(--accent-primary)' : 'var(--text-tertiary)',
                  transition: 'color 0.15s',
                }}
                onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--accent-primary)'; }}
                onMouseLeave={(e) => {
                  if (!tooltipOpen) e.currentTarget.style.color = 'var(--text-tertiary)';
                }}
              >
                <HelpCircle size={16} />
              </button>

              {/* Tooltip popup */}
              {tooltipOpen && (
                <div
                  ref={tooltipRef}
                  role="tooltip"
                  onMouseEnter={() => {
                    if (tooltipTimeoutRef.current) {
                      clearTimeout(tooltipTimeoutRef.current);
                      tooltipTimeoutRef.current = undefined;
                    }
                  }}
                  onMouseLeave={handleTooltipClose}
                  style={{
                    position: 'absolute', bottom: 'calc(100% + 6px)',
                    right: 0,
                    maxWidth: 240,
                    minWidth: 80,
                    width: 'max-content',
                    padding: '8px 10px',
                    borderRadius: 6,
                    fontSize: 11,
                    lineHeight: 1.4,
                    color: 'var(--text-secondary)',
                    background: 'var(--bg-elevated)',
                    boxShadow: 'var(--shadow-md)',
                    zIndex: 100,
                    whiteSpace: 'normal',
                    wordBreak: 'keep-all',
                    overflowWrap: 'break-word',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'flex-start', gap: 4 }}>
                    <span style={{ flex: 1 }}>
                      {hasHint ? hint : t('common:no_hint_available')}
                    </span>
                    <button
                      type="button"
                      onClick={() => setTooltipOpen(false)}
                      aria-label={t('common:close')}
                      tabIndex={-1}
                      style={{
                        background: 'none', border: 'none', cursor: 'pointer',
                        padding: 0, display: 'flex',
                        color: 'var(--text-tertiary)',
                        flexShrink: 0, marginTop: 1,
                      }}
                    >
                      <X size={12} />
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
      {error && (
        <span
          role="alert"
          style={{ fontSize: 12, color: 'var(--accent-danger, #dc2626)' }}
        >
          {error}
        </span>
      )}
    </div>
  );
}
