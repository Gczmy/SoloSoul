import { useState, useRef, useEffect, useCallback } from 'react';
import type { ReactNode } from 'react';
import { Lock, Eye, EyeOff, HelpCircle, X } from 'lucide-react';

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
}

const TOOLTIP_CLOSE_DELAY = 300;

export function SecurePasswordInput({
  value,
  onChange,
  placeholder = 'Enter password',
  disabled = false,
  className = '',
  label,
  error,
  autoComplete,
  hint,
}: SecurePasswordInputProps) {
  const [visible, setVisible] = useState(false);
  const [tooltipOpen, setTooltipOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const tooltipTimeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  // 8.2 — 失焦后自动恢复遮蔽
  const handleBlur = useCallback(() => {
    setVisible(false);
  }, []);

  // 8.7 — 明文状态不持久化到任何 Store
  // visible state is local to this component only

  // 8.3 — 提示 Tooltip 点击外部关闭
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
          placeholder={placeholder}
          disabled={disabled}
          autoComplete={autoComplete}
          aria-live="polite"
          style={{
            flex: 1,
            border: 'none', outline: 'none',
            padding: '10px 72px 10px 32px',
            fontSize: 14,
            background: 'transparent',
            color: 'var(--text-primary)',
            fontFamily: 'inherit',
          }}
        />

        {/* 8.4 — 按钮区（absolute 定位在输入框右侧） */}
        <div style={{
          position: 'absolute', right: 8, top: 0, bottom: 0,
          display: 'flex', alignItems: 'center', gap: 2,
        }}>
          {/* 8.2 — 揭示按钮 */}
          {value.length > 0 && (
            <button
              type="button"
              onClick={() => setVisible((prev) => !prev)}
              aria-label={visible ? 'Hide password' : 'Show password'}
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

          {/* 8.3 — 提示按钮（始终显示） */}
          <div style={{ position: 'relative', display: 'flex' }}>
            <button
              type="button"
              onClick={handleTooltipToggle}
              aria-label="Password hint"
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

            {/* 8.3 — Tooltip 浮层 */}
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
                  padding: '8px 10px',
                  borderRadius: 6,
                  fontSize: 11,
                  lineHeight: 1.4,
                  color: 'var(--text-secondary)',
                  background: 'var(--bg-elevated)',
                  boxShadow: 'var(--shadow-md)',
                  zIndex: 100,
                  wordBreak: 'break-word',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'flex-start', gap: 4 }}>
                  <span style={{ flex: 1 }}>{hasHint ? hint : 'No hint available'}</span>
                  <button
                    type="button"
                    onClick={() => setTooltipOpen(false)}
                    aria-label="Close hint"
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
