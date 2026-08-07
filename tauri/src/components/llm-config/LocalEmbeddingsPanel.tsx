import { useTranslation } from 'react-i18next';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { Cpu, Download } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { ICON_SIZE } from '@/lib/constants';

/**
 * 与后端 `llm_get_embed_models`（embed_model.rs `EmbedModelWithStatus`）实际序列化形状
 * 保持一致：`#[serde(flatten)]` 扁平字段 + snake_case + installed 标志。
 * 此前误设为嵌套 `{ info, installed }` + camelCase，真实数据到达后渲染
 * `m.info.id` 抛 TypeError 导致整页卸载（页面无 ErrorBoundary）。
 */
interface EmbedModelWithStatus {
  id: string;
  name: string;
  description: string;
  disk_size: string;
  dimensions: number;
  download_url: string;
  checksum: string;
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
        <SelectCheckbox checked={useLocalEmbedding} onChange={(v) => onToggle(v)} />
        <span>{t('settings:llm_use_local_embedding')}</span>
      </label>

      {modelsLoading ? (
        <LoadingPlaceholder variant="base" minHeight={80} />
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {embedModels.map((m) => (
            <div
              key={m.id}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                padding: '10px 12px',
                borderRadius: 8,
                background:
                  localModelId === m.id && useLocalEmbedding
                    ? 'rgba(91,124,153,0.08)'
                    : 'var(--bg-toolbar)',
                border:
                  localModelId === m.id && useLocalEmbedding
                    ? '1px solid var(--accent-primary)'
                    : '1px solid var(--border-subtle)',
                fontSize: 'var(--text-body-sm)',
              }}
            >
              <input
                type="radio"
                checked={localModelId === m.id}
                onChange={() => onSelectModel(m.id)}
                disabled={!m.installed}
                style={{ accentColor: 'var(--accent-primary)' }}
              />
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: 500 }}>{m.name}</div>
                <div
                  style={{
                    fontSize: 'var(--text-badge)',
                    color: 'var(--text-tertiary)',
                    marginTop: 2,
                  }}
                >
                  {m.description} · {m.dimensions}
                  {t('settings:llm_dimensions')} · {m.disk_size}
                </div>
                {downloadingId === m.id && (
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
                          background:
                            'linear-gradient(90deg, var(--accent-primary), var(--accent-warm))',
                          transition: 'width 0.3s',
                        }}
                      />
                    </div>
                    <div
                      style={{
                        fontSize: 'var(--text-badge)',
                        color: 'var(--text-tertiary)',
                        marginTop: 2,
                      }}
                    >
                      {downloadProgress}%
                    </div>
                  </div>
                )}
              </div>
              {m.installed ? (
                <DeleteButton
                  onClick={() => onDelete(m.id)}
                  title={t('settings:llm_delete_model')}
                  iconOnly
                />
              ) : (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => onDownload(m.id)}
                  loading={downloadingId === m.id}
                  disabled={downloadingId !== null && downloadingId !== m.id}
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
