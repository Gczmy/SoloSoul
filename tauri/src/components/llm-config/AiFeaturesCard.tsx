import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';

interface AiFeaturesCardProps {
  chatEnabled: boolean;
  onToggle: () => void;
}

export function AiFeaturesCard({ chatEnabled, onToggle }: AiFeaturesCardProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <Card>
      <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 12 }}>
        {t('settings:ai_features')}
      </h3>
      <label
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '6px 0',
          cursor: 'pointer',
          fontSize: 'var(--text-body-sm)',
        }}
      >
        <SelectCheckbox checked={chatEnabled} onChange={onToggle} />
        {t('settings:ai_chat')}
      </label>
    </Card>
  );
}
