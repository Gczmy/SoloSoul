import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useLlmChatCore } from '@/hooks/useLlmChatCore';
import { invokeCommand } from '@/lib/ipcClient';
import type { ChatMsg } from '@/types/llmChat';
import { useLlmStore } from '@/stores/llmStore';
import { setRequestSession } from '@/lib/sessionRequests';
import { buildChatRequest, type ChatRequest } from './chatRequest';
import { searchGuideChunks, type GuideChunk } from './guideService';
import { saveConversationSafely } from './conversationPersistence';

const fixtures = vi.hoisted(() => ({
  auth: { currentAccount: { id: 'account' } },
  provider: { id: 'provider', baseUrl: 'https://example.test', model: 'model', apiType: 'openai' },
  includeSystemPrompt: true,
  t: (key: string) => key,
}));
vi.mock('@/lib/i18n', () => ({ default: { language: 'zh-CN' } }));
vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: fixtures.t }) }));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: vi.fn() }));
vi.mock('@/stores/objectStore', () => ({
  useObjectStore: {
    getState: () => ({
      objects: [{ id: 'public-object', sensitivityLevel: 'public', isDeleted: false }],
    }),
  },
}));
vi.mock('@/stores/authStore', () => ({
  useAuthStore: (select: (state: typeof fixtures.auth) => unknown) => select(fixtures.auth),
}));
vi.mock('@/hooks/useLlmProviderConfig', () => ({
  useLlmProviderConfig: () => ({
    activeProvider: fixtures.provider,
    isConfigured: true,
    isAiEnabled: true,
    includeSystemPrompt: fixtures.includeSystemPrompt,
    loading: false,
  }),
}));
vi.mock('@/hooks/useLlmOnlineStatus', () => ({
  useLlmOnlineStatus: () => ({ isOnline: true, checkingOnline: false, checkOnline: vi.fn() }),
}));
vi.mock('@/hooks/useCopyToClipboard', () => ({
  useCopyToClipboard: () => ({ copy: vi.fn(), copiedKey: null }),
}));
vi.mock('@/lib/notification', () => ({ markConversationPending: vi.fn() }));
vi.mock('./conversationPersistence', () => ({
  saveConversationSafely: vi.fn(),
  notifyConversationSaveFailed: vi.fn(),
}));
vi.mock('./guideService', () => ({ searchGuideChunks: vi.fn() }));

const previous: ChatMsg[] = [
  { id: 'one', role: 'user', content: '旧问题', createdAt: '' },
  { id: 'two', role: 'assistant', content: '旧回答', createdAt: '' },
];
const guideChunks: GuideChunk[] = [
  { guideId: 'guide', guideTitle: '指南', chunkText: 'GUIDE', similarity: 0.8 },
];

function contextSelection(include: boolean) {
  return include
    ? { mode: 'publicProfile', objectIds: ['public-object'], language: 'zh-CN', guideChunks }
    : { mode: 'none' };
}

function sentRequest(): ChatRequest {
  const sends = vi
    .mocked(invokeCommand)
    .mock.calls.filter(([command]) => command === 'llm_send_message_stream');
  expect(sends).toHaveLength(1);
  const payload = sends[0][1];
  expect(payload).toEqual({
    accountId: 'account',
    conversationId: expect.any(String),
    requestId: expect.any(String),
    providerId: fixtures.provider.id,
    messages: expect.any(Array),
    contextSelection: expect.any(Object),
  });
  expect(
    vi.mocked(invokeCommand).mock.calls.some(([command]) => command === 'llm_get_api_key'),
  ).toBe(false);
  return payload as unknown as ChatRequest;
}

beforeEach(() => {
  vi.clearAllMocks();
  useLlmStore.getState().reset();
  setRequestSession('account');
  fixtures.includeSystemPrompt = true;
  vi.mocked(searchGuideChunks).mockResolvedValue(guideChunks);
  vi.mocked(saveConversationSafely).mockResolvedValue(true);
  vi.mocked(invokeCommand).mockImplementation(async (command, args) => {
    if (command === 'llm_get_api_key') throw new Error('Ordinary chat must not read credentials');
    if (command === 'llm_get_conversation') {
      const saved = vi.mocked(saveConversationSafely).mock.calls.at(-1)?.[1];
      return saved
        ? {
            ...saved,
            messages: [...saved.messages, { role: 'assistant', content: '', createdAt: '' }],
          }
        : {
            id: (args as { conversationId: string }).conversationId,
            name: 'kept',
            isTemporary: false,
            messages: [],
            updatedAt: '',
          };
    }
    return [];
  });
});

afterEach(() => {
  cleanup();
  useLlmStore.getState().reset();
  setRequestSession(null);
});

describe('chat request message ownership', () => {
  for (const includeSystemPrompt of [false, true]) {
    for (const history of [[], previous]) {
      const scenario = 'system=' + includeSystemPrompt + ', history=' + history.length;
      const expected = [
        ...history.map(({ role, content }) => ({ role, content })),
        { role: 'user', content: '本次问题' },
      ];

      it('builds the final sequence once (' + scenario + ')', async () => {
        const snapshot = structuredClone(history);
        const request = await buildChatRequest({
          accountId: 'account',
          text: '本次问题',
          history,
          includeSystemPrompt,
        });
        expect(request).toEqual({
          messages: expected,
          contextSelection: contextSelection(includeSystemPrompt),
        });
        expect(history).toEqual(snapshot);
        expect(searchGuideChunks).toHaveBeenCalledTimes(includeSystemPrompt ? 1 : 0);
        if (includeSystemPrompt)
          expect(searchGuideChunks).toHaveBeenCalledWith(
            'account',
            '本次问题',
            'zh-CN',
            expect.objectContaining({
              assertCurrent: expect.any(Function),
              invokeTyped: expect.any(Function),
            }),
          );
      });

      it(
        'sends the real hook history without duplicating UI input (' + scenario + ')',
        async () => {
          const { result } = renderHook(() => useLlmChatCore({ includeSystemPrompt }));
          await act(async () => {
            result.current.setMessages(history);
            result.current.setInput('  本次问题  ');
          });
          await act(async () => {
            await result.current.sendMessage();
          });
          const payload = sentRequest();
          expect(payload.messages).toEqual(expected);
          expect(payload.contextSelection).toEqual(contextSelection(includeSystemPrompt));
          expect(result.current.messages.map(({ role, content }) => ({ role, content }))).toEqual([
            ...expected,
            { role: 'assistant', content: '' },
          ]);
          if (!history.length) {
            expect(saveConversationSafely).toHaveBeenCalledWith(
              'account',
              expect.objectContaining({
                messages: [expect.objectContaining({ role: 'user', content: '本次问题' })],
              }),
              fixtures.t,
              expect.objectContaining({
                isCurrent: expect.any(Function),
                invoke: expect.any(Function),
              }),
            );
          }
        },
      );
    }
  }
});

describe('persisted system prompt preference', () => {
  for (const option of [undefined, false, true]) {
    for (const saved of [false, true]) {
      it('honors both switches (option=' + option + ', saved=' + saved + ')', async () => {
        fixtures.includeSystemPrompt = saved;
        const { result } = renderHook(() => useLlmChatCore({ includeSystemPrompt: option }));
        act(() => result.current.setInput('本次问题'));
        await act(async () => {
          await result.current.sendMessage();
        });
        const effective = option !== false && saved !== false;
        expect(sentRequest().contextSelection).toEqual(contextSelection(effective));
        expect(searchGuideChunks).toHaveBeenCalledTimes(effective ? 1 : 0);
      });
    }
  }

  it.each([false, true])(
    'uses a changed saved switch in the existing send callback (%s)',
    async (saved) => {
      fixtures.includeSystemPrompt = !saved;
      const { result, rerender } = renderHook(() => useLlmChatCore({ includeSystemPrompt: true }));
      act(() => result.current.setInput('本次问题'));
      fixtures.includeSystemPrompt = saved;
      rerender();
      await act(async () => {
        await result.current.sendMessage();
      });
      expect(sentRequest().contextSelection).toEqual(contextSelection(saved));
    },
  );
});
