import { useTranslation } from 'react-i18next';
import { Cpu, Download, Trash2 } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { ICON_SIZE } from '@/lib/iconSizes';


interface EmbedModelInfo {
  id: string;
  name: string;
  description: string;
  diskSize: string;
  dimensions: number;
  downloadUrl: string;
  checksum: string;
}

interface EmbedModelWithStatus {
  info: EmbedModelInfo;
  installed: boolean;
}

interface LocalEmbeddingsPanelProps {
  useLocalEmbedding: boolean;
  localModelId: string | null;
  embedModels: EmbedModelWithStatus[];
  downloadingId: string | null;
  downloadProgress: number;
  modelsLoading: boolean;
  onToggle: (enabled: boolean) => void;
  onSelectModel: (modelId: string) => void;
  onDownload: (modelId: string) => void;
  onDelete: (modelId: string) => void;
}

export function LocalEmbeddingsPanel({
  useLocalEmbedding,
  localModelId,
  embedModels,
  downloadingId,
  downloadProgress,
  modelsLoading,
  onToggle,
  onSelectModel,
  onDownload,
  onDelete,
}: LocalEmbeddingsPanelProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <Card>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
        <Cpu size={ICON_SIZE.lg} color="var(--accent-primary)" />
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600 }}>
          {t('settings:llm_embed_models_title')}
        </h3>
      </div>
      <p
        style={{
          fontSize: 'var(--text-caption)',
          color: 'var(--text-tertiary)',
          marginBottom: 12,
          lineHeight: 1.5,
        }}
      >
        {t('settings:llm_embed_models_desc')}
      </p>

      <label
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '6px 0',
          cursor: 'pointer',
          fontSize: 'var(--text-body-sm)',
          marginBottom: 12,
        }}
      >
        <input
          type="checkbox"
          checked={useLocalEmbedding}
          onChange={(e) => onToggle(e.target.checked)}
          style={{ accentColor: 'var(--accent-primary)' }}
        />
        <span>{t('settings:llm_use_local_embedding')}</span>
      </label>

      {modelsLoading ? (
        <LoadingPlaceholder variant="base" minHeight={80} />
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {embedModels.map((m) => (
            <div
              key={m.info.id}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                padding: '10px 12px',
                borderRadius: 8,
                background:
                  localModelId === m.info.id && useLocalEmbedding
                    ? 'rgba(91,124,153,0.08)'
                    : 'var(--bg-toolbar)',
                border:
                  localModelId === m.info.id && useLocalEmbedding
                    ? '1px solid var(--accent-primary)'
                    : '1px solid var(--border-subtle)',
                fontSize: 'var(--text-body-sm)',
              }}
            >
              <input
                type="radio"
                checked={localModelId === m.info.id}
                onChange={() => onSelectModel(m.info.id)}
                disabled={!m.installed}
                style={{ accentColor: 'var(--accent-primary)' }}
              />
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: 500 }}>{m.info.name}</div>
                <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)', marginTop: 2 }}>
                  {m.info.description} · {m.info.dimensions}
                  {t('settings:llm_dimensions')} · {m.info.diskSize}
                </div>
                {downloadingId === m.info.id && (
                  <div style={{ marginTop: 6 }}>
                    <div
                      style={{
                        height: 4,
                        background: 'var(--bg-elevated)',
                        borderRadius: 2,
                        overflow: 'hidden',
                      }}
                    >
                      <div
                        style={{
                          width: `${downloadProgress}%`,
                          height: '100%',
                          background: 'var(--accent-primary)',
                          transition: 'width 0.3s',
                        }}
                      />
                    </div>
                    <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)', marginTop: 2 }}>
                      {downloadProgress}%
                    </div>
                  </div>
                )}
              </div>
              {m.installed ? (
                <button
                  onClick={() => onDelete(m.info.id)}
                  style={{
                    padding: 6,
                    borderRadius: 6,
                    border: 'none',
                    background: 'transparent',
                    cursor: 'pointer',
                    color: '#e74c3c',
                  }}
                  title={t('settings:llm_delete_model')}
                >
                  <Trash2 size={ICON_SIZE.sm} />
                </button>
              ) : (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => onDownload(m.info.id)}
                  loading={downloadingId === m.info.id}
                  disabled={downloadingId !== null && downloadingId !== m.info.id}
                >
                  <Download size={ICON_SIZE.sm} style={{ marginRight: 4 }} />
                  {t('settings:llm_download')}
                </Button>
              )}
            </div>
          ))}
          {embedModels.length === 0 && (
            <p style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
              {t('settings:llm_no_models')}
            </p>
          )}
        </div>
      )}
    </Card>
  );
}
