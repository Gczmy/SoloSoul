import { createInstance } from 'i18next';
import zhCommon from '@/locales/zh-CN/common.json';
import enCommon from '@/locales/en-US/common.json';
import { getBackendErrorTranslationKey } from '@/lib/backendError';
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { TFunction } from 'i18next';
import { useLlmStore, selectLlmStream } from '@/stores/llmStore';
import { invoke } from '@tauri-apps/api/core';
import { invokeTypedCommand } from '@/lib/typedIpc';
import {
  BackendCommandError,
  normalizeLlmError,
  readBackendError,
  makeBackendError,
} from '@/lib/backendErrorWire';
import { useUiStore } from '@/stores/uiStore';
import fixtures from '../../../src-tauri/src/commands/llm/contracts/rf317-fixtures.json';
import { useLlmStreaming } from '@/hooks/useLlmStreaming';
import { setRequestSession } from '@/lib/sessionRequests';
vi.mock('@/lib/i18n', () => ({
  default: { language: 'zh-CN', t: (key: string) => key, exists: () => false },
}));
const identity = {
  accountId: 'acc_rf317',
  conversationId: 'synthetic-conversation',
  requestId: 'synthetic-request',
};
const t = ((key: string) => key) as TFunction;
beforeEach(async () => {
  vi.clearAllMocks();
  useLlmStore.getState().reset();
  setRequestSession(identity.accountId);
  const run = useLlmStore.getState().startStream({
    ...identity,
    assistantMessageId: 'assistant',
    messages: [{ id: 'assistant', role: 'assistant', content: '', createdAt: '' }],
  });
  await run!.ready;
});
afterEach(() => {
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
  cleanup();
  useLlmStore.getState().reset();
  setRequestSession(null);
});
it('RF317 legacy upstream rejection is classified without exposing response body in the actual projection', () => {
  const hook = renderHook(() =>
    useLlmStreaming({
      messages: [],
      accountId: identity.accountId,
      currentConvId: identity.conversationId,
      t,
    }),
  );
  act(() =>
    useLlmStore.getState().onChunk({
      ...identity,
      sessionGeneration: 0,
      chunk: '',
      isDone: true,
      error: 'HTTP 401: RF317_SYNTHETIC_SECRET_RESPONSE',
    }),
  );
  const content = hook.result.current.at(-1)!.content;
  expect(content).not.toContain('RF317_SYNTHETIC_SECRET_RESPONSE');
  expect(content).toContain('common:backend_llm_provider_rejected');
});

it.each([
  ['LLM_NETWORK_FAILED', 'common:backend_llm_network_failed'],
  ['LLM_TIMEOUT', 'common:backend_llm_timeout'],
  ['LLM_PROVIDER_REJECTED', 'common:backend_llm_provider_rejected'],
  ['SESSION_EXPIRED', 'common:backend_session_expired'],
] as const)(
  'RF317 structured %s drives the real projection without inspecting English text',
  (code, key) => {
    const hook = renderHook(() =>
      useLlmStreaming({
        messages: [],
        accountId: identity.accountId,
        currentConvId: identity.conversationId,
        t,
      }),
    );
    act(() =>
      useLlmStore.getState().onChunk({
        ...identity,
        sessionGeneration: 0,
        chunk: '',
        isDone: false,
        error: 'RF317_SYNTHETIC_SECRET_RESPONSE',
        failure: makeBackendError(code),
      }),
    );
    expect(hook.result.current.at(-1)!.content).toBe('settings:ai_chat_error_prefix: ' + key);
    expect(
      JSON.stringify(
        selectLlmStream(useLlmStore.getState(), identity.accountId, identity.conversationId),
      ),
    ).not.toContain('RF317_SYNTHETIC_SECRET_RESPONSE');
  },
);
it('RF317 generated reply and a single not-saved notice survive structured persist failure in both consumers', () => {
  const toast = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
  const options = {
    messages: [],
    accountId: identity.accountId,
    currentConvId: identity.conversationId,
    t,
  };
  const page = renderHook(() => useLlmStreaming(options));
  const quick = renderHook(() => useLlmStreaming(options));
  act(() => {
    useLlmStore.getState().onChunk({
      ...identity,
      sessionGeneration: 0,
      chunk: 'Generated reply',
      isDone: true,
      error: null,
    });
    useLlmStore.getState().onChunk({
      ...identity,
      sessionGeneration: 0,
      chunk: '',
      isDone: true,
      error: '__LLM_PERSIST_FAILED__: RF317_SYNTHETIC_STORAGE_DETAIL',
      failure: readBackendError(fixtures.replySaveFailed)!,
    });
    useLlmStore.getState().finishStream(identity);
  });
  expect(page.result.current.at(-1)!.content).toBe('Generated reply');
  expect(quick.result.current.at(-1)!.content).toBe('Generated reply');
  expect(toast).toHaveBeenCalledTimes(1);
  expect(toast).toHaveBeenCalledWith(
    expect.objectContaining({ message: 'common:backend_llm_reply_save_failed' }),
  );
  expect(
    selectLlmStream(useLlmStore.getState(), identity.accountId, identity.conversationId),
  ).toMatchObject({ persistFailure: 'notSaved', error: null, settled: true });
});
it('RF317 missing canonical confirmation reports unknown save status, not a confirmed storage failure', () => {
  const toast = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
  renderHook(() =>
    useLlmStreaming({
      messages: [],
      accountId: identity.accountId,
      currentConvId: identity.conversationId,
      t,
    }),
  );
  act(() => {
    useLlmStore.getState().onChunk({
      ...identity,
      sessionGeneration: 0,
      chunk: 'Generated reply',
      isDone: true,
      error: null,
    });
    useLlmStore.getState().markPersistFailure(identity);
    useLlmStore.getState().finishStream(identity);
  });
  expect(toast).toHaveBeenCalledWith(
    expect.objectContaining({ message: 'common:backend_llm_save_unconfirmed' }),
  );
});
it('RF317 late failure from a locked account is not projected or notified to the new account', () => {
  const toast = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
  const hook = renderHook(() =>
    useLlmStreaming({
      messages: [],
      accountId: identity.accountId,
      currentConvId: identity.conversationId,
      t,
    }),
  );
  act(() => {
    setRequestSession(null);
    setRequestSession('new-account');
    useLlmStore.getState().onChunk({
      ...identity,
      sessionGeneration: 0,
      chunk: '',
      isDone: true,
      error: null,
      failure: readBackendError(fixtures.replySaveFailed)!,
    });
  });
  expect(hook.result.current).toEqual([]);
  expect(toast).not.toHaveBeenCalled();
});
it.each(Object.entries(fixtures))(
  'RF317 actual serialized %s passes the safe reader and cannot retain extra details',
  (_, fixture) => {
    expect(
      readBackendError({
        ...fixture,
        message: 'RF317_SYNTHETIC_SECRET_RESPONSE',
        cause: 'RF317_SYNTHETIC_SECRET_RESPONSE',
      }),
    ).toEqual(fixture);
  },
);
it.each([
  ['HTTP 429: RF317_SYNTHETIC_SECRET_RESPONSE', 'LLM_RATE_LIMITED'],
  [
    'HTTP 500 http://synthetic.private/path — RF317_SYNTHETIC_SECRET_RESPONSE',
    'LLM_PROVIDER_UNAVAILABLE',
  ],
  ['__LLM_PERSIST_FAILED__: RF317_SYNTHETIC_STORAGE_DETAIL', 'LLM_REPLY_SAVE_FAILED'],
  ['Chat provider is disabled', 'LLM_PROVIDER_DISABLED'],
  ['Vault session is no longer current', 'SESSION_EXPIRED'],
])('RF317 old %s compatibility produces only %s', (value, code) => {
  expect(normalizeLlmError(value).code).toBe(code);
  expect(JSON.stringify(normalizeLlmError(value))).not.toContain('RF317_SYNTHETIC');
});
it('RF317 actual typed IPC boundary returns a safe Error for structured and unknown legacy failures', async () => {
  const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
  for (const value of [fixtures.rejected, 'RF317_SYNTHETIC_SECRET_RESPONSE']) {
    vi.mocked(invoke).mockRejectedValueOnce(value);
    const error = await invokeTypedCommand('llm_test_provider', {
      baseUrl: 'http://localhost',
      apiKey: '',
      model: 'synthetic',
      apiType: 'openAI',
    }).catch((error: unknown) => error);
    expect(error).toBeInstanceOf(BackendCommandError);
    expect(JSON.stringify(error)).not.toContain('RF317_SYNTHETIC');
    expect((error as Error).message).toBe(
      typeof value === 'string' ? 'INTERNAL_ERROR' : 'LLM_PROVIDER_REJECTED',
    );
  }
  expect(JSON.stringify(warn.mock.calls)).not.toContain('RF317_SYNTHETIC');
});
it('RF317 invalid structured payload cannot introduce arbitrary data into the current stream', () => {
  const before = selectLlmStream(
    useLlmStore.getState(),
    identity.accountId,
    identity.conversationId,
  );
  const malformed = {
    ...identity,
    sessionGeneration: 0,
    chunk: 'RF317_SYNTHETIC',
    isDone: true,
    error: null,
    failure: { code: '__proto__', retryable: true, safeDetails: null },
  };
  useLlmStore.getState().onChunk(malformed as never);
  expect(selectLlmStream(useLlmStore.getState(), identity.accountId, identity.conversationId)).toBe(
    before,
  );
});

it('RF317 an invoke-only reply-save failure preserves generated text when a terminal event cannot be delivered', () => {
  const toast = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
  const hook = renderHook(() =>
    useLlmStreaming({
      messages: [],
      accountId: identity.accountId,
      currentConvId: identity.conversationId,
      t,
    }),
  );
  act(() => {
    useLlmStore.getState().onChunk({
      ...identity,
      sessionGeneration: 0,
      chunk: 'Generated reply',
      isDone: true,
      error: null,
    });
    useLlmStore.getState().finishStream(identity, readBackendError(fixtures.replySaveFailed)!);
  });
  expect(hook.result.current.at(-1)!.content).toBe('Generated reply');
  expect(toast).toHaveBeenCalledTimes(1);
  expect(toast).toHaveBeenCalledWith(
    expect.objectContaining({ message: 'common:backend_llm_reply_save_failed' }),
  );
});

it.each(['zh-CN', 'en-US'])(
  'RF317 real %s translations resolve every shared error fixture without raw machine keys',
  async (language) => {
    const i18n = createInstance();
    await i18n.init({
      lng: language,
      fallbackLng: false,
      resources: { 'zh-CN': { common: zhCommon }, 'en-US': { common: enCommon } },
      defaultNS: 'common',
      initAsync: false,
    });
    for (const fixture of Object.values(fixtures)) {
      const error = readBackendError(fixture)!;
      const key = getBackendErrorTranslationKey(error.code);
      expect(i18n.exists(key)).toBe(true);
      expect(i18n.t(key)).not.toBe(key);
      expect(i18n.t(key).length).toBeGreaterThan(0);
    }
  },
);
