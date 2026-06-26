import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { MessageSquare, Settings } from 'lucide-react';
import { ICON_SIZE } from '@/lib/iconSizes';


interface UnconfiguredHintProps {
  onClose: () => void;
}

export function UnconfiguredHint({ onClose }: UnconfiguredHintProps) {
  const { t } = useTranslation(['settings', 'common']);
  const navigate = useNavigate();

  return (
    <div
      style={{
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 24,
        gap: 12,
      }}
    >
      <MessageSquare size={ICON_SIZE['3xl']} style={{ opacity: 0.3, color: 'var(--text-tertiary)' }} />
      <p
        style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', textAlign: 'center', margin: 0 }}
      >
        {t('settings:ai_quick_chat_configure_hint')}
      </p>
      <button
        onClick={() => {
          onClose();
          navigate('/settings/llm');
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
          e.currentTarget.style.borderColor = 'var(--accent-primary)';
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.background = 'var(--bg-toolbar)';
          e.currentTarget.style.borderColor = 'var(--border-subtle)';
        }}
        style={{
          padding: '8px 16px',
          borderRadius: 8,
          border: '1px solid var(--border-subtle)',
          background: 'var(--bg-toolbar)',
          color: 'var(--text-primary)',
          fontSize: 'var(--text-body-sm)',
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          transition: 'background 0.2s, border-color 0.2s',
        }}
      >
        <Settings size={ICON_SIZE.sm} /> {t('settings:ai_chat_configure')}
      </button>
    </div>
  );
}
