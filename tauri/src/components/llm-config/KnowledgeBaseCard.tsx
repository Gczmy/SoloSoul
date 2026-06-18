import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';

interface KnowledgeBaseCardProps {
  embeddingAvailable: boolean | null;
  rebuilding: boolean;
  onRebuild: () => void;
}

export function KnowledgeBaseCard({ embeddingAvailable, rebuilding, onRebuild }: KnowledgeBaseCardProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <Card>
      <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
        {t('settings:llm_knowledge_base_title')}
      </h3>
      <p
        style={{
          fontSize: 12,
          color: 'var(--text-tertiary)',
          marginBottom: 12,
          lineHeight: 1.5,
        }}
      >
        {embeddingAvailable === true
          ? t('settings:llm_kb_embedding_supported')
          : embeddingAvailable === false
            ? t('settings:llm_kb_embedding_unsupported')
            : t('settings:llm_kb_embedding_checking')}
      </p>
      <Button
        variant="secondary"
        size="sm"
        onClick={onRebuild}
        loading={rebuilding}
        disabled={embeddingAvailable === false}
      >
        {rebuilding ? t('settings:llm_rebuilding') : t('settings:llm_rebuild_kb')}
      </Button>
    </Card>
  );
}
