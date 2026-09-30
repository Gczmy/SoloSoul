import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { invokeCommand } from '@/lib/ipcClient';
import type { ChatMsg } from '@/types/llmChat';

export interface StreamIdentity {
  accountId: string;
  conversationId: string;
  requestId: string;
}
export interface LlmStreamPayload extends StreamIdentity {
  sessionGeneration: number;
  chunk: string;
  isDone: boolean;
  error?: string | null;
}
export interface StreamSnapshot extends StreamIdentity {
  assistantMessageId: string;
  messages: ChatMsg[];
  buffer: string;
  error: string | null;
  persistFailed: boolean;
  failureNotified: boolean;
  settled: boolean;
  persisted: boolean;
  backendGeneration: number | null;
}
type Ticket = ReturnType<ReturnType<typeof createSessionRequests>['begin']>;
export interface StreamRun {
  ready: Promise<void>;
  isCurrent: () => boolean;
  assertCurrent: () => void;
  invoke: Ticket['invoke'];
}
interface LlmState {
  streams: Record<string, StreamSnapshot>;
  mutatingConversations: Record<string, boolean>;
  startStream: (
    initial: StreamIdentity & { assistantMessageId: string; messages: ChatMsg[] },
  ) => StreamRun | null;
  isCurrent: (identity: StreamIdentity) => boolean;
  onChunk: (payload: LlmStreamPayload) => void;
  prepareStream: (identity: StreamIdentity, messages: ChatMsg[]) => void;
  markConversationPersisted: (identity: StreamIdentity) => void;
  finishStream: (identity: StreamIdentity, error?: string, finalText?: string) => void;
  markPersistFailure: (identity: StreamIdentity) => void;
  cancelStream: (identity: StreamIdentity, savedHistory?: ChatMsg[]) => void;
  clearCompleted: (identity: StreamIdentity) => void;
  claimPersistFailure: (identity: StreamIdentity) => boolean;
  reset: () => void;
}
const keyOf = ({
  accountId,
  conversationId,
}: Pick<StreamIdentity, 'accountId' | 'conversationId'>) =>
  JSON.stringify([accountId, conversationId]);
export function selectLlmStream(
  state: Pick<LlmState, 'streams'>,
  accountId?: string,
  conversationId?: string | null,
) {
  return accountId && conversationId
    ? state.streams[keyOf({ accountId, conversationId })]
    : undefined;
}
const requests = createSessionRequests();
const tickets = new Map<string, Ticket>();
const mutations = new Map<string, symbol>();
let listener: UnlistenFn | null = null;
let pendingListener: Promise<UnlistenFn> | null = null;
let listenerGeneration = 0;
function stopListener() {
  listenerGeneration += 1;
  listener?.();
  listener = null;
  pendingListener = null;
}
function ensureListener(): Promise<UnlistenFn> {
  if (listener) return Promise.resolve(listener);
  if (pendingListener) return pendingListener;
  const generation = listenerGeneration;
  const pending = Promise.resolve().then(() =>
    listen<LlmStreamPayload>('llm-stream-chunk', (event) => {
      if (generation === listenerGeneration) useLlmStore.getState().onChunk(event.payload);
    }),
  );
  pendingListener = pending;
  void pending.then(
    (unlisten) => {
      if (generation !== listenerGeneration) unlisten();
      else {
        listener = unlisten;
        pendingListener = null;
      }
    },
    () => {
      if (generation === listenerGeneration) pendingListener = null;
    },
  );
  return pending;
}
function stopIfIdle() {
  if (!Object.values(useLlmStore.getState().streams).some((stream) => !stream.settled))
    stopListener();
}

export const useLlmStore = create<LlmState>((set, get) => ({
  streams: {},
  mutatingConversations: {},
  startStream: (initial) => {
    const key = keyOf(initial);
    if (get().mutatingConversations[key] || (get().streams[key] && !get().streams[key].settled))
      return null;
    const ticket = requests.begin(undefined, initial.accountId);
    if (!ticket.isCurrent()) return null;
    tickets.set(key, ticket);
    set((state) => ({
      streams: {
        ...state.streams,
        [key]: {
          ...initial,
          messages: initial.messages.map((message) => ({ ...message })),
          buffer: '',
          error: null,
          persistFailed: false,
          failureNotified: false,
          settled: false,
          persisted: get().streams[key]?.persisted === true,
          backendGeneration: null,
        },
      },
    }));
    const isCurrent = () =>
      ticket.isCurrent() && get().streams[key]?.requestId === initial.requestId;
    const assertCurrent = () => {
      if (!isCurrent()) throw new Error('Stream belongs to an expired request');
    };
    const invoke: Ticket['invoke'] = async <T>(
      command: string,
      args?: Record<string, unknown>,
      options?: Parameters<Ticket['invoke']>[2],
    ): Promise<T> => {
      assertCurrent();
      const value = await ticket.invoke<T>(command, args, {
        ...options,
        requestIsCurrent: isCurrent,
      });
      assertCurrent();
      return value;
    };
    return {
      isCurrent,
      assertCurrent,
      invoke,
      ready: ensureListener().then(() => {
        assertCurrent();
      }),
    };
  },
  isCurrent: (identity) =>
    get().streams[keyOf(identity)]?.requestId === identity.requestId &&
    tickets.get(keyOf(identity))?.isCurrent() === true,
  markConversationPersisted: (identity) => {
    if (!get().isCurrent(identity)) return;
    const key = keyOf(identity);
    set((state) => ({
      streams: { ...state.streams, [key]: { ...state.streams[key], persisted: true } },
    }));
  },
  prepareStream: (identity, messages) => {
    const key = keyOf(identity),
      stream = get().streams[key];
    if (!get().isCurrent(identity) || stream.settled) return;
    set((state) => ({
      streams: {
        ...state.streams,
        [key]: {
          ...stream,
          messages: messages.map((message) => ({ ...message })),
        },
      },
    }));
  },
  onChunk: (payload) => {
    if (
      !payload ||
      typeof payload.accountId !== 'string' ||
      typeof payload.conversationId !== 'string' ||
      typeof payload.requestId !== 'string' ||
      (payload.error != null && typeof payload.error !== 'string') ||
      !Number.isSafeInteger(payload.sessionGeneration) ||
      payload.sessionGeneration < 0 ||
      typeof payload.chunk !== 'string' ||
      typeof payload.isDone !== 'boolean'
    )
      return;
    const key = keyOf(payload),
      stream = get().streams[key];
    if (
      !get().isCurrent(payload) ||
      stream.settled ||
      (stream.backendGeneration !== null && stream.backendGeneration !== payload.sessionGeneration)
    )
      return;
    const persistFailed =
      payload.isDone && payload.error?.startsWith('__LLM_PERSIST_FAILED__') === true;
    set((state) => ({
      streams: {
        ...state.streams,
        [key]: {
          ...stream,
          backendGeneration: payload.sessionGeneration,
          buffer: stream.buffer + payload.chunk,
          error: persistFailed ? stream.error : payload.error || stream.error,
          persistFailed: stream.persistFailed || persistFailed,
        },
      },
    }));
    // isDone可能带尾正文且早于后端保存，监听直到invoke结算，不能在此退订。
  },
  finishStream: (identity, error, finalText) => {
    const key = keyOf(identity),
      stream = get().streams[key];
    if (!get().isCurrent(identity) || stream.settled) return;
    set((state) => ({
      streams: {
        ...state.streams,
        [key]: {
          ...stream,
          buffer: finalText ?? stream.buffer,
          settled: true,
          error: stream.persistFailed ? stream.error : error || stream.error,
        },
      },
    }));
    stopIfIdle();
  },
  markPersistFailure: (identity) => {
    if (!get().isCurrent(identity)) return;
    const key = keyOf(identity);
    set((state) => ({
      streams: { ...state.streams, [key]: { ...state.streams[key], persistFailed: true } },
    }));
  },
  cancelStream: (identity, savedHistory) => {
    if (!get().isCurrent(identity)) return;
    const key = keyOf(identity);
    if (savedHistory) {
      // 预保存失败时保留已读取的规范历史，另一入口不能退回过期的本地消息。
      const stream = get().streams[key];
      set((state) => ({
        streams: {
          ...state.streams,
          [key]: {
            ...stream,
            messages: savedHistory.map((message) => ({ ...message })),
            assistantMessageId: '',
            buffer: '',
            error: 'Request cancelled before sending',
            persisted: true,
            settled: true,
          },
        },
      }));
    } else {
      const streams = { ...get().streams };
      delete streams[key];
      tickets.delete(key);
      set({ streams });
    }
    stopIfIdle();
  },
  clearCompleted: (identity) => {
    if (get().streams[keyOf(identity)]?.settled) get().cancelStream(identity);
  },
  claimPersistFailure: (identity) => {
    const key = keyOf(identity),
      stream = get().streams[key];
    if (!get().isCurrent(identity) || !stream.persistFailed || stream.failureNotified) return false;
    set((state) => ({
      streams: { ...state.streams, [key]: { ...stream, failureNotified: true } },
    }));
    return true;
  },
  reset: () => {
    requests.invalidate();
    tickets.clear();
    mutations.clear();
    stopListener();
    set({ streams: {}, mutatingConversations: {} });
  },
}));
onRequestSessionChange(() => useLlmStore.getState().reset());

/** 与发送同步占位，防止read→save覆盖同时发生的改名/删除；两种启动顺序均互斥。 */
export async function invokeConversationChange<T>(
  command: Parameters<typeof invokeCommand>[0],
  args: { accountId: string; conversationId: string; [key: string]: unknown },
): Promise<T> {
  const key = keyOf(args),
    state = useLlmStore.getState();
  if (mutations.has(key) || (state.streams[key] && !state.streams[key].settled))
    throw new Error('当前对话正在处理，请完成后重试');
  const token = Symbol();
  mutations.set(key, token);
  useLlmStore.setState({ mutatingConversations: { ...state.mutatingConversations, [key]: true } });
  const request = requests.begin(undefined, args.accountId);
  try {
    return await request.invoke<T>(command, args);
  } finally {
    if (mutations.get(key) === token) {
      mutations.delete(key);
      const busy = { ...useLlmStore.getState().mutatingConversations };
      delete busy[key];
      useLlmStore.setState({ mutatingConversations: busy });
    }
  }
}
export function isConversationBusy(
  state: Pick<LlmState, 'streams' | 'mutatingConversations'>,
  accountId?: string,
  conversationId?: string | null,
): boolean {
  if (!accountId || !conversationId) return false;
  const key = keyOf({ accountId, conversationId });
  return (
    state.mutatingConversations[key] === true ||
    (state.streams[key] !== undefined && !state.streams[key].settled)
  );
}
