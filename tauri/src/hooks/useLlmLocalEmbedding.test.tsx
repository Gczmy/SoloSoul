import { act, renderHook, waitFor } from '@testing-library/react';
import type { TFunction } from 'i18next';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useLlmLocalEmbedding } from './useLlmLocalEmbedding';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  onError: vi.fn(),
  onSuccess: vi.fn(),
  requestConfirm: vi.fn(),
}));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));

const t = ((key: string) => key) as TFunction;
const model = {
  id: 'model-a',
  name: 'Model A',
  description: 'Local model',
  disk_size: '80MB',
  dimensions: 384,
  download_url: 'https://example.test/model.zip',
  checksum: 'sha256:abc',
  installed: false,
};

function setup() {
  return renderHook(() =>
    useLlmLocalEmbedding({
      accountId: 'account-a',
      t,
      onError: mocks.onError,
      onSuccess: mocks.onSuccess,
      requestConfirm: mocks.requestConfirm,
    }),
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.invoke.mockImplementation(async (command: string) => {
    if (command === 'llm_check_embedding_available') return true;
    if (command === 'llm_get_embed_models') return [model];
    return undefined;
  });
});

describe('local embedding download preference', () => {
  it('keeps the previous model selection when auto-enable cannot be persisted', async () => {
    const saveError = new Error('profile save failed');
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'llm_check_embedding_available') return true;
      if (command === 'llm_get_embed_models') return [model];
      if (command === 'llm_set_local_embedding') throw saveError;
      return undefined;
    });
    const { result } = setup();
    await waitFor(() => expect(result.current.embedModels).toHaveLength(1));

    await act(async () => {
      await result.current.handleDownloadModel(model.id);
    });

    expect(mocks.invoke).toHaveBeenCalledWith('llm_set_local_embedding', {
      accountId: 'account-a',
      enabled: true,
      modelId: model.id,
    });
    expect(result.current.localModelId).toBeNull();
    expect(result.current.useLocalEmbedding).toBe(false);
    expect(result.current.downloadingId).toBeNull();
    expect(mocks.onSuccess).toHaveBeenCalledWith('settings:llm_model_downloaded');
    expect(mocks.onError).toHaveBeenCalledWith(saveError, 'settings:llm_enable_local_failed');
  });

  it('selects the downloaded model only after auto-enable succeeds', async () => {
    let finishSave!: () => void;
    const save = new Promise<void>((resolve) => {
      finishSave = resolve;
    });
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'llm_check_embedding_available') return true;
      if (command === 'llm_get_embed_models') return [model];
      if (command === 'llm_set_local_embedding') return save;
      return undefined;
    });
    const { result } = setup();
    await waitFor(() => expect(result.current.embedModels).toHaveLength(1));

    let download!: Promise<void>;
    act(() => {
      download = result.current.handleDownloadModel(model.id);
    });
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('llm_set_local_embedding', {
        accountId: 'account-a',
        enabled: true,
        modelId: model.id,
      }),
    );
    expect(result.current.localModelId).toBeNull();

    await act(async () => {
      finishSave();
      await download;
    });
    expect(result.current.localModelId).toBe(model.id);
    expect(result.current.useLocalEmbedding).toBe(true);
    expect(mocks.onError).not.toHaveBeenCalled();
  });
});

describe('local embedding model selection', () => {
  it('persists an installed model choice while local embedding is disabled', async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'llm_check_embedding_available') return true;
      if (command === 'llm_get_embed_models') return [{ ...model, installed: true }];
      return undefined;
    });
    const { result } = setup();
    await waitFor(() => expect(result.current.embedModels).toHaveLength(1));

    await act(async () => {
      await result.current.handleSelectLocalModel(model.id);
    });

    expect(mocks.invoke).toHaveBeenCalledWith('llm_set_local_embedding', {
      accountId: 'account-a',
      enabled: false,
      modelId: model.id,
    });
    expect(result.current.localModelId).toBe(model.id);
  });

  it('restores the previous choice if disabled-model selection cannot be saved', async () => {
    const saveError = new Error('profile save failed');
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'llm_check_embedding_available') return true;
      if (command === 'llm_get_embed_models') return [{ ...model, installed: true }];
      if (command === 'llm_set_local_embedding') throw saveError;
      return undefined;
    });
    const { result } = setup();
    await waitFor(() => expect(result.current.embedModels).toHaveLength(1));

    await act(async () => {
      await result.current.handleSelectLocalModel(model.id);
    });

    expect(result.current.localModelId).toBeNull();
    expect(mocks.onError).toHaveBeenCalledWith(saveError, 'settings:llm_select_local_model_failed');
  });

  it('preserves enabled state and the previous model if switching an active model fails', async () => {
    const saveError = new Error('profile save failed');
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'llm_check_embedding_available') return true;
      if (command === 'llm_get_embed_models') return [{ ...model, installed: true }];
      if (command === 'llm_set_local_embedding') throw saveError;
      return undefined;
    });
    const { result } = setup();
    await waitFor(() => expect(result.current.embedModels).toHaveLength(1));
    act(() => {
      result.current.setLocalModelId('old-model');
      result.current.setUseLocalEmbedding(true);
    });

    await act(async () => {
      await result.current.handleSelectLocalModel(model.id);
    });

    expect(mocks.invoke).toHaveBeenCalledWith('llm_set_local_embedding', {
      accountId: 'account-a',
      enabled: true,
      modelId: model.id,
    });
    expect(result.current.localModelId).toBe('old-model');
    expect(result.current.useLocalEmbedding).toBe(true);
    expect(mocks.onError).toHaveBeenCalledWith(saveError, 'settings:llm_enable_local_failed');
  });
});
