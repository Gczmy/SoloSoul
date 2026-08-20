/**
 * LlmConfigPage 本地 embedding 域：可用性探测、模型列表、下载/删除/选择、
 * 下载进度事件、知识库重建。
 */
import { useState, useEffect, useCallback } from 'react';
import type { TFunction } from 'i18next';
import i18n from '@/lib/i18n';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { listen } from '@tauri-apps/api/event';
import { logger } from '@/lib/logger';

/**
 * 与后端 `llm_get_embed_models`（embed_model.rs `EmbedModelWithStatus`）实际序列化形状
 * 保持一致：`#[serde(flatten)]` 扁平字段 + snake_case + installed 标志。
 * 此前误设为嵌套 `{ info, installed }` + camelCase，真实数据到达后渲染
 * `m.info.id` 抛 TypeError → 整页卸载（页面无 ErrorBoundary）。
 */
export interface EmbedModelWithStatus {
  id: string;
  name: string;
  description: string;
  disk_size: string;
  dimensions: number;
  download_url: string;
  checksum: string;
  installed: boolean;
}

export interface UseLlmLocalEmbeddingOptions {
  accountId?: string;
  t: TFunction;
  onError: (err: unknown, context: string) => void;
  onSuccess: (message: string) => void;
  requestConfirm: (
    title: string,
    message: string,
    onConfirm: () => void,
    options?: { confirmLabel?: string; cancelLabel?: string },
  ) => void;
}

export function useLlmLocalEmbedding({
  accountId,
  t,
  onError,
  onSuccess,
  requestConfirm,
}: UseLlmLocalEmbeddingOptions) {
  const [embeddingAvailable, setEmbeddingAvailable] = useState<boolean | null>(null);
  const [embedModels, setEmbedModels] = useState<EmbedModelWithStatus[]>([]);
  const [useLocalEmbedding, setUseLocalEmbedding] = useState(false);
  const [localModelId, setLocalModelId] = useState<string | null>(null);
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<number>(0);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [rebuilding, setRebuilding] = useState(false);

  // 可用性探测
  useEffect(() => {
    if (!accountId) return;
    invoke<boolean>('llm_check_embedding_available', { accountId: accountId })
      .then((avail) => setEmbeddingAvailable(avail))
      .catch(() => setEmbeddingAvailable(false));
  }, [accountId]);

  // 模型列表加载
  const loadEmbedModels = useCallback(async () => {
    setModelsLoading(true);
    try {
      const models = await invoke<EmbedModelWithStatus[]>('llm_get_embed_models');
      setEmbedModels(models);
    } catch (err) {
      // P227: 模型列表加载失败静默降级可接受（UI 仍有重试入口），但留痕。
      logger.warn('[LLMConfig] loadEmbedModels failed:', err);
    } finally {
      setModelsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadEmbedModels();
  }, [loadEmbedModels]);

  // 下载进度事件
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ modelId: string; progress: number }>('embed-download-progress', (event) => {
      if (event.payload.modelId === downloadingId) {
        setDownloadProgress(event.payload.progress);
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, [downloadingId]);

  const handleDownloadModel = async (modelId: string) => {
    setDownloadingId(modelId);
    setDownloadProgress(0);
    try {
      await invoke('llm_download_embed_model', { modelId: modelId });
      onSuccess(t('settings:llm_model_downloaded'));
      await loadEmbedModels();
      if (!localModelId) {
        setLocalModelId(modelId);
        if (accountId) {
          await invoke('llm_set_local_embedding', {
            accountId: accountId,
            enabled: true,
            modelId: modelId,
          });
          setUseLocalEmbedding(true);
        }
      }
    } catch (e) {
      onError(e, t('settings:llm_model_download_failed'));
    } finally {
      setDownloadingId(null);
      setDownloadProgress(0);
    }
  };

  const handleDeleteModel = (modelId: string) => {
    requestConfirm(
      t('settings:llm_delete_model_title', { defaultValue: 'Delete model' }),
      t('settings:llm_confirm_delete_model', {
        defaultValue: 'Delete this local embedding model?',
      }),
      async () => {
        try {
          await invoke('llm_delete_embed_model', { modelId: modelId });
          onSuccess(t('settings:llm_model_deleted'));
          await loadEmbedModels();
          if (localModelId === modelId) {
            setLocalModelId(null);
            setUseLocalEmbedding(false);
            if (accountId) {
              await invoke('llm_set_local_embedding', {
                accountId: accountId,
                enabled: false,
                modelId: null,
              });
            }
          }
        } catch (e) {
          onError(e, t('settings:llm_delete_model_failed'));
        }
      },
      {
        confirmLabel: t('common:delete', { defaultValue: 'Delete' }),
        cancelLabel: t('common:cancel', { defaultValue: 'Cancel' }),
      },
    );
  };

  const handleToggleLocalEmbedding = async (enabled: boolean) => {
    if (!accountId) return;
    if (enabled && !localModelId && embedModels.length > 0) {
      const firstInstalled = embedModels.find((m) => m.installed);
      if (firstInstalled) {
        const prevModelId = localModelId;
        setLocalModelId(firstInstalled.id);
        // P004: invoke 失败回滚模型选择并提示，避免「前端已选、后端未生效」漂移。
        try {
          await invoke('llm_set_local_embedding', {
            accountId: accountId,
            enabled: true,
            modelId: firstInstalled.id,
          });
        } catch (e) {
          setLocalModelId(prevModelId);
          onError(e, t('settings:llm_enable_local_failed'));
          return;
        }
      } else {
        onError(
          new Error(t('settings:llm_enable_local_first')),
          t('settings:llm_enable_local_failed'),
        );
        return;
      }
    } else {
      try {
        await invoke('llm_set_local_embedding', {
          accountId: accountId,
          enabled,
          modelId: localModelId,
        });
      } catch (e) {
        // P004: 开关失败不回改 useLocalEmbedding，仅提示。
        onError(e, t('settings:llm_enable_local_failed'));
        return;
      }
    }
    setUseLocalEmbedding(enabled);
  };

  const handleSelectLocalModel = async (modelId: string) => {
    if (!accountId) return;
    const prevModelId = localModelId;
    setLocalModelId(modelId);
    if (useLocalEmbedding) {
      // P004: invoke 失败回滚模型选择并提示，避免前后端状态漂移。
      try {
        await invoke('llm_set_local_embedding', {
          accountId: accountId,
          enabled: true,
          modelId: modelId,
        });
      } catch (e) {
        setLocalModelId(prevModelId);
        onError(e, t('settings:llm_enable_local_failed'));
      }
    }
  };

  const handleRebuildEmbeddings = async () => {
    if (!accountId) return;
    setRebuilding(true);
    try {
      const count = await invoke<number>('llm_rebuild_guide_embeddings', {
        accountId: accountId,
        language: i18n.language || 'zh-CN',
      });
      onSuccess(t('settings:llm_kb_rebuilt', { count: String(count) }));
    } catch (e) {
      onError(e, t('settings:llm_rebuild_kb'));
    } finally {
      setRebuilding(false);
    }
  };

  return {
    embeddingAvailable,
    embedModels,
    useLocalEmbedding,
    localModelId,
    downloadingId,
    downloadProgress,
    modelsLoading,
    rebuilding,
    setUseLocalEmbedding,
    setLocalModelId,
    handleDownloadModel,
    handleDeleteModel,
    handleToggleLocalEmbedding,
    handleSelectLocalModel,
    handleRebuildEmbeddings,
  };
}
