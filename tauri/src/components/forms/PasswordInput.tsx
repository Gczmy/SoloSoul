import { useState } from 'react';
import { Eye, EyeOff, Lock } from 'lucide-react';

interface PasswordInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  showToggle?: boolean;
  disabled?: boolean;
  className?: string;
  label?: string;
  error?: string | null;
  autoComplete?: string;
}

export function PasswordInput({
  value,
  onChange,
  placeholder = 'Enter password',
  showToggle = true,
  disabled = false,
  className = '',
  label,
  error,
  autoComplete,
}: PasswordInputProps) {
  const [visible, setVisible] = useState(false);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      {label && (
        <label style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-secondary)' }}>
          {label}
        </label>
      )}
      <div style={{
        display: 'flex', alignItems: 'center',
        border: error ? '1px solid var(--accent-danger, #dc2626)' : '1px solid var(--border-subtle)',
        borderRadius: 8, overflow: 'hidden',
        transition: 'border-color 0.2s',
        backgroundColor: 'var(--bg-input)',
      }}
        className={className}
      >
        <Lock size={14} style={{
          marginLeft: 10, color: 'var(--text-tertiary)', flexShrink: 0,
        }} />
        <input
          type={visible ? 'text' : 'password'}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          disabled={disabled}
          autoComplete={autoComplete}
          style={{
            flex: 1, border: 'none', outline: 'none',
            padding: '8px 8px', fontSize: 14,
            background: 'transparent',
            color: 'var(--text-primary)',
            fontFamily: 'inherit',
          }}
        />
        {showToggle && value.length > 0 && (
          <button
            type="button"
            onClick={() => setVisible(!visible)}
            aria-label={visible ? 'Hide password' : 'Show password'}
            style={{
              background: 'none', border: 'none', cursor: 'pointer',
              padding: '8px 10px', display: 'flex', alignItems: 'center',
              color: 'var(--text-tertiary)',
            }}
          >
            {visible ? <EyeOff size={16} /> : <Eye size={16} />}
          </button>
        )}
      </div>
      {error && (
        <span style={{ fontSize: 12, color: 'var(--accent-danger, #dc2626)' }}>
          {error}
        </span>
      )}
    </div>
  );
}
