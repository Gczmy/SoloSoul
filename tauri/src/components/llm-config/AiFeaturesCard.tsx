import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';

interface AiFeatures {
  chat: boolean;
  smartFill: boolean;
  commandGen: boolean;
  naturalLanguageSearch: boolean;
}

interface AiFeaturesCardProps {
  features: AiFeatures;
  onToggle: (key: keyof AiFeatures) => void;
}

export function AiFeaturesCard({ features, onToggle }: AiFeaturesCardProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <Card>
      <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 12 }}>
        {t('settings:ai_features')}
      </h3>
      {(['chat', 'smartFill', 'commandGen', 'naturalLanguageSearch'] as const).map((key) => (
        <label
          key={key}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            padding: '6px 0',
            cursor: key === 'chat' ? 'pointer' : 'not-allowed',
            fontSize: 'var(--text-body-sm)',
            opacity: key === 'chat' ? 1 : 0.5,
          }}
        >
          <SelectCheckbox
            checked={features[key]}
            onChange={() => key === 'chat' && onToggle(key)}
            disabled={key !== 'chat'}
          />
          {t('settings:ai_' + key)}
          {key !== 'chat' && (
            <span
              style={{
                fontSize: 'var(--text-badge)',
                color: 'var(--text-tertiary)',
                marginLeft: 4,
              }}
            >
              ({t('settings:ai_in_development')})
            </span>
          )}
        </label>
      ))}
    </Card>
  );
}
