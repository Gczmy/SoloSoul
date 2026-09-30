import { act, cleanup, renderHook } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type Event } from '@tauri-apps/api/event';
import type { TFunction } from 'i18next';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useLlmStreaming } from '@/hooks/useLlmStreaming';
import { notifyConversationSaveFailed } from '@/lib/llm/conversationPersistence';
import { setRequestSession } from '@/lib/sessionRequests';
import {
  selectLlmStream,
  useLlmStore,
  type LlmStreamPayload,
  type StreamIdentity,
  type StreamRun,
} from '@/stores/llmStore';
import type { ChatMsg } from '@/types/llmChat';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));
vi.mock('@/lib/llm/conversationPersistence', () => ({
  notifyConversationSaveFailed: vi.fn(),
}));

const accountId = 'account';
const t = ((key: string) => key) as TFunction;
let callbacks: ((event: Event<LlmStreamPayload>) => void)[];
let unlisten: ReturnType<typeof vi.fn<() => void>>;

function messagesFor(conversationId: string): ChatMsg[] {
  return [
    {
      id: `${conversationId}-user`,
      role: 'user',
      content: `问题-${conversationId}`,
      createdAt: '',
    },
    { id: `${conversationId}-assistant`, role: 'assistant', content: '', createdAt: '' },
  ];
}

function freezeMessages(messages: ChatMsg[]): void {
  for (const message of messages) Object.freeze(message);
  Object.freeze(messages);
}

function identity(conversationId: string, requestId = `${conversationId}-request`): StreamIdentity {
  return { accountId, conversationId, requestId };
}

async function start(
  streamIdentity: StreamIdentity,
  messages: ChatMsg[],
  assistantMessageId = `${streamIdentity.conversationId}-assistant`,
): Promise<StreamRun> {
  let run: StreamRun | null = null;
  await act(async () => {
    run = useLlmStore.getState().startStream({
      ...streamIdentity,
      assistantMessageId,
      messages,
    });
    if (!run) throw new Error('Expected a current stream request');
    await run.ready;
  });
  return run!;
}

function emit(
  streamIdentity: StreamIdentity,
  chunk: string,
  options: Partial<Pick<LlmStreamPayload, 'isDone' | 'error' | 'sessionGeneration'>> = {},
  callback = callbacks[callbacks.length - 1],
): void {
  callback({
    event: 'llm-stream-chunk',
    id: 1,
    payload: { ...streamIdentity, sessionGeneration: 7, chunk, isDone: false, ...options },
  });
}

beforeEach(() => {
  setRequestSession(null);
  useLlmStore.getState().reset();
  vi.clearAllMocks();
  callbacks = [];
  unlisten = vi.fn();
  vi.mocked(listen).mockImplementation(async (_event, callback) => {
    callbacks.push(callback as (event: Event<LlmStreamPayload>) => void);
    return unlisten;
  });
  setRequestSession(accountId);
});

afterEach(() => {
  cleanup();
  useLlmStore.getState().reset();
  setRequestSession(null);
  // 投影和完成提示不能重新保存会话；最终 assistant 内容由后端持久化。
  expect(
    vi.mocked(invoke).mock.calls.filter(([command]) => command === 'llm_save_conversation'),
  ).toHaveLength(0);
});

describe('RF-104 stream projection with real session ownership', () => {
  it('keeps A streaming while switching to B without changing B or either input history', async () => {
    const a = messagesFor('a');
    const b = messagesFor('b');
    const originalA = a.map((message) => ({ ...message }));
    const originalB = b.map((message) => ({ ...message }));
    freezeMessages(a);
    freezeMessages(b);
    const streamIdentity = identity('a');
    const { result, rerender } = renderHook(
      ({ messages, currentConvId }) => useLlmStreaming({ messages, accountId, currentConvId, t }),
      { initialProps: { messages: a, currentConvId: 'a' } },
    );
    const run = await start(streamIdentity, a);

    act(() => emit(streamIdentity, 'A 的前半段'));
    expect(result.current[1].content).toBe('A 的前半段');

    rerender({ messages: b, currentConvId: 'b' });
    act(() => emit(streamIdentity, '与后半段'));
    expect(result.current).toBe(b);
    expect(run.isCurrent()).toBe(true);

    rerender({ messages: a, currentConvId: 'a' });
    expect(result.current[1].content).toBe('A 的前半段与后半段');
    expect(a).toEqual(originalA);
    expect(b).toEqual(originalB);
  });

  it('updates page and quick chat together and claims a persistence failure notification only once', async () => {
    const messages = messagesFor('a');
    const streamIdentity = identity('a');
    const onPageSaved = vi.fn();
    const onQuickSaved = vi.fn();
    const page = renderHook(() =>
      useLlmStreaming({
        messages,
        accountId,
        currentConvId: 'a',
        onConversationSaved: onPageSaved,
        t,
      }),
    );
    const quick = renderHook(() =>
      useLlmStreaming({
        messages,
        accountId,
        currentConvId: 'a',
        onConversationSaved: onQuickSaved,
        t,
      }),
    );
    await start(streamIdentity, messages);

    act(() => emit(streamIdentity, '共享的回复'));
    expect(page.result.current[1].content).toBe('共享的回复');
    expect(quick.result.current[1].content).toBe('共享的回复');
    act(() => {
      emit(streamIdentity, '', { isDone: true, error: '__LLM_PERSIST_FAILED__: db full' });
      useLlmStore.getState().finishStream(streamIdentity);
    });
    page.rerender();
    quick.rerender();

    expect(notifyConversationSaveFailed).toHaveBeenCalledTimes(1);
    expect(notifyConversationSaveFailed).toHaveBeenCalledWith(t);
    expect(page.result.current[1].content).toBe('共享的回复');
    expect(quick.result.current[1].content).toBe('共享的回复');
    expect(onPageSaved).not.toHaveBeenCalled();
    expect(onQuickSaved).not.toHaveBeenCalled();
    expect(messages[1].content).toBe('');
  });

  it('retains done tail text through the later save failure and invoke settlement', async () => {
    const messages = messagesFor('a');
    const streamIdentity = identity('a');
    const onConversationSaved = vi.fn();
    const { result } = renderHook(() =>
      useLlmStreaming({
        messages,
        accountId,
        currentConvId: 'a',
        onConversationSaved,
        t,
      }),
    );
    await start(streamIdentity, messages);

    act(() => {
      emit(streamIdentity, '完整回复');
      emit(streamIdentity, '的尾段', { isDone: true });
    });
    expect(result.current[1].content).toBe('完整回复的尾段');
    expect(unlisten).not.toHaveBeenCalled();

    act(() =>
      emit(streamIdentity, '', { isDone: true, error: '__LLM_PERSIST_FAILED__: write failed' }),
    );
    act(() =>
      useLlmStore.getState().finishStream(streamIdentity, '__LLM_PERSIST_FAILED__: write failed'),
    );

    expect(result.current[1]).toEqual(
      expect.objectContaining({ id: 'a-assistant', content: '完整回复的尾段' }),
    );
    expect(result.current[1].isError).not.toBe(true);
    expect(selectLlmStream(useLlmStore.getState(), accountId, 'a')).toEqual(
      expect.objectContaining({ settled: true, persistFailed: true, error: null }),
    );
    expect(notifyConversationSaveFailed).toHaveBeenCalledTimes(1);
    expect(onConversationSaved).not.toHaveBeenCalled();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('updates the named assistant even when later user and assistant messages follow it', async () => {
    const messages = [
      ...messagesFor('a'),
      { id: 'later-user', role: 'user', content: '稍后的问题', createdAt: '' },
      { id: 'later-assistant', role: 'assistant', content: '已有的稍后回复', createdAt: '' },
    ];
    const original = messages.map((message) => ({ ...message }));
    freezeMessages(messages);
    const streamIdentity = identity('a');
    const onConversationSaved = vi.fn();
    const { result, rerender } = renderHook(() =>
      useLlmStreaming({
        messages,
        accountId,
        currentConvId: 'a',
        onConversationSaved,
        t,
      }),
    );
    await start(streamIdentity, messages, 'a-assistant');
    act(() => {
      emit(streamIdentity, '目标回复', { isDone: true });
      useLlmStore.getState().finishStream(streamIdentity);
    });
    rerender();

    expect(result.current.map(({ id, content }) => ({ id, content }))).toEqual([
      { id: 'a-user', content: '问题-a' },
      { id: 'a-assistant', content: '目标回复' },
      { id: 'later-user', content: '稍后的问题' },
      { id: 'later-assistant', content: '已有的稍后回复' },
    ]);
    expect(messages).toEqual(original);
    expect(onConversationSaved).toHaveBeenCalledTimes(1);
  });

  it('does not redisplay an expired reply after lock and same-account unlock', async () => {
    const messages = messagesFor('a');
    const oldIdentity = identity('a', 'old-request');
    const { result } = renderHook(() =>
      useLlmStreaming({ messages, accountId, currentConvId: 'a', t }),
    );
    const oldRun = await start(oldIdentity, messages);
    const oldCallback = callbacks[0];
    act(() => emit(oldIdentity, '旧会话密文解密后的回复'));
    expect(result.current[1].content).toBe('旧会话密文解密后的回复');

    act(() => {
      setRequestSession(null);
      setRequestSession(accountId);
    });
    expect(result.current).toBe(messages);
    expect(result.current[1].content).toBe('');
    expect(oldRun.isCurrent()).toBe(false);
    act(() =>
      emit(
        oldIdentity,
        '迟到旧正文',
        { isDone: true, error: '__LLM_PERSIST_FAILED__: old session' },
        oldCallback,
      ),
    );
    expect(result.current[1].content).toBe('');

    const currentIdentity = identity('a', 'current-request');
    await start(currentIdentity, messages);
    act(() => {
      emit(oldIdentity, '从新监听器送来的旧请求');
      emit(currentIdentity, '当前会话回复', { sessionGeneration: 8 });
      useLlmStore.getState().finishStream(oldIdentity);
    });
    expect(result.current[1].content).toBe('当前会话回复');
    expect(selectLlmStream(useLlmStore.getState(), accountId, 'a')?.settled).toBe(false);
    expect(notifyConversationSaveFailed).not.toHaveBeenCalled();
  });
});
