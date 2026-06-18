import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';

interface SystemPromptCardProps {
  checked: boolean;
  onToggle: () => void;
}

export function SystemPromptCard({ checked, onToggle }: SystemPromptCardProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <Card>
      <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
        {t('settings:ai_system_prompt_title')}
      </h3>
      <label
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '6px 0',
          cursor: 'pointer',
          fontSize: 13,
        }}
      >
        <input
          type="checkbox"
          checked={checked}
          onChange={onToggle}
          style={{ accentColor: 'var(--accent-primary)' }}
        />
        {t('settings:ai_system_prompt_software')}
      </label>
      <p style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 4, paddingLeft: 26 }}>
        {t('settings:ai_system_prompt_desc')}
      </p>
    </Card>
  );
}
