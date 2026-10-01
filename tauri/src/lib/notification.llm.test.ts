import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { waitFor } from '@testing-library/react';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import { invokeCommand } from '@/lib/ipcClient';
import { setRequestSession } from '@/lib/sessionRequests';
import { useUiStore } from '@/stores/uiStore';
import {
  selectLlmStream,
  useLlmStore,
  type LlmStreamPayload,
  type StreamIdentity,
} from '@/stores/llmStore';
import {
  initLlmNotificationListener,
  markConversationPending,
  sendSystemNotificationWithFallback,
  setAiPageOpen,
  setQuickChatOpen,
} from './notification';

const { mockListen } = vi.hoisted(() => ({ mockListen: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));
vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
  sendNotification: vi.fn(),
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
  sessionGeneration: 500,
  chunk: '',
  isDone: false,
  error: null,
  ...overrides,
});
const disposers: Array<() => void> = [];
const flushNotifications = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

async function subscribe() {
  const unsubscribe = await initLlmNotificationListener();
  disposers.push(unsubscribe);
  return unsubscribe;
}

async function start(owner = identity()) {
  const run = useLlmStore.getState().startStream({
    ...owner,
    assistantMessageId: `assistant-${owner.requestId}`,
    messages: [],
  });
  expect(run).not.toBeNull();
  if (!run) throw new Error('Expected the notification request to reserve a conversation');
  await run.ready;
  return run;
}

describe('AI completion notifications use the current settled request', () => {
  beforeEach(() => {
    useLlmStore.getState().reset();
    setRequestSession(null);
    vi.resetAllMocks();
    mockListen.mockImplementation(async () => vi.fn());
    vi.mocked(isPermissionGranted).mockResolvedValue(true);
    vi.mocked(requestPermission).mockResolvedValue('denied');
    vi.mocked(invokeCommand).mockResolvedValue({ notificationPermissionRequested: true });
    setRequestSession(accountId);
    setAiPageOpen(false);
    setQuickChatOpen(false);
  });

  afterEach(() => {
    for (const dispose of disposers.splice(0)) dispose();
    useLlmStore.getState().reset();
    setRequestSession(null);
    setAiPageOpen(false);
    setQuickChatOpen(false);
    vi.restoreAllMocks();
  });

  it('subscribes to store settlement and notifies once after invoke completes outside both chat views', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    expect(mockListen).not.toHaveBeenCalled();
    const owner = identity();
    await start(owner);
    markConversationPending(owner);
    useLlmStore.getState().onChunk(chunk(owner, { chunk: 'Complete', isDone: true }));
    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
    expect(mockListen).toHaveBeenCalledTimes(1);

    useLlmStore.getState().finishStream(owner);
    await waitFor(() => expect(sendNotification).toHaveBeenCalledTimes(1));
    expect(toastSpy).toHaveBeenCalledOnce();
    useLlmStore.getState().finishStream(owner);
    useLlmStore.getState().onChunk(chunk(owner, { isDone: true }));
    expect(sendNotification).toHaveBeenCalledTimes(1);
    expect(toastSpy).toHaveBeenCalledTimes(1);
  });

  it.each(['page', 'quick'] as const)(
    'suppresses completion alerts while the %s chat view is open',
    async (view) => {
      const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
      await subscribe();
      const owner = identity();
      await start(owner);
      markConversationPending(owner);
      if (view === 'page') setAiPageOpen(true);
      else setQuickChatOpen(true);
      useLlmStore.getState().finishStream(owner);
      expect(sendNotification).not.toHaveBeenCalled();
      expect(isPermissionGranted).not.toHaveBeenCalled();
      expect(toastSpy).not.toHaveBeenCalled();
      setAiPageOpen(false);
      setQuickChatOpen(false);
      useLlmStore.getState().finishStream(owner);
      expect(sendNotification).not.toHaveBeenCalled();
    },
  );

  it('shows a background persistence-failure toast once even if done preceded the failure', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    await subscribe();
    const owner = identity();
    await start(owner);
    markConversationPending(owner);
    useLlmStore.getState().onChunk(chunk(owner, { chunk: '完整回复', isDone: true }));
    useLlmStore
      .getState()
      .onChunk(chunk(owner, { isDone: true, error: '__LLM_PERSIST_FAILED__: db full' }));
    expect(toastSpy).toHaveBeenCalledTimes(1);
    expect(toastSpy).toHaveBeenCalledWith({
      type: 'error',
      message: expect.any(String),
      duration: 5000,
    });
    expect(useLlmStore.getState().claimPersistFailure(owner)).toBe(false);
    useLlmStore
      .getState()
      .onChunk(chunk(owner, { isDone: true, error: '__LLM_PERSIST_FAILED__: repeated failure' }));
    useLlmStore.getState().finishStream(owner, 'save failed');
    expect(toastSpy).toHaveBeenCalledTimes(1);
    expect(sendNotification).not.toHaveBeenCalled();
    expect(isPermissionGranted).not.toHaveBeenCalled();
    expect(
      selectLlmStream(useLlmStore.getState(), owner.accountId, owner.conversationId),
    ).toMatchObject({ buffer: '完整回复', persistFailed: true, settled: true, error: null });
  });

  it('shares the persist warning claim with chat consumers so a previously reported failure is not duplicated', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    const owner = identity();
    await start(owner);
    useLlmStore.getState().markPersistFailure(owner);
    expect(useLlmStore.getState().claimPersistFailure(owner)).toBe(true);
    await subscribe();
    markConversationPending(owner);
    useLlmStore.getState().finishStream(owner);
    expect(toastSpy).not.toHaveBeenCalled();
    expect(sendNotification).not.toHaveBeenCalled();
  });

  it('does not notify for a stream generation error or a subsequent done event', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    const owner = identity();
    await start(owner);
    markConversationPending(owner);
    useLlmStore.getState().onChunk(chunk(owner, { error: 'model error' }));
    useLlmStore.getState().onChunk(chunk(owner, { isDone: true }));
    useLlmStore.getState().finishStream(owner);
    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
  });

  it('does not notify when invoke settlement failed without a stream error event', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    const owner = identity();
    await start(owner);
    markConversationPending(owner);
    useLlmStore.getState().finishStream(owner, 'invoke rejected');
    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
  });

  it('notifies after canonical invoke text recovers missing stream events', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    const owner = identity();
    await start(owner);
    markConversationPending(owner);
    useLlmStore.getState().finishStream(owner, undefined, 'Canonical response');
    await waitFor(() => expect(sendNotification).toHaveBeenCalledTimes(1));
    expect(
      selectLlmStream(useLlmStore.getState(), owner.accountId, owner.conversationId)?.buffer,
    ).toBe('Canonical response');
    expect(toastSpy).toHaveBeenCalledOnce();
  });

  it('keeps two conversations independent and notifies each completed request once', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    const firstOwner = identity();
    const secondOwner = identity('conversation-b', 'request-b');
    await start(firstOwner);
    await start(secondOwner);
    markConversationPending(firstOwner);
    markConversationPending(secondOwner);
    useLlmStore.getState().finishStream(secondOwner);
    await waitFor(() => expect(sendNotification).toHaveBeenCalledTimes(1));
    expect(
      selectLlmStream(useLlmStore.getState(), firstOwner.accountId, firstOwner.conversationId)
        ?.settled,
    ).toBe(false);
    useLlmStore.getState().finishStream(firstOwner);
    await waitFor(() => expect(sendNotification).toHaveBeenCalledTimes(2));
    expect(toastSpy).toHaveBeenCalledTimes(2);
    expect(mockListen).toHaveBeenCalledTimes(1);
  });

  it('ignores a cancelled request completion and notifies only its replacement request', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    const oldOwner = identity();
    await start(oldOwner);
    markConversationPending(oldOwner);
    useLlmStore.getState().cancelStream(oldOwner);
    const nextOwner = identity('conversation-a', 'request-next');
    await start(nextOwner);
    markConversationPending(nextOwner);
    useLlmStore.getState().finishStream(oldOwner);
    expect(sendNotification).not.toHaveBeenCalled();
    useLlmStore.getState().finishStream(nextOwner);
    await waitFor(() => expect(sendNotification).toHaveBeenCalledTimes(1));
    expect(toastSpy).toHaveBeenCalledOnce();
  });

  it('ignores an old-account pending identity and same-account pre-lock completion', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    const oldOwner = identity();
    await start(oldOwner);
    markConversationPending(oldOwner);
    setRequestSession(null);
    setRequestSession(accountId);
    const nextOwner = identity('conversation-a', 'request-next');
    await start(nextOwner);
    markConversationPending(oldOwner);
    useLlmStore.getState().onChunk(chunk(oldOwner, { isDone: true }));
    useLlmStore.getState().finishStream(oldOwner);
    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
    markConversationPending(nextOwner);
    useLlmStore.getState().finishStream(nextOwner);
    await waitFor(() => expect(sendNotification).toHaveBeenCalledTimes(1));
  });

  it('suppresses both system notification and toast if the session locks while permission is pending', async () => {
    const permission = deferred<boolean>();
    vi.mocked(isPermissionGranted).mockReturnValue(permission.promise);
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    const owner = identity();
    await start(owner);
    markConversationPending(owner);
    useLlmStore.getState().finishStream(owner);
    expect(isPermissionGranted).toHaveBeenCalledTimes(1);
    setRequestSession(null);
    permission.resolve(true);
    await waitFor(() => expect(useLlmStore.getState().isCurrent(owner)).toBe(false));
    await flushNotifications();
    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
  });

  it('suppresses a permission-delayed completion once a new request replaces the same conversation', async () => {
    const permission = deferred<boolean>();
    vi.mocked(isPermissionGranted).mockReturnValueOnce(permission.promise).mockResolvedValue(true);
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    const oldOwner = identity();
    await start(oldOwner);
    markConversationPending(oldOwner);
    useLlmStore.getState().finishStream(oldOwner);
    const nextOwner = identity('conversation-a', 'request-next');
    await start(nextOwner);
    markConversationPending(nextOwner);
    permission.resolve(true);
    await flushNotifications();
    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
    useLlmStore.getState().finishStream(nextOwner);
    await waitFor(() => expect(sendNotification).toHaveBeenCalledTimes(1));
    expect(toastSpy).toHaveBeenCalledOnce();
  });

  it('does not add a fallback toast after a delayed permission error from an expired request', async () => {
    const permission = deferred<boolean>();
    vi.mocked(isPermissionGranted).mockReturnValue(permission.promise);
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await subscribe();
    const owner = identity();
    await start(owner);
    markConversationPending(owner);
    useLlmStore.getState().finishStream(owner);
    setRequestSession(null);
    permission.reject(new Error('Permission backend failed'));
    await flushNotifications();
    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
  });

  it('stops notifications when the caller releases the store subscription', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    const unsubscribe = await subscribe();
    const owner = identity();
    await start(owner);
    markConversationPending(owner);
    unsubscribe();
    useLlmStore.getState().finishStream(owner);
    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
  });

  it('falls back to an in-app toast when notification permission was denied', async () => {
    vi.mocked(isPermissionGranted).mockResolvedValue(false);
    vi.mocked(invokeCommand).mockResolvedValue({ notificationPermissionRequested: true });
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await sendSystemNotificationWithFallback('AI result', 'Finished', 'Open chat', 'info');
    expect(sendNotification).not.toHaveBeenCalled();
    expect(requestPermission).not.toHaveBeenCalled();
    expect(toastSpy).toHaveBeenCalledWith({
      message: 'Open chat',
      type: 'info',
      duration: 5000,
    });
  });

  it('sends a direct system notification and only adds a toast when requested', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await sendSystemNotificationWithFallback('AI result', 'Finished');
    expect(sendNotification).toHaveBeenCalledWith({ title: 'AI result', body: 'Finished' });
    expect(toastSpy).not.toHaveBeenCalled();
    await sendSystemNotificationWithFallback('AI result', 'Finished', undefined, 'info', true);
    expect(sendNotification).toHaveBeenCalledTimes(2);
    expect(toastSpy).toHaveBeenCalledOnce();
  });

  it('keeps the direct-call fallback for a native notification exception', async () => {
    vi.mocked(sendNotification).mockImplementation(() => {
      throw new Error('Native notification unavailable');
    });
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await sendSystemNotificationWithFallback('AI result', 'Finished', 'Open chat', 'warning');
    expect(toastSpy).toHaveBeenCalledWith({
      message: 'Open chat',
      type: 'warning',
      duration: 5000,
    });
  });
});
