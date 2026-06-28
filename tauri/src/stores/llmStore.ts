import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface LlmStreamPayload {
  conversationId: string;
  chunk: string;
  isDone: boolean;
  error?: string;
}

interface LlmState {
  isStreaming: boolean;
  streamingConvId: string | null;
  streamBuffer: string;
  streamError: string | null;
  unlisten: UnlistenFn | null;
  unlistenPromise: Promise<UnlistenFn> | null;

  startStream: (conversationId: string) => void;
  onChunk: (payload: LlmStreamPayload) => void;
  stopStream: () => void;
  reset: () => void;
}

export const useLlmStore = create<LlmState>((set, get) => ({
  isStreaming: false,
  streamingConvId: null,
  streamBuffer: '',
  streamError: null,
  unlisten: null,
  unlistenPromise: null,

  startStream: (conversationId) => {
    const state = get();
    // Cancel any previous listener (sync + pending) before subscribing again
    state.unlisten?.();
    if (state.unlistenPromise) {
      state.unlistenPromise.then((fn) => fn()).catch((err) => console.warn('[llmStore] Failed to clean up old listener:', err));
    }

    set({
      isStreaming: true,
      streamingConvId: conversationId,
      streamBuffer: '',
      streamError: null,
      unlisten: null,
      unlistenPromise: null,
    });

    // 订阅 Tauri Event
    const pending = listen<LlmStreamPayload>('llm-stream-chunk', (event) => {
      get().onChunk(event.payload);
    });
    set({ unlistenPromise: pending });
    pending
      .then((unlistenFn) => {
        set({ unlisten: unlistenFn, unlistenPromise: null });
      })
      .catch((err) => {
        set({ streamError: String(err), isStreaming: false, unlistenPromise: null });
      });
  },

  onChunk: (payload) => {
    const state = get();
    if (payload.conversationId !== state.streamingConvId) return;

    if (payload.error) {
      set({ streamError: payload.error, isStreaming: false });
      state.unlisten?.();
      return;
    }

    if (payload.isDone) {
      set({ isStreaming: false });
      state.unlisten?.();
      return;
    }

    set({ streamBuffer: state.streamBuffer + payload.chunk });
  },

  stopStream: () => {
    const state = get();
    state.unlisten?.();
    if (state.unlistenPromise) {
      state.unlistenPromise.then((fn) => fn()).catch((err) => console.warn('[llmStore] Failed to clean up old listener:', err));
    }
    set({ isStreaming: false, streamingConvId: null, unlisten: null, unlistenPromise: null });
  },

  reset: () => {
    const state = get();
    state.unlisten?.();
    if (state.unlistenPromise) {
      state.unlistenPromise.then((fn) => fn()).catch((err) => console.warn('[llmStore] Failed to clean up old listener:', err));
    }
    set({
      isStreaming: false,
      streamingConvId: null,
      streamBuffer: '',
      streamError: null,
      unlisten: null,
      unlistenPromise: null,
    });
  },
}));
