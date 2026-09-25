import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useLlmChatCore } from '@/hooks/useLlmChatCore';
import { invokeCommand } from '@/lib/ipcClient';
import type { ChatMsg } from '@/types/llmChat';
import { buildChatRequestMessages } from './chatRequest';
import { searchGuideChunks, formatChunksAsSystemMessage } from './guideService';
import { saveConversationSafely } from './conversationPersistence';

const fixtures = vi.hoisted(() => ({
  auth: { currentAccount: { id: 'account' } },
  stream: { streamBuffer: '', startStream: vi.fn(), onChunk: vi.fn(), reset: vi.fn() },
  provider: { id: 'provider', baseUrl: 'https://example.test', model: 'model', apiType: 'openai' },
  t: (key: string) => key,
}));
vi.mock('@/lib/i18n', () => ({ default: { language: 'zh-CN' } }));
vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: fixtures.t }) }));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: vi.fn() }));
vi.mock('@/stores/settingsStore', () => ({
  useSettingsStore: { getState: () => ({ settings: {} }) },
}));
vi.mock('@/stores/objectStore', () => ({ useObjectStore: { getState: () => ({ objects: [] }) } }));
vi.mock('@/stores/templateStore', () => ({
  useTemplateStore: { getState: () => ({ templates: [] }) },
}));
vi.mock('@/stores/authStore', () => ({
  useAuthStore: (select: (state: typeof fixtures.auth) => unknown) => select(fixtures.auth),
}));
vi.mock('@/stores/llmStore', () => ({
  useLlmStore: (select: (state: typeof fixtures.stream) => unknown) => select(fixtures.stream),
}));
vi.mock('@/hooks/useLlmProviderConfig', () => ({
  useLlmProviderConfig: () => ({
    activeProvider: fixtures.provider,
    isConfigured: true,
    isAiEnabled: true,
    loading: false,
  }),
}));
vi.mock('@/hooks/useLlmOnlineStatus', () => ({
  useLlmOnlineStatus: () => ({ isOnline: true, checkingOnline: false, checkOnline: vi.fn() }),
}));
vi.mock('@/hooks/useLlmStreaming', () => ({ useLlmStreaming: vi.fn() }));
vi.mock('@/hooks/useCopyToClipboard', () => ({
  useCopyToClipboard: () => ({ copy: vi.fn(), copiedKey: null }),
}));
vi.mock('@/lib/notification', () => ({ markConversationPending: vi.fn() }));
vi.mock('./conversationPersistence', () => ({ saveConversationSafely: vi.fn() }));
vi.mock('./guideService', () => ({
  searchGuideChunks: vi.fn(),
  formatChunksAsSystemMessage: vi.fn(),
}));
vi.mock('./systemPromptBuilder', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./systemPromptBuilder')>()),
  buildSystemPrompt: () => 'SYSTEM',
}));

const previous: ChatMsg[] = [
  { id: 'one', role: 'user', content: '旧问题', createdAt: '' },
  { id: 'two', role: 'assistant', content: '旧回答', createdAt: '' },
];

describe('chat request message ownership', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(searchGuideChunks).mockResolvedValue([]);
    vi.mocked(formatChunksAsSystemMessage).mockReturnValue('GUIDE');
    vi.mocked(saveConversationSafely).mockResolvedValue(true);
    vi.mocked(invokeCommand).mockImplementation(async (command) =>
      command === 'llm_get_api_key' ? 'test-key' : [],
    );
  });

  for (const includeSystemPrompt of [false, true]) {
    for (const history of [[], previous]) {
      const scenario = `system=${includeSystemPrompt}, history=${history.length}`;
      const expected = [
        ...(includeSystemPrompt ? [{ role: 'system', content: 'SYSTEM\n\nGUIDE' }] : []),
        ...history.map(({ role, content }) => ({ role, content })),
        { role: 'user', content: '本次问题' },
      ];

      it(`builds the final sequence once (${scenario})`, async () => {
        const snapshot = structuredClone(history);
        const messages = await buildChatRequestMessages({
          text: '本次问题',
          history,
          includeSystemPrompt,
        });
        expect(messages.map(({ role, content }) => ({ role, content }))).toEqual(expected);
        expect(history).toEqual(snapshot);
        expect(searchGuideChunks).toHaveBeenCalledTimes(includeSystemPrompt ? 1 : 0);
        if (includeSystemPrompt)
          expect(searchGuideChunks).toHaveBeenCalledWith('本次问题', 'zh-CN');
      });

      it(`sends the real hook history without duplicating UI input (${scenario})`, async () => {
        const { result } = renderHook(() => useLlmChatCore({ includeSystemPrompt }));
        await act(async () => {
          result.current.setMessages(history);
          result.current.setInput('  本次问题  ');
        });
        await act(async () => {
          await result.current.sendMessage();
        });
        const sends = vi
          .mocked(invokeCommand)
          .mock.calls.filter(([command]) => command === 'llm_send_message_stream');
        expect(sends).toHaveLength(1);
        const payload = sends[0][1] as { messages: Array<{ role: string; content: string }> };
        expect(payload.messages.map(({ role, content }) => ({ role, content }))).toEqual(expected);
        expect(result.current.messages.map(({ role, content }) => ({ role, content }))).toEqual([
          ...expected.filter(({ role }) => role !== 'system'),
          { role: 'assistant', content: '' },
        ]);
        if (!history.length) {
          expect(saveConversationSafely).toHaveBeenCalledWith(
            'account',
            expect.objectContaining({
              messages: [expect.objectContaining({ role: 'user', content: '本次问题' })],
            }),
            fixtures.t,
          );
        }
      });
    }
  }
});
