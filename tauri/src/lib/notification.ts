import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { useUiStore } from '@/stores/uiStore';

interface LlmStreamPayload {
  conversationId: string;
  chunk: string;
  isDone: boolean;
  error?: string;
}

/**
 * Tracks conversation IDs for which the user sent a message and is awaiting
 * an AI response. When the stream completes and the user is no longer viewing
 * the AI chat (full page or quick card), a system notification + in-app toast
 * are shown.
 */
const pendingConversations = new Set<string>();
let unlisten: UnlistenFn | null = null;

/**
 * Initialize the global LLM stream listener for notification purposes.
 * Should be called once at app startup.
 */
export async function initLlmNotificationListener(): Promise<void> {
  const permission = await isPermissionGranted();
  if (!permission) {
    await requestPermission();
  }

  if (unlisten) return;

  unlisten = await listen<LlmStreamPayload>('llm-stream-chunk', (event) => {
    const payload = event.payload;

    // Error or not done → don't notify yet (but still clear error ones)
    if (payload.error) {
      pendingConversations.delete(payload.conversationId);
      return;
    }
    if (!payload.isDone) return;

    if (!pendingConversations.has(payload.conversationId)) return;
    pendingConversations.delete(payload.conversationId);

    const isOnAiPage = window.location.pathname.startsWith('/llm-chat');
    const hasQuickChatOpen = document.querySelector('[data-ai-quick-chat]') !== null;

    if (!isOnAiPage && !hasQuickChatOpen) {
      // System notification
      sendNotification({
        title: 'SoloSoul AI',
        body: 'AI 已完成回复，点击查看',
      });
      // In-app toast (sync with system notification)
      useUiStore.getState().showToast({
        message: 'AI 已完成回复',
        type: 'info',
        duration: 5000,
      });
    }
  });
}

/**
 * Mark a conversation as pending notification. Call this right before
 * invoking `llm_send_message_stream`.
 */
export function markConversationPending(convId: string): void {
  pendingConversations.add(convId);
}

/**
 * Clear a pending conversation (e.g. when user manually cancels or leaves).
 */
export function clearPendingConversation(convId: string): void {
  pendingConversations.delete(convId);
}
