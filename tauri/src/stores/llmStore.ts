import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { logger } from '@/lib/logger';

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
  /** 流级错误（生成中断）——前端会把已展示回复替换为错误文案。 */
  streamError: string | null;
  /** 持久化失败标记（is_done=true 且 error 带 __LLM_PERSIST_FAILED__ 前缀）——
   *  回复已完整展示，仅提示保存失败，不替换内容。 */
  persistFailed: boolean;
  unlisten: UnlistenFn | null;
  unlistenPromise: Promise<UnlistenFn> | null;

  startStream: (conversationId: string) => void;
  onChunk: (payload: LlmStreamPayload) => void;
  reset: () => void;
}

export const useLlmStore = create<LlmState>((set, get) => ({
  isStreaming: false,
  streamingConvId: null,
  streamBuffer: '',
  streamError: null,
  persistFailed: false,
  unlisten: null,
  unlistenPromise: null,

  startStream: (conversationId) => {
    const state = get();
    // Cancel any previous listener (sync + pending) before subscribing again
    state.unlisten?.();
    if (state.unlistenPromise) {
      state.unlistenPromise
        .then((fn) => fn())
        .catch((err) => logger.warn('[llmStore] Failed to clean up old listener:', err));
    }

    set({
      isStreaming: true,
      streamingConvId: conversationId,
      streamBuffer: '',
      streamError: null,
      persistFailed: false,
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
      if (payload.isDone && payload.error.startsWith('__LLM_PERSIST_FAILED__')) {
        // P002-R1: 持久化失败——流已正常结束（is_done=true），回复已完整展示。
        // 保留 buffer，仅置 persistFailed 标记供页面 toast，不替换已展示内容。
        set({ isStreaming: false, persistFailed: true, streamError: null });
      } else {
        // 流级错误（生成中断）：中断并标记错误（前端替换为错误文案）
        set({ streamError: payload.error, isStreaming: false });
      }
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

  reset: () => {
    const state = get();
    state.unlisten?.();
    if (state.unlistenPromise) {
      state.unlistenPromise
        .then((fn) => fn())
        .catch((err) => logger.warn('[llmStore] Failed to clean up old listener:', err));
    }
    set({
      isStreaming: false,
      streamingConvId: null,
      streamBuffer: '',
      streamError: null,
      persistFailed: false,
      unlisten: null,
      unlistenPromise: null,
    });
  },
}));
