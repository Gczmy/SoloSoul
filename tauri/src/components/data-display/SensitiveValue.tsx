import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Eye, EyeOff, Lock } from 'lucide-react';

export type SensitivityLevel = 'public' | 'internal' | 'sensitive' | 'critical';

interface SensitiveValueProps {
  value: string;
  level: SensitivityLevel;
  /** Called when user wants to reveal a masked value */
  onReveal?: () => Promise<boolean>;
  /** If true, the value is shown regardless of level */
  forceRevealed?: boolean;
  className?: string;
}

/**
 * Displays a value according to its sensitivity level:
 * - public:    plain text, no restrictions
 * - internal:  plain text, light indicator
 * - sensitive: blurred with "Click to reveal" overlay
 * - critical:  locked icon with "Unlock to view" overlay
 */
export function SensitiveValue({
  value,
  level,
  onReveal,
  forceRevealed = false,
  className = '',
}: SensitiveValueProps) {
  const [revealed, setRevealed] = useState(forceRevealed);
  const { t } = useTranslation('sensitivity');

  if (level === 'public' || level === 'internal' || revealed || forceRevealed) {
    return (
      <span
        className={className}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: 6,
          filter: level === 'internal' ? 'none' : 'none',
        }}
      >
        {level === 'internal' && (
          <span
            style={{
              width: 6,
              height: 6,
              borderRadius: '50%',
              background: 'var(--accent-warning, #f59e0b)',
              display: 'inline-block',
              flexShrink: 0,
            }}
            title={t('internal_hint')}
          />
        )}
        {value}
        {!forceRevealed && level === 'sensitive' && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              setRevealed(false);
            }}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--text-tertiary)',
              padding: 2,
              display: 'inline-flex',
              alignItems: 'center',
            }}
            title={t('hide')}
          >
            <EyeOff size={14} />
          </button>
        )}
      </span>
    );
  }

  // Sensitive — blurred
  if (level === 'sensitive') {
    return (
      <button
        onClick={async () => {
          if (onReveal) {
            const ok = await onReveal();
            if (ok) setRevealed(true);
          } else {
            setRevealed(true);
          }
        }}
        className={className}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: 6,
          background: 'none',
          border: 'none',
          cursor: onReveal ? 'pointer' : 'default',
          padding: '2px 4px',
          borderRadius: 4,
          fontFamily: 'inherit',
          fontSize: 'inherit',
          backdropFilter: 'blur(6px)',
          WebkitBackdropFilter: 'blur(6px)',
          backgroundColor: 'var(--bg-subtle, rgba(128,128,128,0.08))',
          color: 'transparent',
          textShadow: '0 0 8px var(--text-primary, #000)',
          transition: 'all 0.2s',
        }}
        title={onReveal ? t('click_to_reveal') : t('sensitive_label')}
      >
        <Eye size={14} style={{ color: 'var(--text-secondary)' }} />
        ••••••••
      </button>
    );
  }

  // Critical — locked
  return (
    <button
      onClick={async () => {
        if (onReveal) {
          const ok = await onReveal();
          if (ok) setRevealed(true);
        }
      }}
      className={className}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        background: 'var(--bg-critical, rgba(220,38,38,0.06))',
        border: '1px solid var(--border-critical, rgba(220,38,38,0.2))',
        cursor: onReveal ? 'pointer' : 'default',
        padding: '2px 8px',
        borderRadius: 4,
        fontFamily: 'inherit',
        fontSize: 'inherit',
        color: 'var(--text-critical, #dc2626)',
        transition: 'all 0.2s',
      }}
      title={onReveal ? t('unlock_to_view') : t('critical_label')}
    >
      <Lock size={14} />
      {onReveal ? t('unlock_to_view') : '••••••••'}
    </button>
  );
}
