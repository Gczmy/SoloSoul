import { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { X } from 'lucide-react';
import { getSchemesByMode, applyScheme, THEME_SCHEMES, type ThemeScheme } from '@/lib/themeSchemes';

interface ThemeSchemePanelProps {
  isOpen: boolean;
  onClose: () => void;
  defaultLightTheme: string;
  defaultDarkTheme: string;
  currentThemeMode: 'light' | 'dark' | 'system';
  onSelectScheme: (scheme: ThemeScheme) => void;
}

export function ThemeSchemePanel({
  isOpen,
  onClose,
  defaultLightTheme,
  defaultDarkTheme,
  currentThemeMode,
  onSelectScheme,
}: ThemeSchemePanelProps) {
  const { t } = useTranslation(['settings', 'common']);
  // Initialize panel mode from the currently effective theme mode
  const effectiveMode: 'light' | 'dark' = useMemo(() => {
    if (currentThemeMode === 'system') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }
    return currentThemeMode;
  }, [currentThemeMode]);

  const [panelMode, setPanelMode] = useState<'light' | 'dark'>(effectiveMode);

  const schemes = useMemo(() => getSchemesByMode(panelMode), [panelMode]);
  const activeId = panelMode === 'light' ? defaultLightTheme : defaultDarkTheme;

  if (!isOpen) return null;

  return (
    <div
      style={{
        width: 280,
        flexShrink: 0,
        height: '100%',
        background: 'var(--bg-elevated)',
        borderRight: '1px solid var(--border-subtle)',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        animation: 'slideIn 0.2s ease',
      }}
    >
      <style>{`@keyframes slideIn { from { opacity: 0; transform: translateX(-12px); } to { opacity: 1; transform: translateX(0); } }`}</style>

      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '16px 16px 12px',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <h3 style={{ fontSize: 15, fontWeight: 600, margin: 0 }}>{t('settings:theme_schemes')}</h3>
        <button
          onClick={onClose}
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--text-tertiary)',
            padding: 4,
            borderRadius: 6,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
          title={t('common:close')}
        >
          <X size={18} />
        </button>
      </div>

      {/* Mode tabs */}
      <div style={{ padding: '12px 16px', display: 'flex', gap: 8 }}>
        <ModePill
          active={panelMode === 'light'}
          onClick={() => setPanelMode('light')}
          label={t('settings:light_themes')}
        />
        <ModePill
          active={panelMode === 'dark'}
          onClick={() => setPanelMode('dark')}
          label={t('settings:dark_themes')}
        />
      </div>

      {/* Scheme grid */}
      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '0 16px 16px',
          display: 'grid',
          gridTemplateColumns: 'repeat(2, 1fr)',
          gap: 12,
        }}
      >
        {schemes.map((scheme) => (
          <SchemeThumbnail
            key={scheme.id}
            scheme={scheme}
            isActive={activeId === scheme.id}
            onClick={() => onSelectScheme(scheme)}
            defaultLabel={t('settings:default_theme_badge')}
          />
        ))}
      </div>
    </div>
  );
}

function ModePill({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      style={{
        flex: 1,
        padding: '6px 0',
        borderRadius: 8,
        border: '1px solid',
        borderColor: active ? 'var(--accent-primary)' : 'var(--border-subtle)',
        background: active ? 'var(--state-selected)' : 'transparent',
        color: active ? 'var(--accent-primary)' : 'var(--text-primary)',
        fontSize: 13,
        fontWeight: 500,
        cursor: 'pointer',
        transition: 'all 0.15s ease',
      }}
    >
      {label}
    </button>
  );
}

function SchemeThumbnail({
  scheme,
  isActive,
  onClick,
  defaultLabel,
}: {
  scheme: ThemeScheme;
  isActive: boolean;
  onClick: () => void;
  defaultLabel: string;
}) {
  const { t } = useTranslation('settings');

  return (
    <button
      onClick={onClick}
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: 8,
        padding: 10,
        borderRadius: 12,
        border: '2px solid',
        borderColor: isActive ? 'var(--accent-primary)' : 'transparent',
        background: 'var(--bg-base)',
        cursor: 'pointer',
        transition: 'transform 0.15s, box-shadow 0.15s',
        position: 'relative',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.transform = 'translateY(-2px)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.transform = 'translateY(0)';
      }}
    >
      {/* Thumbnail preview */}
      <div
        style={{
          width: 64,
          height: 80,
          borderRadius: 8,
          background: scheme.preview.bg,
          border: '1px solid var(--border-subtle)',
          overflow: 'hidden',
          position: 'relative',
          boxShadow: 'inset 0 0 0 1px rgba(0,0,0,0.03)',
        }}
      >
        {/* Elevated card stub */}
        <div
          style={{
            position: 'absolute',
            top: 8,
            left: 8,
            right: 8,
            height: 36,
            borderRadius: 4,
            background: scheme.preview.elevated,
            boxShadow: '0 1px 2px rgba(0,0,0,0.06)',
          }}
        />
        {/* Accent stub */}
        <div
          style={{
            position: 'absolute',
            bottom: 10,
            left: 8,
            width: 20,
            height: 4,
            borderRadius: 2,
            background: scheme.preview.accent,
          }}
        />
        {/* Text stub */}
        <div
          style={{
            position: 'absolute',
            bottom: 10,
            right: 8,
            width: 12,
            height: 4,
            borderRadius: 2,
            background: scheme.preview.text,
            opacity: 0.6,
          }}
        />
      </div>

      {/* Name */}
      <span
        style={{
          fontSize: 12,
          fontWeight: 500,
          color: 'var(--text-primary)',
          textAlign: 'center',
          lineHeight: 1.2,
        }}
      >
        {t(scheme.nameKey.replace('settings:', '') as any)}
      </span>

      {/* Default badge */}
      {isActive && (
        <span
          style={{
            position: 'absolute',
            top: 6,
            right: 6,
            fontSize: 10,
            fontWeight: 600,
            padding: '2px 6px',
            borderRadius: 4,
            background: 'var(--accent-primary)',
            color: '#fff',
          }}
        >
          {defaultLabel}
        </span>
      )}
    </button>
  );
}
