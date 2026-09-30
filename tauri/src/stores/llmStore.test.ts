import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invokeCommand } from '@/lib/ipcClient';
import { setRequestSession } from '@/lib/sessionRequests';
import type { ChatMsg } from '@/types/llmChat';
import {
  invokeConversationChange,
  isConversationBusy,
  selectLlmStream,
  useLlmStore,
  type LlmStreamPayload,
  type StreamIdentity,
} from './llmStore';

type UnlistenFn = () => void;
type ChunkHandler = (event: { payload: LlmStreamPayload }) => void;
const { mockListen } = vi.hoisted(() => ({ mockListen: vi.fn() }));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: vi.fn() }));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

const accountId = 'account-a';
const identity = (conversationId = 'conversation-a', requestId = 'request-a'): StreamIdentity => ({
  accountId,
  conversationId,
  requestId,
});
const chunk = (
  owner: StreamIdentity,
  overrides: Partial<LlmStreamPayload> = {},
): LlmStreamPayload => ({
  ...owner,
  sessionGeneration: 730,
  chunk: '',
  isDone: false,
  ...overrides,
});
const messages = (): ChatMsg[] => [
  { id: 'user-a', role: 'user', content: 'Question', createdAt: '2026-09-30T00:00:00Z' },
  { id: 'assistant-a', role: 'assistant', content: '', createdAt: '2026-09-30T00:00:01Z' },
];

function start(owner = identity()) {
  const run = useLlmStore.getState().startStream({
    ...owner,
    assistantMessageId: `assistant-${owner.requestId}`,
    messages: messages(),
  });
  expect(run).not.toBeNull();
  if (!run) throw new Error('Expected the conversation reservation to succeed');
  return run;
}

function snapshot(owner = identity()) {
  const stream = selectLlmStream(useLlmStore.getState(), owner.accountId, owner.conversationId);
  expect(stream).toBeDefined();
  if (!stream) throw new Error('Expected the conversation snapshot to exist');
  return stream;
}

describe('llmStore request ownership', () => {
  beforeEach(() => {
    useLlmStore.getState().reset();
    setRequestSession(null);
    mockListen.mockReset();
    mockListen.mockImplementation(async () => vi.fn());
    vi.mocked(invokeCommand).mockReset();
    vi.mocked(invokeCommand).mockResolvedValue(undefined);
    setRequestSession(accountId);
  });

  afterEach(() => {
    useLlmStore.getState().reset();
    setRequestSession(null);
    vi.restoreAllMocks();
  });

  it('reserves the conversation synchronously before the event subscription resolves', async () => {
    const subscription = deferred<UnlistenFn>();
    const unlisten = vi.fn();
    mockListen.mockReturnValue(subscription.promise);
    const owner = identity();
    const run = start(owner);

    expect(isConversationBusy(useLlmStore.getState(), accountId, owner.conversationId)).toBe(true);
    expect(snapshot(owner)).toMatchObject({
      ...owner,
      buffer: '',
      error: null,
      persistFailed: false,
      settled: false,
      backendGeneration: null,
    });
    expect(run.isCurrent()).toBe(true);
    await Promise.resolve();
    expect(mockListen).toHaveBeenCalledWith('llm-stream-chunk', expect.any(Function));
    subscription.resolve(unlisten);
    await run.ready;
    expect(unlisten).not.toHaveBeenCalled();
  });

  it('copies message snapshots so a caller cannot replace the reserved conversation content', async () => {
    const source = messages();
    const owner = identity();
    const run = useLlmStore.getState().startStream({
      ...owner,
      assistantMessageId: 'assistant-a',
      messages: source,
    });
    expect(run).not.toBeNull();
    await run!.ready;
    source[0].content = 'Changed externally';
    source.push({ role: 'user', content: 'Other', createdAt: '2026-09-30T00:00:02Z' });
    expect(snapshot(owner).messages).toEqual(messages());
  });

  it('rejects two synchronous sends to one conversation without replacing the first request', async () => {
    const first = start();
    const secondOwner = identity('conversation-a', 'request-b');
    const second = useLlmStore.getState().startStream({
      ...secondOwner,
      assistantMessageId: 'assistant-b',
      messages: messages(),
    });
    expect(second).toBeNull();
    expect(snapshot().requestId).toBe('request-a');
    await first.ready;
    expect(mockListen).toHaveBeenCalledTimes(1);
  });

  it('shares one pending listener across two independent conversations and retains it until both settle', async () => {
    const subscription = deferred<UnlistenFn>();
    const unlisten = vi.fn();
    mockListen.mockReturnValue(subscription.promise);
    const firstOwner = identity();
    const secondOwner = identity('conversation-b', 'request-b');
    const first = start(firstOwner);
    const second = start(secondOwner);
    await Promise.resolve();
    expect(mockListen).toHaveBeenCalledTimes(1);
    subscription.resolve(unlisten);
    await Promise.all([first.ready, second.ready]);

    const handler = mockListen.mock.calls[0][1] as ChunkHandler;
    handler({ payload: chunk(firstOwner, { chunk: 'A' }) });
    handler({ payload: chunk(secondOwner, { chunk: 'B', sessionGeneration: 900 }) });
    expect(snapshot(firstOwner).buffer).toBe('A');
    expect(snapshot(secondOwner).buffer).toBe('B');
    useLlmStore.getState().finishStream(firstOwner);
    expect(snapshot(firstOwner).settled).toBe(true);
    expect(snapshot(secondOwner).settled).toBe(false);
    expect(unlisten).not.toHaveBeenCalled();
    useLlmStore.getState().finishStream(secondOwner);
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('lets the invoking caller settle an event subscription failure without leaving a busy conversation', async () => {
    const failure = new Error('Tauri event error');
    mockListen.mockRejectedValue(failure);
    const owner = identity();
    const run = start(owner);
    await run.ready.catch((error: unknown) => {
      useLlmStore.getState().finishStream(owner, String(error));
    });
    expect(snapshot(owner)).toMatchObject({
      settled: true,
      error: 'Error: Tauri event error',
      buffer: '',
    });
    expect(isConversationBusy(useLlmStore.getState(), accountId, owner.conversationId)).toBe(false);
    expect(invokeCommand).not.toHaveBeenCalled();
  });

  it('appends ordinary chunks and the final done chunk but waits for invoke settlement to unsubscribe', async () => {
    const unlisten = vi.fn();
    mockListen.mockResolvedValue(unlisten);
    const owner = identity();
    await start(owner).ready;
    useLlmStore.getState().onChunk(chunk(owner, { chunk: 'Hello' }));
    useLlmStore.getState().onChunk(chunk(owner, { chunk: ' World', isDone: true }));
    expect(snapshot(owner)).toMatchObject({ buffer: 'Hello World', settled: false });
    expect(unlisten).not.toHaveBeenCalled();
    useLlmStore.getState().finishStream(owner);
    expect(snapshot(owner)).toMatchObject({ buffer: 'Hello World', settled: true });
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('preserves a done event that carries all of the response body', async () => {
    const owner = identity();
    await start(owner).ready;
    useLlmStore.getState().onChunk(chunk(owner, { chunk: 'Complete response', isDone: true }));
    expect(snapshot(owner).buffer).toBe('Complete response');
    useLlmStore.getState().finishStream(owner);
    expect(snapshot(owner).buffer).toBe('Complete response');
  });

  it('preserves generated text when persist failure arrives after done and before invoke settlement', async () => {
    const unlisten = vi.fn();
    mockListen.mockResolvedValue(unlisten);
    const owner = identity();
    await start(owner).ready;
    useLlmStore.getState().onChunk(chunk(owner, { chunk: '完整回复', isDone: true }));
    useLlmStore
      .getState()
      .onChunk(chunk(owner, { isDone: true, error: '__LLM_PERSIST_FAILED__: db full' }));
    expect(snapshot(owner)).toMatchObject({
      buffer: '完整回复',
      persistFailed: true,
      error: null,
      settled: false,
    });
    expect(unlisten).not.toHaveBeenCalled();
    useLlmStore.getState().finishStream(owner, 'save failed');
    expect(snapshot(owner)).toMatchObject({
      buffer: '完整回复',
      persistFailed: true,
      error: null,
      settled: true,
    });
    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(useLlmStore.getState().claimPersistFailure(owner)).toBe(true);
    expect(useLlmStore.getState().claimPersistFailure(owner)).toBe(false);
  });

  it('keeps a real stream error distinct from persist failure and preserves partial text', async () => {
    const owner = identity();
    await start(owner).ready;
    useLlmStore.getState().onChunk(chunk(owner, { chunk: 'Partial' }));
    useLlmStore.getState().onChunk(chunk(owner, { error: 'HTTP 500: upstream error' }));
    expect(snapshot(owner)).toMatchObject({
      buffer: 'Partial',
      error: 'HTTP 500: upstream error',
      persistFailed: false,
      settled: false,
    });
    useLlmStore.getState().finishStream(owner);
    expect(snapshot(owner).settled).toBe(true);
  });

  it('uses canonical invoke text to recover omitted chunks without duplicating the received suffix', async () => {
    const owner = identity();
    await start(owner).ready;
    useLlmStore.getState().onChunk(chunk(owner, { chunk: 'He' }));
    useLlmStore.getState().onChunk(chunk(owner, { chunk: 'lo', isDone: true }));
    useLlmStore.getState().finishStream(owner, undefined, 'Hello World');
    expect(snapshot(owner)).toMatchObject({ buffer: 'Hello World', settled: true, error: null });
  });

  it.each([
    { accountId: 'account-b' },
    { conversationId: 'conversation-b' },
    { requestId: 'request-b' },
  ])('ignores an event with a mismatched identity: %o', async (foreign) => {
    const owner = identity();
    await start(owner).ready;
    useLlmStore.getState().onChunk(chunk(owner, { chunk: 'Keep' }));
    useLlmStore
      .getState()
      .onChunk(chunk(owner, { ...foreign, chunk: 'Ignore', isDone: true, error: 'Foreign error' }));
    expect(snapshot(owner)).toMatchObject({
      buffer: 'Keep',
      error: null,
      persistFailed: false,
      settled: false,
    });
  });

  it.each([
    { sessionGeneration: -1 },
    { sessionGeneration: Number.NaN },
    { sessionGeneration: Number.MAX_SAFE_INTEGER + 1 },
    { sessionGeneration: '730' },
    { error: 42 },
    { isDone: 'true' },
    { chunk: null },
    { accountId: null },
  ])('rejects malformed events before changing the stream: %o', async (invalid) => {
    const owner = identity();
    await start(owner).ready;
    useLlmStore.getState().onChunk({
      ...chunk(owner, { chunk: 'Invalid' }),
      ...invalid,
    } as unknown as LlmStreamPayload);
    expect(snapshot(owner)).toMatchObject({
      buffer: '',
      error: null,
      backendGeneration: null,
      settled: false,
    });
  });

  it('binds the first backend generation independently of local session increments', async () => {
    setRequestSession(null);
    setRequestSession(accountId);
    const owner = identity();
    await start(owner).ready;
    useLlmStore.getState().onChunk(chunk(owner, { sessionGeneration: 4242, chunk: 'Current' }));
    useLlmStore
      .getState()
      .onChunk(
        chunk(owner, { sessionGeneration: 4241, chunk: 'Old', isDone: true, error: 'Old error' }),
      );
    expect(snapshot(owner)).toMatchObject({
      backendGeneration: 4242,
      buffer: 'Current',
      error: null,
      settled: false,
    });
  });

  it('ignores chunks and duplicate settlement after the request has settled', async () => {
    const owner = identity();
    await start(owner).ready;
    useLlmStore.getState().finishStream(owner, undefined, 'Canonical');
    useLlmStore.getState().onChunk(chunk(owner, { chunk: 'Late', error: 'Late error' }));
    useLlmStore.getState().finishStream(owner, 'Duplicate error', 'Duplicate text');
    expect(snapshot(owner)).toMatchObject({ buffer: 'Canonical', error: null, settled: true });
  });

  it('clears streams, invalidates request tickets, and releases the listener on session lock', async () => {
    const unlisten = vi.fn();
    mockListen.mockResolvedValue(unlisten);
    const owner = identity();
    const run = start(owner);
    await run.ready;
    useLlmStore.getState().onChunk(chunk(owner, { chunk: 'Sensitive' }));
    setRequestSession(null);
    expect(useLlmStore.getState().streams).toEqual({});
    expect(useLlmStore.getState().mutatingConversations).toEqual({});
    expect(run.isCurrent()).toBe(false);
    expect(unlisten).toHaveBeenCalledTimes(1);
    await expect(run.invoke('llm_send_message', { ...owner })).rejects.toThrow('expired request');
    expect(invokeCommand).not.toHaveBeenCalled();
  });

  it('drops old account and same-account re-unlock events without damaging the current request', async () => {
    const owner = identity();
    const old = start(owner);
    await old.ready;
    const oldHandler = mockListen.mock.calls[0][1] as ChunkHandler;
    setRequestSession('account-b');
    setRequestSession(accountId);
    const nextOwner = identity('conversation-a', 'request-next');
    await start(nextOwner).ready;
    useLlmStore.getState().onChunk(chunk(nextOwner, { chunk: 'Current', sessionGeneration: 888 }));
    oldHandler({ payload: chunk(owner, { chunk: 'Old', isDone: true }) });
    useLlmStore.getState().finishStream(owner, 'Old rejection');
    expect(old.isCurrent()).toBe(false);
    expect(snapshot(nextOwner)).toMatchObject({
      requestId: 'request-next',
      buffer: 'Current',
      error: null,
    });
  });

  it('releases a late old listener resolution and preserves the new listener and stream', async () => {
    const oldSubscription = deferred<UnlistenFn>();
    const oldUnlisten = vi.fn();
    const newUnlisten = vi.fn();
    mockListen.mockReturnValueOnce(oldSubscription.promise).mockResolvedValue(newUnlisten);
    const old = start();
    const oldReady = old.ready.catch((error: unknown) => error);
    await Promise.resolve();
    const oldHandler = mockListen.mock.calls[0][1] as ChunkHandler;
    setRequestSession(null);
    setRequestSession(accountId);
    const nextOwner = identity('conversation-a', 'request-next');
    const next = start(nextOwner);
    await next.ready;
    oldSubscription.resolve(oldUnlisten);
    expect(await oldReady).toBeInstanceOf(Error);
    oldHandler({ payload: chunk(nextOwner, { chunk: 'Old listener forged current identity' }) });
    expect(oldUnlisten).toHaveBeenCalledTimes(1);
    expect(newUnlisten).not.toHaveBeenCalled();
    expect(next.isCurrent()).toBe(true);
    expect(snapshot(nextOwner).buffer).toBe('');
    useLlmStore.getState().finishStream(nextOwner);
    expect(newUnlisten).toHaveBeenCalledTimes(1);
  });

  it('contains a late old listener rejection without clearing the new stream or subscription', async () => {
    const oldSubscription = deferred<UnlistenFn>();
    const newUnlisten = vi.fn();
    mockListen.mockReturnValueOnce(oldSubscription.promise).mockResolvedValue(newUnlisten);
    const oldOwner = identity();
    const old = start(oldOwner);
    const oldReady = old.ready.catch((error: unknown) => {
      useLlmStore.getState().finishStream(oldOwner, String(error));
      return error;
    });
    await Promise.resolve();
    setRequestSession(null);
    setRequestSession(accountId);
    const nextOwner = identity('conversation-a', 'request-next');
    await start(nextOwner).ready;
    oldSubscription.reject(new Error('Old subscription failed'));
    expect(await oldReady).toBeInstanceOf(Error);
    expect(snapshot(nextOwner)).toMatchObject({
      error: null,
      settled: false,
      requestId: 'request-next',
    });
    expect(newUnlisten).not.toHaveBeenCalled();
    const otherOwner = identity('conversation-b', 'request-other');
    await start(otherOwner).ready;
    expect(mockListen).toHaveBeenCalledTimes(2);
  });

  it('ignores a late request catch after locking and starting a new request in the same account', async () => {
    const operation = deferred<string>();
    vi.mocked(invokeCommand).mockReturnValueOnce(operation.promise);
    const oldOwner = identity();
    const old = start(oldOwner);
    await old.ready;
    const oldOutcome = old.invoke<string>('llm_send_message', { ...oldOwner }).then(
      (text) => useLlmStore.getState().finishStream(oldOwner, undefined, text),
      (error: unknown) => useLlmStore.getState().finishStream(oldOwner, String(error)),
    );
    setRequestSession(null);
    setRequestSession(accountId);
    const nextOwner = identity('conversation-a', 'request-next');
    await start(nextOwner).ready;
    useLlmStore.getState().onChunk(chunk(nextOwner, { chunk: 'New response' }));
    operation.reject(new Error('Old upstream failed'));
    await oldOutcome;
    expect(snapshot(nextOwner)).toMatchObject({
      buffer: 'New response',
      error: null,
      settled: false,
    });
  });

  it('rejects late invoke success after cancellation instead of settling the replacement request', async () => {
    const operation = deferred<string>();
    vi.mocked(invokeCommand).mockReturnValueOnce(operation.promise);
    const oldOwner = identity();
    const old = start(oldOwner);
    await old.ready;
    const result = old
      .invoke<string>('llm_send_message', { ...oldOwner })
      .catch((error: unknown) => error);
    useLlmStore.getState().cancelStream(oldOwner);
    const nextOwner = identity('conversation-a', 'request-next');
    await start(nextOwner).ready;
    operation.resolve('Old response');
    expect(await result).toBeInstanceOf(Error);
    useLlmStore.getState().finishStream(oldOwner, undefined, 'Old response');
    expect(snapshot(nextOwner)).toMatchObject({
      requestId: 'request-next',
      buffer: '',
      settled: false,
    });
  });

  it('does not let old cancellation, clearing, or persist warnings affect a replacement request', async () => {
    const oldOwner = identity();
    await start(oldOwner).ready;
    useLlmStore.getState().finishStream(oldOwner);
    const nextOwner = identity('conversation-a', 'request-next');
    await start(nextOwner).ready;
    useLlmStore.getState().markPersistFailure(oldOwner);
    useLlmStore.getState().cancelStream(oldOwner);
    useLlmStore.getState().clearCompleted(oldOwner);
    expect(useLlmStore.getState().claimPersistFailure(oldOwner)).toBe(false);
    expect(snapshot(nextOwner)).toMatchObject({
      requestId: 'request-next',
      persistFailed: false,
      settled: false,
    });
  });

  it('refuses a new send owned by a different active account', () => {
    const run = useLlmStore.getState().startStream({
      ...identity(),
      accountId: 'account-b',
      assistantMessageId: 'assistant-b',
      messages: messages(),
    });
    expect(run).toBeNull();
    expect(useLlmStore.getState().streams).toEqual({});
    expect(mockListen).not.toHaveBeenCalled();
  });

  it.each(['llm_rename_conversation', 'llm_soft_delete_conversation'])(
    'blocks %s when sending reserved the same conversation first',
    async (command) => {
      const owner = identity();
      await start(owner).ready;
      await expect(
        invokeConversationChange(command, { ...owner, name: 'Renamed' }),
      ).rejects.toThrow('当前对话正在处理');
      expect(invokeCommand).not.toHaveBeenCalled();
      expect(snapshot(owner).settled).toBe(false);
    },
  );

  it.each(['llm_rename_conversation', 'llm_soft_delete_conversation'])(
    'blocks a send when %s synchronously reserved the same conversation first',
    async (command) => {
      const operation = deferred<void>();
      vi.mocked(invokeCommand).mockReturnValue(operation.promise);
      const owner = identity();
      const mutation = invokeConversationChange(command, { ...owner, name: 'Renamed' });
      expect(isConversationBusy(useLlmStore.getState(), accountId, owner.conversationId)).toBe(
        true,
      );
      expect(
        useLlmStore.getState().startStream({
          ...owner,
          assistantMessageId: 'assistant-a',
          messages: messages(),
        }),
      ).toBeNull();
      expect(mockListen).not.toHaveBeenCalled();
      operation.resolve(undefined);
      await mutation;
      expect(isConversationBusy(useLlmStore.getState(), accountId, owner.conversationId)).toBe(
        false,
      );
      await start(owner).ready;
    },
  );

  it('allows unrelated conversations while a rename is pending and rejects a second same-conversation mutation', async () => {
    const operation = deferred<void>();
    vi.mocked(invokeCommand).mockReturnValueOnce(operation.promise).mockResolvedValue(undefined);
    const mutation = invokeConversationChange('llm_rename_conversation', {
      accountId,
      conversationId: 'conversation-a',
      name: 'Renamed',
    });
    await expect(
      invokeConversationChange('llm_soft_delete_conversation', {
        accountId,
        conversationId: 'conversation-a',
      }),
    ).rejects.toThrow('当前对话正在处理');
    const otherOwner = identity('conversation-b', 'request-b');
    await start(otherOwner).ready;
    await invokeConversationChange('llm_rename_conversation', {
      accountId,
      conversationId: 'conversation-c',
      name: 'Other',
    });
    expect(snapshot(otherOwner).settled).toBe(false);
    expect(isConversationBusy(useLlmStore.getState(), accountId, 'conversation-a')).toBe(true);
    operation.resolve(undefined);
    await mutation;
    expect(isConversationBusy(useLlmStore.getState(), accountId, 'conversation-a')).toBe(false);
  });

  it('does not let an expired mutation finally release a new mutation reservation', async () => {
    const oldOperation = deferred<void>();
    const nextOperation = deferred<void>();
    vi.mocked(invokeCommand)
      .mockReturnValueOnce(oldOperation.promise)
      .mockReturnValueOnce(nextOperation.promise);
    const args = { accountId, conversationId: 'conversation-a', name: 'Renamed' };
    const old = invokeConversationChange('llm_rename_conversation', args).catch(
      (error: unknown) => error,
    );
    setRequestSession(null);
    setRequestSession(accountId);
    const next = invokeConversationChange('llm_rename_conversation', args);
    oldOperation.resolve(undefined);
    expect(await old).toBeInstanceOf(Error);
    expect(isConversationBusy(useLlmStore.getState(), accountId, args.conversationId)).toBe(true);
    nextOperation.resolve(undefined);
    await next;
    expect(isConversationBusy(useLlmStore.getState(), accountId, args.conversationId)).toBe(false);
  });
});
