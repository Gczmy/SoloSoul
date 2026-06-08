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

  startStream: (conversationId) => {
    // 如果已有监听器，先取消
    get().unlisten?.();

    set({
      isStreaming: true,
      streamingConvId: conversationId,
      streamBuffer: '',
      streamError: null,
    });

    // 订阅 Tauri Event
    listen<LlmStreamPayload>('llm-stream-chunk', (event) => {
      get().onChunk(event.payload);
    }).then((unlistenFn) => {
      set({ unlisten: unlistenFn });
    }).catch((err) => {
      set({ streamError: String(err), isStreaming: false });
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
    set({ isStreaming: false, streamingConvId: null, unlisten: null });
  },

  reset: () => {
    get().unlisten?.();
    set({
      isStreaming: false,
      streamingConvId: null,
      streamBuffer: '',
      streamError: null,
      unlisten: null,
    });
  },
}));
