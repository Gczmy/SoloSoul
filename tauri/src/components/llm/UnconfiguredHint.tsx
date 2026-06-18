import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { MessageSquare, Settings } from 'lucide-react';

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
      <MessageSquare size={36} style={{ opacity: 0.3, color: 'var(--text-tertiary)' }} />
      <p
        style={{ fontSize: 13, color: 'var(--text-secondary)', textAlign: 'center', margin: 0 }}
      >
        {t('settings:ai_quick_chat_configure_hint')}
      </p>
      <button
        onClick={() => {
          onClose();
          navigate('/settings/llm');
        }}
        style={{
          padding: '8px 16px',
          borderRadius: 8,
          border: 'none',
          background: 'var(--accent-primary)',
          color: 'white',
          fontSize: 13,
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          gap: 6,
        }}
      >
        <Settings size={14} /> {t('settings:ai_chat_configure')}
      </button>
    </div>
  );
}
