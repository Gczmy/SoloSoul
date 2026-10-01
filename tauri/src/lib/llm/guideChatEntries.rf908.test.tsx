import {
  act,
  cleanup,
  fireEvent,
  render,
  renderHook,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { useLlmChat } from '@/pages/ai/LlmChatPage/useLlmChat';
import { AiQuickChatPopover } from '@/components/layout/AiQuickChatPopover';
import { useLlmStore } from '@/stores/llmStore';
import { setRequestSession } from '@/lib/sessionRequests';
import type { ChatMsg, Conversation } from '@/types/llmChat';
import type { GuideChunk } from './guideService';
import requests from '../../../src-tauri/src/commands/llm/rf908-requests.json';

const fixtures = vi.hoisted(() => ({
  auth: { currentAccount: { id: 'acc_rf908' } as { id: string } | null },
  provider: {
    id: 'provider',
    name: 'Synthetic',
    baseUrl: 'https://example.test',
    model: 'model',
    apiType: 'openai',
  },
  includeSystemPrompt: true,
  t: (key: string) => key,
  quickSend: null as null | (() => Promise<void>),
}));
vi.mock('@/lib/i18n', () => ({ default: { language: 'zh-CN' } }));
vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: fixtures.t }) }));
vi.mock('@/stores/authStore', () => ({
  useAuthStore: (select: (state: typeof fixtures.auth) => unknown) => select(fixtures.auth),
}));
vi.mock('@/stores/objectStore', () => ({
  useObjectStore: {
    getState: () => ({
      objects: [
        { id: 'public-object', sensitivityLevel: 'public', isDeleted: false },
        { id: 'private-object', sensitivityLevel: 'sensitive', isDeleted: false },
        { id: 'deleted-object', sensitivityLevel: 'public', isDeleted: true },
      ],
    }),
  },
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
vi.mock('@/hooks/useToastError', () => ({ useToastError: () => ({ onError: vi.fn() }) }));
vi.mock('@/lib/notification', () => ({
  markConversationPending: vi.fn(),
  setAiPageOpen: vi.fn(),
  setQuickChatOpen: vi.fn(),
}));
// 仅替换展示控件。页面/浮窗、core、builder、guide service、session ticket、IPC 与预保存均使用生产实现。
vi.mock('@/components/llm/ChatMessageList', () => ({
  ChatMessageList: ({ messages }: { messages: ChatMsg[] }) => (
    <div data-testid="messages">{messages.map((m) => m.content).join('|')}</div>
  ),
}));
vi.mock('@/components/llm/ChatInputBar', () => ({
  ChatInputBar: ({
    input,
    onInputChange,
    onSend,
  }: {
    input: string;
    onInputChange: (value: string) => void;
    onSend: () => Promise<void>;
  }) => {
    fixtures.quickSend = onSend;
    return (
      <input
        aria-label="quick-draft"
        value={input}
        onChange={(e) => onInputChange(e.target.value)}
      />
    );
  },
}));

const chunks: GuideChunk[] = [
  { guideId: 'rf908-guide', guideTitle: '合成指南', chunkText: 'RF908_GUIDE', similarity: 0.8 },
];
const saved = new Map<string, Conversation>();
const calls = (name: string) =>
  vi.mocked(invoke).mock.calls.filter(([command]) => command === name);
function openEntry(entry: 'ordinary' | 'quick') {
  if (entry === 'ordinary') {
    const hook = renderHook(() => useLlmChat());
    return {
      draft: (value: string) => act(() => hook.result.current.setInput(value)),
      send: () => hook.result.current.sendMessage(),
      refresh: () => hook.rerender(),
      messages: () => hook.result.current.messages.map((m) => m.content).join('|'),
    };
  }
  const view = render(
    <MemoryRouter>
      <AiQuickChatPopover onClose={vi.fn()} />
    </MemoryRouter>,
  );
  return {
    draft: (value: string) =>
      fireEvent.change(screen.getByLabelText('quick-draft'), { target: { value } }),
    send: () => fixtures.quickSend!(),
    refresh: () =>
      view.rerender(
        <MemoryRouter>
          <AiQuickChatPopover onClose={vi.fn()} />
        </MemoryRouter>,
      ),
    messages: () => screen.getByTestId('messages').textContent ?? '',
  };
}
beforeEach(() => {
  vi.clearAllMocks();
  saved.clear();
  localStorage.clear();
  useLlmStore.getState().reset();
  fixtures.auth.currentAccount = { id: requests.bound.accountId };
  fixtures.includeSystemPrompt = true;
  setRequestSession(requests.bound.accountId);
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    if (command === 'llm_search_guide_chunks') return chunks;
    if (command === 'llm_save_conversation') {
      const value = (args as { conversation: Conversation }).conversation;
      saved.set(value.id, value);
      return null;
    }
    if (command === 'llm_get_conversation') {
      const value = saved.get((args as { conversationId: string }).conversationId)!;
      return {
        ...value,
        messages: [
          ...value.messages,
          { role: 'assistant', content: 'synthetic reply', createdAt: '' },
        ],
      };
    }
    if (command === 'llm_get_api_key') throw new Error('Must not read a provider credential');
    return [];
  });
});
afterEach(() => {
  cleanup();
  useLlmStore.getState().reset();
  setRequestSession(null);
  localStorage.clear();
});

for (const entry of ['ordinary', 'quick'] as const)
  describe(`RF908 actual ${entry} chat entry`, () => {
    it.each([true, false])(
      'uses the saved context switch=%s and forwards guide chunks with the original account',
      async (include) => {
        fixtures.includeSystemPrompt = include;
        const view = openEntry(entry);
        view.draft(requests.bound.query);
        await act(async () => {
          await view.send();
        });
        expect(calls('llm_search_guide_chunks')).toEqual(
          include ? [['llm_search_guide_chunks', requests.bound]] : [],
        );
        expect(calls('llm_save_conversation')).toHaveLength(1);
        expect(calls('llm_save_conversation')[0][1]).toMatchObject({
          accountId: requests.bound.accountId,
          conversation: { messages: [{ role: 'user', content: requests.bound.query }] },
        });
        expect(calls('llm_send_message_stream')).toHaveLength(1);
        expect(calls('llm_send_message_stream')[0][1]).toMatchObject({
          accountId: requests.bound.accountId,
          messages: [{ role: 'user', content: requests.bound.query }],
          contextSelection: include
            ? {
                mode: 'publicProfile',
                language: 'zh-CN',
                objectIds: ['public-object'],
                guideChunks: chunks,
              }
            : { mode: 'none' },
        });
        expect(calls('llm_get_api_key')).toEqual([]);
        expect(view.messages()).toContain('synthetic reply');
      },
    );
    it.each([
      { result: [] as GuideChunk[] },
      { result: [{ ...chunks[0], similarity: null }] as GuideChunk[] },
    ])(
      'keeps populated/empty retrieval and normalizes the actual nullable wire score: %j',
      async ({ result }) => {
        const original = vi.mocked(invoke).getMockImplementation()!;
        vi.mocked(invoke).mockImplementation((command, args) =>
          command === 'llm_search_guide_chunks' ? Promise.resolve(result) : original(command, args),
        );
        const view = openEntry(entry);
        view.draft(requests.bound.query);
        await act(async () => {
          await view.send();
        });
        expect(calls('llm_search_guide_chunks')).toEqual([
          ['llm_search_guide_chunks', requests.bound],
        ]);
        expect(calls('llm_send_message_stream')).toHaveLength(1);
        expect(calls('llm_send_message_stream')[0][1]).toMatchObject({
          accountId: requests.bound.accountId,
          contextSelection: {
            guideChunks: result.map((chunk) => ({ ...chunk, similarity: chunk.similarity ?? 0 })),
          },
        });
        expect(view.messages()).toContain('synthetic reply');
      },
    );
    it.each([null, 'new-account'])(
      'discards a delayed guide response after lock/switch to %s',
      async (nextAccount) => {
        let resolve!: (result: GuideChunk[]) => void;
        const pending = new Promise<GuideChunk[]>((ok) => {
          resolve = ok;
        });
        vi.mocked(invoke).mockImplementationOnce(async () => []); // 初次会话列表读取
        vi.mocked(invoke).mockImplementation(async (command) =>
          command === 'llm_search_guide_chunks' ? pending : [],
        );
        const view = openEntry(entry);
        view.draft(requests.bound.query);
        let sending!: Promise<void>;
        await act(async () => {
          sending = view.send();
        });
        await waitFor(() =>
          expect(calls('llm_search_guide_chunks')).toEqual([
            ['llm_search_guide_chunks', requests.bound],
          ]),
        );
        await act(async () => {
          fixtures.auth.currentAccount = nextAccount ? { id: nextAccount } : null;
          setRequestSession(nextAccount);
          view.refresh();
        });
        await act(async () => {
          resolve(chunks);
          await sending;
        });
        expect(calls('llm_search_guide_chunks')).toHaveLength(1);
        expect(calls('llm_save_conversation')).toEqual([]);
        expect(calls('llm_send_message_stream')).toEqual([]);
        expect(view.messages()).toBe('');
        expect(useLlmStore.getState().streams).toEqual({});
      },
    );
  });
