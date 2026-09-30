import { act, renderHook, render, fireEvent, screen, cleanup } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useLlmChatCore, type UseLlmChatCoreReturn } from '@/hooks/useLlmChatCore';
import { invokeCommand } from '@/lib/ipcClient';
import { searchGuideChunks } from '@/lib/llm/guideService';
import type { ChatMsg, Conversation } from '@/types/llmChat';

const fixtures = vi.hoisted(() => ({
  auth: { currentAccount: { id: 'account' } },
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
vi.mock('@/hooks/useLlmProviderConfig', () => ({
  useLlmProviderConfig: () => ({
    activeProvider: fixtures.provider,
    isConfigured: true,
    isAiEnabled: true,
    includeSystemPrompt: true,
    loading: false,
  }),
}));
vi.mock('@/hooks/useLlmOnlineStatus', () => ({
  useLlmOnlineStatus: () => ({ isOnline: true, checkingOnline: false, checkOnline: vi.fn() }),
}));
vi.mock('@/hooks/useCopyToClipboard', () => ({
  useCopyToClipboard: () => ({ copy: vi.fn(), copiedKey: null }),
}));
vi.mock('@/lib/notification', () => ({
  markConversationPending: vi.fn(),
  setAiPageOpen: vi.fn(),
  setQuickChatOpen: vi.fn(),
}));
vi.mock('@/lib/llm/conversationPersistence', () => ({
  saveConversationSafely: vi.fn(),
  notifyConversationSaveFailed: vi.fn(),
}));
vi.mock('@/lib/llm/guideService', () => ({
  searchGuideChunks: vi.fn(),
}));

import { selectLlmStream, useLlmStore } from '@/stores/llmStore';
import {
  notifyConversationSaveFailed,
  saveConversationSafely,
} from '@/lib/llm/conversationPersistence';
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
  useLlmStore.getState().reset();
  vi.spyOn(useLlmStore.getState(), 'startStream');
  vi.mocked(saveConversationSafely).mockResolvedValue(true);
  localStorage.clear();
  fixtures.auth.currentAccount.id = 'account';
  setRequestSession('account');
  bodies = new Map();
  lists = [];
  trashLists = [];
  HTMLElement.prototype.scrollIntoView = vi.fn();
  vi.mocked(invokeCommand).mockImplementation(async (command, args) => {
    if (command === 'llm_get_conversation') {
      const id = (args as { conversationId: string }).conversationId;
      if (bodies.has(id)) return bodies.get(id)!.promise;
      const saved = vi
        .mocked(saveConversationSafely)
        .mock.calls.filter(([, conv]) => conv.id === id)
        .at(-1)?.[1];
      return saved
        ? {
            ...saved,
            messages: [...saved.messages, { role: 'assistant', content: 'reply', createdAt: '' }],
          }
        : conversation(id);
    }
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

describe('RF-005 ordinary chat provider selection', () => {
  it.each([false, true])(
    'sends only providerId with the existing conversation and context (includeSystemPrompt=%s)',
    async (includeSystemPrompt) => {
      vi.mocked(searchGuideChunks).mockResolvedValue([]);
      vi.mocked(invokeCommand).mockImplementation(async (command) => {
        if (command === 'llm_get_api_key')
          throw new Error('Ordinary chat must not read credentials');
        if (command === 'llm_get_conversation') {
          const saved = vi.mocked(saveConversationSafely).mock.calls.at(-1)?.[1];
          return saved
            ? {
                ...saved,
                messages: [...saved.messages, { role: 'assistant', content: '', createdAt: '' }],
              }
            : {
                id: 'existing-conversation',
                name: 'kept-name',
                isTemporary: false,
                messages: history.map((message) => ({ ...message })),
                updatedAt: '',
              };
        }
        return [];
      });
      const history: ChatMsg[] = [
        { id: 'user-one', role: 'user', content: '旧问题', createdAt: '' },
        { id: 'assistant-one', role: 'assistant', content: '旧回答', createdAt: '' },
      ];
      const { result } = renderHook(() => useLlmChatCore({ includeSystemPrompt }));
      await act(async () => {
        await result.current.loadConversation('existing-conversation');
      });
      act(() => result.current.setInput('  本次问题  '));

      await act(async () => {
        await result.current.sendMessage();
      });

      const sends = vi
        .mocked(invokeCommand)
        .mock.calls.filter(([command]) => command === 'llm_send_message_stream');
      expect(sends).toHaveLength(1);
      expect(sends[0][1]).toEqual({
        accountId: 'account',
        conversationId: 'existing-conversation',
        requestId: expect.any(String),
        providerId: fixtures.provider.id,
        messages: [
          { role: 'user', content: '旧问题' },
          { role: 'assistant', content: '旧回答' },
          { role: 'user', content: '本次问题' },
        ],
        contextSelection: includeSystemPrompt
          ? { mode: 'publicProfile', objectIds: [], language: 'zh-CN', guideChunks: [] }
          : { mode: 'none' },
      });
      for (const field of ['baseUrl', 'apiKey', 'model', 'apiType']) {
        expect(sends[0][1]).not.toHaveProperty(field);
      }
      expect(
        vi.mocked(invokeCommand).mock.calls.some(([command]) => command === 'llm_get_api_key'),
      ).toBe(false);
      expect(useLlmStore.getState().startStream).toHaveBeenCalledWith(
        expect.objectContaining({
          accountId: 'account',
          conversationId: 'existing-conversation',
          requestId: sends[0][1]?.requestId,
        }),
      );
      expect(result.current.messages.slice(0, history.length)).toEqual(history);
      expect(result.current.messages.map(({ role, content }) => ({ role, content }))).toEqual([
        { role: 'user', content: '旧问题' },
        { role: 'assistant', content: '旧回答' },
        { role: 'user', content: '本次问题' },
        { role: 'assistant', content: '' },
      ]);
    },
  );
});

describe('RF-104 real send ownership and canonical conversation history', () => {
  type SendRecord = {
    accountId: string;
    conversationId: string;
    requestId: string;
    messages: { role: string; content: string }[];
    pending: ReturnType<typeof deferred>;
  };

  function storedConversation(id: string, messages: ChatMsg[]): Conversation {
    return { id, name: 'stored-' + id, isTemporary: false, messages, updatedAt: '' };
  }

  function cloneConversation(value: Conversation): Conversation {
    return { ...value, messages: value.messages.map((message) => ({ ...message })) };
  }

  function installBackend(initial: Conversation[]) {
    const stored = new Map(initial.map((value) => [value.id, cloneConversation(value)]));
    const sends: SendRecord[] = [];
    vi.mocked(searchGuideChunks).mockResolvedValue([]);
    vi.mocked(saveConversationSafely).mockImplementation(async (_accountId, value) => {
      stored.set(value.id, cloneConversation(value));
      return true;
    });
    vi.mocked(invokeCommand).mockImplementation(async (command, args) => {
      if (command === 'llm_get_conversation') {
        const value = stored.get((args as { conversationId: string }).conversationId);
        if (!value) throw new Error('Conversation unavailable');
        return cloneConversation(value);
      }
      if (command === 'llm_send_message_stream') {
        const request = args as {
          accountId: string;
          conversationId: string;
          requestId: string;
          messages: { role: string; content: string }[];
        };
        const pending = deferred();
        sends.push({ ...request, pending });
        return pending.promise;
      }
      return [];
    });
    return { stored, sends };
  }

  function emit(record: SendRecord, chunk: string, isDone = false) {
    useLlmStore.getState().onChunk({
      accountId: record.accountId,
      conversationId: record.conversationId,
      requestId: record.requestId,
      sessionGeneration: 31,
      chunk,
      isDone,
    });
  }

  async function beginSend(
    core: { result: { current: UseLlmChatCoreReturn } },
    text: string,
    backend: ReturnType<typeof installBackend>,
    expectedCount: number,
  ) {
    act(() => core.result.current.setInput(text));
    let pending!: Promise<void>;
    await act(async () => {
      pending = core.result.current.sendMessage();
      await vi.waitFor(() => expect(backend.sends).toHaveLength(expectedCount));
    });
    return { pending, record: backend.sends[expectedCount - 1] };
  }

  async function completeSend(
    backend: ReturnType<typeof installBackend>,
    run: { pending: Promise<void>; record: SendRecord },
    content: string,
  ) {
    await act(async () => {
      const previous = backend.stored.get(run.record.conversationId)!;
      backend.stored.set(run.record.conversationId, {
        ...previous,
        messages: [
          ...previous.messages,
          { id: 'host-' + run.record.requestId, role: 'assistant', content, createdAt: '' },
        ],
      });
      // 后端拥有最终写入；最后事件可携带正文，随后 invoke 才结算。
      emit(run.record, content, true);
      run.record.pending.resolve(undefined);
      await run.pending;
    });
  }

  function contents(messages: ChatMsg[]) {
    return messages.map(({ role, content }) => ({ role, content }));
  }

  it('retains A completion when B reloads the same conversation and uses host history for the next A send', async () => {
    const initial: ChatMsg[] = [
      { id: 'first-user', role: 'user', content: '旧问题', createdAt: '' },
      { id: 'first-assistant', role: 'assistant', content: '旧回答', createdAt: '' },
    ];
    const backend = installBackend([storedConversation('a', initial)]);
    const a = renderHook(() => useLlmChatCore({ includeSystemPrompt: false }));
    const b = renderHook(() => useLlmChatCore({ includeSystemPrompt: false }));
    await act(async () => {
      await a.result.current.loadConversation('a');
      await b.result.current.loadConversation('a');
    });

    const first = await beginSend(a, '第一轮问题', backend, 1);
    expect(a.result.current.isSending).toBe(true);
    expect(b.result.current.isSending).toBe(true);
    act(() => b.result.current.setInput('同时发送应被拒绝'));
    await act(async () => {
      await b.result.current.sendMessage();
    });
    expect(backend.sends).toHaveLength(1);
    expect(saveConversationSafely).toHaveBeenCalledTimes(1);
    await completeSend(backend, first, '第一轮完整回答');
    expect(a.result.current.messages.at(-1)?.content).toBe('第一轮完整回答');
    expect(b.result.current.messages.at(-1)?.content).toBe('第一轮完整回答');

    await act(async () => {
      await b.result.current.loadConversation('a');
    });
    expect(a.result.current.messages.at(-1)?.content).toBe('第一轮完整回答');
    expect(b.result.current.messages.at(-1)?.content).toBe('第一轮完整回答');

    // Host 历史在当前视图快照之外变更，下一轮必须重新读取，不能用旧视图覆盖它。
    const canonical = backend.stored.get('a')!;
    backend.stored.set('a', {
      ...canonical,
      messages: [
        ...canonical.messages,
        { id: 'host-user', role: 'user', content: 'Host 已存的问题', createdAt: '' },
        { id: 'host-assistant', role: 'assistant', content: 'Host 已存的回答', createdAt: '' },
      ],
    });
    const second = await beginSend(a, '第二轮问题', backend, 2);
    const expectedHistory = [
      ...contents(initial),
      { role: 'user', content: '第一轮问题' },
      { role: 'assistant', content: '第一轮完整回答' },
      { role: 'user', content: 'Host 已存的问题' },
      { role: 'assistant', content: 'Host 已存的回答' },
      { role: 'user', content: '第二轮问题' },
    ];
    expect(second.record.messages).toEqual(expectedHistory);
    expect(contents(a.result.current.messages.slice(0, -1))).toEqual(expectedHistory);
    expect(contents(b.result.current.messages.slice(0, -1))).toEqual(expectedHistory);
    expect(saveConversationSafely).toHaveBeenCalledTimes(2);
    expect(contents(vi.mocked(saveConversationSafely).mock.calls[1][1].messages)).toEqual(
      expectedHistory,
    );

    await completeSend(backend, second, '第二轮完整回答');
    expect(contents(a.result.current.messages)).toEqual([
      ...expectedHistory,
      { role: 'assistant', content: '第二轮完整回答' },
    ]);
    expect(contents(b.result.current.messages)).toEqual(contents(a.result.current.messages));
    // 两轮各一次发送前保存；两个消费者和完成回调都不能补写最终回复。
    expect(saveConversationSafely).toHaveBeenCalledTimes(2);
    expect(
      vi
        .mocked(invokeCommand)
        .mock.calls.filter(([command]) => command === 'llm_save_conversation'),
    ).toHaveLength(0);
  });

  it('restores the draft after pre-save fails and retries without inventing a stored conversation', async () => {
    const backend = installBackend([]);
    vi.mocked(saveConversationSafely).mockResolvedValueOnce(false);
    const core = renderHook(() => useLlmChatCore({ includeSystemPrompt: false }));
    act(() => core.result.current.setInput('不能丢失历史的问题'));
    await act(async () => {
      await core.result.current.sendMessage();
    });

    expect(saveConversationSafely).toHaveBeenCalledTimes(1);
    expect(contents(vi.mocked(saveConversationSafely).mock.calls[0][1].messages)).toEqual([
      { role: 'user', content: '不能丢失历史的问题' },
    ]);
    expect(backend.sends).toHaveLength(0);
    expect(core.result.current.isSending).toBe(false);
    expect(core.result.current.messages).toEqual([]);
    expect(core.result.current.input).toBe('不能丢失历史的问题');
    expect(backend.stored.size).toBe(0);
    expect(
      vi.mocked(invokeCommand).mock.calls.filter(([command]) => command === 'llm_get_conversation'),
    ).toHaveLength(0);

    let retryPending!: Promise<void>;
    await act(async () => {
      // 用户直接重试；不手动恢复 input、messages 或 currentConvId。
      retryPending = core.result.current.sendMessage();
      await vi.waitFor(() => expect(backend.sends).toHaveLength(1));
    });
    expect(backend.sends[0].messages).toEqual([{ role: 'user', content: '不能丢失历史的问题' }]);
    await completeSend(
      backend,
      { pending: retryPending, record: backend.sends[0] },
      '重试成功回复',
    );
    expect(contents(core.result.current.messages)).toEqual([
      { role: 'user', content: '不能丢失历史的问题' },
      { role: 'assistant', content: '重试成功回复' },
    ]);
    expect(saveConversationSafely).toHaveBeenCalledTimes(2);
  });

  it('finishes the background A invoke without changing B, then displays the stored A completion', async () => {
    const aHistory: ChatMsg[] = [
      { id: 'a-history', role: 'user', content: 'A 历史', createdAt: '' },
    ];
    const bHistory: ChatMsg[] = [
      { id: 'b-history', role: 'assistant', content: 'B 已有回复', createdAt: '' },
    ];
    const backend = installBackend([
      storedConversation('a', aHistory),
      storedConversation('b', bHistory),
    ]);
    const core = renderHook(() => useLlmChatCore({ includeSystemPrompt: false }));
    await act(async () => {
      await core.result.current.loadConversation('a');
    });
    const run = await beginSend(core, 'A 的后台问题', backend, 1);
    await act(async () => {
      await core.result.current.loadConversation('b');
    });
    act(() => emit(run.record, 'A 的半段'));
    expect(core.result.current.currentConvId).toBe('b');
    expect(core.result.current.messages).toEqual(bHistory);

    await completeSend(backend, run, 'A 的尾段');
    expect(core.result.current.currentConvId).toBe('b');
    expect(core.result.current.messages).toEqual(bHistory);
    expect(saveConversationSafely).toHaveBeenCalledTimes(1);
    await act(async () => {
      await core.result.current.loadConversation('a');
    });
    expect(core.result.current.messages.at(-1)?.content).toBe('A 的尾段');
    expect(contents(core.result.current.messages)).toEqual([
      ...contents(aHistory),
      { role: 'user', content: 'A 的后台问题' },
      { role: 'assistant', content: 'A 的尾段' },
    ]);
  });

  it.each(['resolve', 'reject'] as const)(
    'ignores the old model invoke %s after locking and unlocking the same account',
    async (outcome) => {
      const backend = installBackend([storedConversation('a', [])]);
      const core = renderHook(() => useLlmChatCore({ includeSystemPrompt: false }));
      await act(async () => {
        await core.result.current.loadConversation('a');
      });
      const run = await beginSend(core, '锁定前的问题', backend, 1);
      act(() => emit(run.record, '锁定前的部分回复'));
      expect(core.result.current.messages.at(-1)?.content).toBe('锁定前的部分回复');
      const readsBeforeLock = vi
        .mocked(invokeCommand)
        .mock.calls.filter(([command]) => command === 'llm_get_conversation').length;

      act(() => {
        setRequestSession(null);
        setRequestSession('account');
      });
      expect(core.result.current.messages).toEqual([]);
      expect(core.result.current.currentConvId).toBeNull();
      await act(async () => {
        emit(run.record, '迟到的旧正文', true);
        if (outcome === 'resolve') run.record.pending.resolve(undefined);
        else run.record.pending.reject(new Error('旧会话模型失败'));
        await run.pending;
      });

      expect(core.result.current.messages).toEqual([]);
      expect(core.result.current.currentConvId).toBeNull();
      expect(core.result.current.isSending).toBe(false);
      expect(selectLlmStream(useLlmStore.getState(), 'account', 'a')).toBeUndefined();
      expect(
        vi
          .mocked(invokeCommand)
          .mock.calls.filter(([command]) => command === 'llm_get_conversation'),
      ).toHaveLength(readsBeforeLock);
      expect(saveConversationSafely).toHaveBeenCalledTimes(1);
      expect(notifyConversationSaveFailed).not.toHaveBeenCalled();
    },
  );

  it.each(['rename', 'delete'] as const)(
    'does not rename or clear B when a deferred A %s settles after changing selection',
    async (operation) => {
      const aHistory: ChatMsg[] = [
        { id: 'a-history', role: 'user', content: 'A 历史', createdAt: '' },
      ];
      const bHistory: ChatMsg[] = [
        { id: 'b-history', role: 'assistant', content: 'B 已有回复', createdAt: '' },
      ];
      installBackend([storedConversation('a', aHistory), storedConversation('b', bHistory)]);
      const command =
        operation === 'rename' ? 'llm_rename_conversation' : 'llm_soft_delete_conversation';
      const mutation = deferred();
      const baseInvoke = vi.mocked(invokeCommand).getMockImplementation()!;
      vi.mocked(invokeCommand).mockImplementation((name, args, options) =>
        name === command ? mutation.promise : baseInvoke(name, args, options),
      );
      const page = renderHook(() => useLlmChat());
      await act(async () => {
        await page.result.current.loadConversation('a');
      });
      let pending!: Promise<void>;
      await act(async () => {
        pending =
          operation === 'rename'
            ? page.result.current.handleRename('a', 'A 的新名称')
            : page.result.current.handleSoftDelete('a');
        await vi.waitFor(() =>
          expect(invokeCommand).toHaveBeenCalledWith(
            command,
            expect.objectContaining({ accountId: 'account', conversationId: 'a' }),
            expect.any(Object),
          ),
        );
      });
      await act(async () => {
        await page.result.current.loadConversation('b');
      });
      const previousB = page.result.current.currentConv;
      expect(previousB?.id).toBe('b');
      await act(async () => {
        mutation.resolve(undefined);
        await pending;
      });

      expect(page.result.current.currentConvId).toBe('b');
      expect(page.result.current.currentConv).toEqual(previousB);
      expect(page.result.current.messages).toEqual(bHistory);
      expect(page.result.current.currentConv?.name).not.toBe('A 的新名称');
      expect(saveConversationSafely).not.toHaveBeenCalled();
    },
  );

  it('keeps persisted ownership after deleted host history rejects two later quick-chat sends', async () => {
    const backend = installBackend([]);
    const quick = renderHook(() => useLlmChatCore({ includeSystemPrompt: false }));
    const first = await beginSend(quick, '首次创建的问题', backend, 1);
    await completeSend(backend, first, '首次创建的完整回答');
    const conversationId = first.record.conversationId;
    const completed = backend.stored.get(conversationId)!;
    backend.stored.set(conversationId, { ...completed, deletedAt: '2026-09-30T00:00:00Z' });
    expect(saveConversationSafely).toHaveBeenCalledTimes(1);

    for (const text of ['删除后的第一次发送', '删除后的第二次发送']) {
      act(() => quick.result.current.setInput(text));
      await act(async () => {
        await quick.result.current.sendMessage();
      });
      expect(backend.sends).toHaveLength(1);
      expect(saveConversationSafely).toHaveBeenCalledTimes(1);
      expect(selectLlmStream(useLlmStore.getState(), 'account', conversationId)?.persisted).toBe(
        true,
      );
      expect(backend.stored.get(conversationId)).toEqual({
        ...completed,
        deletedAt: '2026-09-30T00:00:00Z',
      });
    }
    // 一次完成确认、两次规范历史读取；读取被拒绝不能让下一轮绕过已有会话检查。
    expect(
      vi.mocked(invokeCommand).mock.calls.filter(([command]) => command === 'llm_get_conversation'),
    ).toHaveLength(3);
  });
});
