import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { waitFor } from '@testing-library/react';
import { isPermissionGranted, sendNotification } from '@tauri-apps/plugin-notification';
import { invokeCommand } from '@/lib/ipcClient';
import { useUiStore } from '@/stores/uiStore';
import {
  initLlmNotificationListener,
  markConversationPending,
  sendSystemNotificationWithFallback,
  setAiPageOpen,
  setQuickChatOpen,
} from './notification';

type StreamEvent = {
  payload: { conversationId: string; chunk: string; isDone: boolean; error?: string };
};
const eventStub = vi.hoisted(() => ({
  handlers: new Map<string, (event: StreamEvent) => void>(),
  unlisten: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, handler: (event: StreamEvent) => void) => {
    eventStub.handlers.set(name, handler);
    return eventStub.unlisten;
  }),
}));
vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
  sendNotification: vi.fn(),
}));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: vi.fn() }));

const streamEvent = (conversationId: string, isDone: boolean, error?: string): StreamEvent => ({
  payload: { conversationId, chunk: '', isDone, error },
});

describe('AI completion notifications', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventStub.handlers.clear();
    vi.mocked(isPermissionGranted).mockResolvedValue(true);
    setAiPageOpen(false);
    setQuickChatOpen(false);
  });

  afterEach(() => {
    setAiPageOpen(false);
    setQuickChatOpen(false);
    vi.restoreAllMocks();
  });

  it('notifies once when a pending stream finishes outside both chat views', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    const unlisten = await initLlmNotificationListener();
    const onChunk = eventStub.handlers.get('llm-stream-chunk')!;
    markConversationPending('conversation-a');

    onChunk(streamEvent('conversation-a', false));
    expect(sendNotification).not.toHaveBeenCalled();
    onChunk(streamEvent('conversation-a', true));
    await waitFor(() => expect(sendNotification).toHaveBeenCalledTimes(1));
    expect(toastSpy).toHaveBeenCalledOnce();
    onChunk(streamEvent('conversation-a', true));
    expect(sendNotification).toHaveBeenCalledTimes(1);
    expect(unlisten).toBe(eventStub.unlisten);
  });

  it('suppresses completion alerts while either chat view is open', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await initLlmNotificationListener();
    const onChunk = eventStub.handlers.get('llm-stream-chunk')!;

    setAiPageOpen(true);
    markConversationPending('conversation-page');
    onChunk(streamEvent('conversation-page', true));
    setAiPageOpen(false);
    setQuickChatOpen(true);
    markConversationPending('conversation-card');
    onChunk(streamEvent('conversation-card', true));

    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
  });

  it('clears a failed stream so a later completion cannot notify', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await initLlmNotificationListener();
    const onChunk = eventStub.handlers.get('llm-stream-chunk')!;
    markConversationPending('conversation-error');

    onChunk(streamEvent('conversation-error', false, 'model error'));
    onChunk(streamEvent('conversation-error', true));
    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
  });

  it('falls back to an in-app toast when notification permission was denied', async () => {
    vi.mocked(isPermissionGranted).mockResolvedValue(false);
    vi.mocked(invokeCommand).mockResolvedValue({ notificationPermissionRequested: true });
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});

    await sendSystemNotificationWithFallback('AI result', 'Finished', 'Open chat', 'info');

    expect(sendNotification).not.toHaveBeenCalled();
    expect(toastSpy).toHaveBeenCalledWith({
      message: 'Open chat',
      type: 'info',
      duration: 5000,
    });
  });

  it('sends a system notification and only adds a toast when requested', async () => {
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});

    await sendSystemNotificationWithFallback('AI result', 'Finished');
    expect(sendNotification).toHaveBeenCalledWith({ title: 'AI result', body: 'Finished' });
    expect(toastSpy).not.toHaveBeenCalled();

    await sendSystemNotificationWithFallback('AI result', 'Finished', undefined, 'info', true);
    expect(sendNotification).toHaveBeenCalledTimes(2);
    expect(toastSpy).toHaveBeenCalledOnce();
  });
});
