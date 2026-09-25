import { act, renderHook, render, fireEvent, screen, cleanup } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useLlmChatCore } from '@/hooks/useLlmChatCore';
import { invokeCommand } from '@/lib/ipcClient';

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
vi.mock('@/lib/notification', () => ({
  markConversationPending: vi.fn(),
  setAiPageOpen: vi.fn(),
  setQuickChatOpen: vi.fn(),
}));
vi.mock('@/lib/llm/conversationPersistence', () => ({ saveConversationSafely: vi.fn() }));
vi.mock('@/lib/llm/guideService', () => ({
  searchGuideChunks: vi.fn(),
  formatChunksAsSystemMessage: vi.fn(),
}));
vi.mock('@/lib/llm/systemPromptBuilder', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/llm/systemPromptBuilder')>()),
  buildSystemPrompt: () => 'SYSTEM',
}));

import { useLlmChat } from '@/pages/ai/LlmChatPage/useLlmChat';
import { AiQuickChatPopover } from '@/components/layout/AiQuickChatPopover';
import { setRequestSession } from '@/lib/sessionRequests';
import { logger } from '@/lib/logger';
import { ST_QUICK_CHAT_PREFIX } from '@/lib/constants';
vi.mock('@/lib/logger', () => ({ logger: { warn: vi.fn() } }));
vi.mock('@/hooks/useToastError', () => ({ useToastError: () => ({ onError: vi.fn() }) }));
vi.mock('react-router-dom', () => ({ useNavigate: () => vi.fn() }));
vi.mock('@/components/llm/ChatMessageList', () => ({
  ChatMessageList: ({ messages }: { messages: { content: string }[] }) => (
    <div>{messages.map((m) => m.content).join('|')}</div>
  ),
}));
vi.mock('@/components/llm/ChatInputBar', () => ({ ChatInputBar: () => null }));
vi.mock('@/components/llm/ConversationHistory', () => ({ ConversationHistory: () => null }));
vi.mock('@/components/llm/UnconfiguredHint', () => ({ UnconfiguredHint: () => null }));

function deferred() {
  let resolve!: (value: unknown) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
function conversation(id: string) {
  return {
    id,
    name: id,
    isTemporary: false,
    messages: [{ role: 'user', content: `body-${id}`, createdAt: '' }],
    updatedAt: '',
  };
}
let bodies: Map<string, ReturnType<typeof deferred>>;
let lists: ReturnType<typeof deferred>[];
let trashLists: ReturnType<typeof deferred>[];
function pendingBody(id: string) {
  const pending = deferred();
  bodies.set(id, pending);
  return pending;
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  fixtures.auth.currentAccount.id = 'account';
  setRequestSession('account');
  bodies = new Map();
  lists = [];
  trashLists = [];
  HTMLElement.prototype.scrollIntoView = vi.fn();
  vi.mocked(invokeCommand).mockImplementation(async (command, args) => {
    if (command === 'llm_get_conversation')
      return bodies.get((args as { conversationId: string }).conversationId)!.promise;
    if (command === 'llm_list_conversations') {
      const p = deferred();
      lists.push(p);
      return p.promise;
    }
    if (command === 'llm_list_trash') {
      const p = deferred();
      trashLists.push(p);
      return p.promise;
    }
    return undefined;
  });
});
afterEach(() => {
  cleanup();
  setRequestSession(null);
});

describe('RF-103 conversation reads', () => {
  it('only commits B after choosing A then B, preserving generated message ids', async () => {
    const a = pendingBody('a'),
      b = pendingBody('b');
    const { result } = renderHook(() => useLlmChatCore());
    act(() => {
      void result.current.loadConversation('a');
      void result.current.loadConversation('b');
    });
    await act(async () => {
      b.resolve(conversation('b'));
    });
    await act(async () => {
      a.resolve(conversation('a'));
    });
    expect(result.current.currentConvId).toBe('b');
    expect(result.current.messages).toEqual([
      expect.objectContaining({ id: expect.any(String), content: 'body-b' }),
    ]);
  });
  it('list latest request and body request are independent', async () => {
    const a = pendingBody('a');
    const { result } = renderHook(() => useLlmChatCore());
    act(() => {
      void result.current.loadConversation('a');
      void result.current.loadConversationList();
    });
    await act(async () => {
      lists[1].resolve([{ id: 'new-list' }]);
      a.resolve(conversation('a'));
    });
    await act(async () => {
      lists[0].resolve([{ id: 'old-list' }]);
    });
    expect(result.current.conversations).toEqual([{ id: 'new-list' }]);
    expect(result.current.currentConvId).toBe('a');
  });
  it.each(['new', 'close', 'unmount'] as const)('%s invalidates pending reads', async (action) => {
    const a = pendingBody('a');
    const { result, unmount } = renderHook(() => useLlmChatCore());
    act(() => {
      void result.current.loadConversation('a');
    });
    if (action === 'unmount') unmount();
    else
      act(() => {
        if (action === 'new') result.current.setCurrentConvId('new');
        else result.current.invalidateReads();
      });
    await act(async () => {
      a.reject(new Error('late failure'));
    });
    expect(logger.warn).not.toHaveBeenCalled();
    if (action !== 'unmount')
      expect(result.current.currentConvId).toBe(action === 'new' ? 'new' : null);
  });
  it('lock and same-account unlock reject old body/list responses and clear visible data', async () => {
    const old = pendingBody('old'),
      current = pendingBody('current');
    const { result } = renderHook(() => useLlmChatCore());
    act(() => {
      result.current.setMessages(conversation('secret').messages as never);
      void result.current.loadConversation('old');
    });
    act(() => {
      setRequestSession(null);
      setRequestSession('account');
    });
    expect(result.current.messages).toEqual([]);
    act(() => {
      void result.current.loadConversation('current');
    });
    await act(async () => {
      old.resolve(conversation('old'));
      lists[0].resolve([{ id: 'secret' }]);
    });
    expect(result.current.messages).toEqual([]);
    expect(result.current.conversations).toEqual([]);
    await act(async () => {
      current.resolve(conversation('current'));
    });
    expect(result.current.currentConvId).toBe('current');
  });
  it('account change does not accept the previous account error', async () => {
    const old = pendingBody('old');
    const { result, rerender } = renderHook(() => useLlmChatCore());
    act(() => {
      void result.current.loadConversation('old');
    });
    fixtures.auth.currentAccount.id = 'second';
    act(() => setRequestSession('second'));
    rerender();
    await act(async () => {
      old.reject(new Error('old account'));
    });
    expect(result.current.messages).toEqual([]);
    expect(logger.warn).not.toHaveBeenCalled();
  });
  it('page new conversation invalidates a pending body load', async () => {
    const a = pendingBody('a');
    const { result } = renderHook(() => useLlmChat());
    act(() => {
      void result.current.loadConversation('a');
    });
    act(() => result.current.handleNewConversation());
    const newId = result.current.currentConvId;
    await act(async () => {
      a.resolve(conversation('a'));
    });
    expect(result.current.currentConvId).toBe(newId);
    expect(result.current.currentConv?.isTemporary).toBe(true);
    expect(result.current.messages).toEqual([]);
  });
  it('trash body uses latest selection and closing the preview prevents late reopening', async () => {
    const a = pendingBody('a'),
      b = pendingBody('b');
    const { result } = renderHook(() => useLlmChat());
    act(() => {
      result.current.setShowTrash(true);
    });
    act(() => {
      void result.current.handleViewTrashConv('a');
      void result.current.handleViewTrashConv('b');
    });
    await act(async () => {
      b.resolve(conversation('b'));
    });
    await act(async () => {
      a.resolve(conversation('a'));
    });
    expect(result.current.floatingConv?.id).toBe('b');
    const c = pendingBody('c');
    act(() => {
      void result.current.handleViewTrashConv('c');
    });
    act(() => result.current.setFloatingConv(null));
    await act(async () => {
      c.resolve(conversation('c'));
    });
    expect(result.current.floatingConv).toBeNull();
  });
  it('trash list ignores prior refresh and clears on lock', async () => {
    const { result } = renderHook(() => useLlmChat());
    act(() => result.current.setShowTrash(true));
    act(() => result.current.setShowTrash(false));
    act(() => result.current.setShowTrash(true));
    await act(async () => {
      trashLists[1].resolve([{ id: 'new' }]);
    });
    await act(async () => {
      trashLists[0].resolve([{ id: 'old' }]);
    });
    expect(result.current.trashList).toEqual([{ id: 'new' }]);
    act(() => setRequestSession(null));
    expect(result.current.trashList).toEqual([]);
  });
  it.each(['new', 'close'] as const)(
    'quick chat %s prevents a restored response and stale storage write',
    async (action) => {
      const old = pendingBody('old');
      const key = `${ST_QUICK_CHAT_PREFIX}account`;
      localStorage.setItem(key, 'old');
      const onClose = vi.fn();
      render(<AiQuickChatPopover onClose={onClose} />);
      fireEvent.click(
        screen.getByTitle(action === 'new' ? 'settings:ai_new_conv' : 'common:close'),
      );
      await act(async () => {
        old.resolve(conversation('old'));
      });
      expect(screen.queryByText('body-old')).toBeNull();
      if (action === 'new') expect(localStorage.getItem(key)).toBeNull();
      else expect(onClose).toHaveBeenCalledTimes(1);
    },
  );
});
