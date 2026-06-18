import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';

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
      <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
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
            fontSize: 13,
            opacity: key === 'chat' ? 1 : 0.5,
          }}
        >
          <input
            type="checkbox"
            checked={features[key]}
            onChange={() => key === 'chat' && onToggle(key)}
            disabled={key !== 'chat'}
            style={{ accentColor: 'var(--accent-primary)' }}
          />
          {t('settings:ai_' + key)}
          {key !== 'chat' && (
            <span style={{ fontSize: 11, color: 'var(--text-tertiary)', marginLeft: 4 }}>
              ({t('settings:ai_in_development')})
            </span>
          )}
        </label>
      ))}
    </Card>
  );
}
