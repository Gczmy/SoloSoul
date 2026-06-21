import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

type UnlistenFn = () => void;

// 模拟 @tauri-apps/api/event
const mockListen = vi.fn();
vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

describe('llmStore', () => {
  beforeEach(() => {
    mockListen.mockReset();
  });

  afterEach(() => {
    vi.resetModules();
  });

  describe('startStream', () => {
    it('设置流式状态并订阅 Tauri 事件', async () => {
      const unlistenFn: UnlistenFn = vi.fn();
      mockListen.mockResolvedValue(unlistenFn);

      const { useLlmStore } = await import('./llmStore');
      useLlmStore.getState().startStream('conv-1');

      // 立即状态：流开始，buffer 清空
      const state = useLlmStore.getState();
      expect(state.isStreaming).toBe(true);
      expect(state.streamingConvId).toBe('conv-1');
      expect(state.streamBuffer).toBe('');
      expect(state.streamError).toBeNull();

      // 等待 listen Promise 解析
      await vi.waitFor(() => {
        expect(useLlmStore.getState().unlisten).toBeDefined();
      });

      expect(mockListen).toHaveBeenCalledWith('llm-stream-chunk', expect.any(Function));
    });

    it('startStream 时取消旧 listener', async () => {
      const oldUnlisten = vi.fn();
      const newUnlisten = vi.fn();

      const { useLlmStore } = await import('./llmStore');
      // 模拟已有 listener
      useLlmStore.setState({ unlisten: oldUnlisten });
      mockListen.mockResolvedValue(newUnlisten);

      useLlmStore.getState().startStream('conv-2');
      await vi.waitFor(() => {
        expect(useLlmStore.getState().unlisten).toBe(newUnlisten);
      });

      // 旧 listener 被取消
      expect(oldUnlisten).toHaveBeenCalledTimes(1);
    });

    it('listen 失败时设置 streamError 并停止流', async () => {
      mockListen.mockRejectedValue(new Error('Tauri event error'));

      const { useLlmStore } = await import('./llmStore');
      useLlmStore.getState().startStream('conv-1');

      await vi.waitFor(() => {
        const state = useLlmStore.getState();
        expect(state.isStreaming).toBe(false);
        expect(state.streamError).toBe('Error: Tauri event error');
      });
    });
  });

  describe('onChunk', () => {
    it('追加普通 chunk 到 streamBuffer', async () => {
      const unlistenFn: UnlistenFn = vi.fn();
      mockListen.mockResolvedValue(unlistenFn);

      const { useLlmStore } = await import('./llmStore');
      useLlmStore.getState().startStream('conv-1');
      await vi.waitFor(() => { expect(useLlmStore.getState().unlisten).toBeDefined(); });

      // 模拟收到 chunk
      useLlmStore.getState().onChunk({ conversationId: 'conv-1', chunk: 'Hello', isDone: false });
      expect(useLlmStore.getState().streamBuffer).toBe('Hello');

      useLlmStore.getState().onChunk({ conversationId: 'conv-1', chunk: ' World', isDone: false });
      expect(useLlmStore.getState().streamBuffer).toBe('Hello World');
    });

    it('isDone 时停止流并取消 listener（最后一块内容在前一个 !isDone 事件中到达）', async () => {
      const unlistenFn = vi.fn();
      mockListen.mockResolvedValue(unlistenFn);

      const { useLlmStore } = await import('./llmStore');
      useLlmStore.getState().startStream('conv-1');
      await vi.waitFor(() => { expect(useLlmStore.getState().unlisten).toBeDefined(); });

      // 先发送内容（最后一块在 isDone 之前到达）
      useLlmStore.getState().onChunk({ conversationId: 'conv-1', chunk: 'Hello', isDone: false });
      // 然后发 isDone 信号
      useLlmStore.getState().onChunk({ conversationId: 'conv-1', chunk: '', isDone: true });

      const state = useLlmStore.getState();
      expect(state.isStreaming).toBe(false);
      expect(state.streamBuffer).toBe('Hello'); // 内容保留
      expect(unlistenFn).toHaveBeenCalledTimes(1); // 已取消订阅
    });

    it('isDone 不追加 chunk 到 buffer（isDone 是结束标记，内容已由前序事件送达）', async () => {
      const unlistenFn = vi.fn();
      mockListen.mockResolvedValue(unlistenFn);

      const { useLlmStore } = await import('./llmStore');
      useLlmStore.getState().startStream('conv-1');
      await vi.waitFor(() => { expect(useLlmStore.getState().unlisten).toBeDefined(); });

      // 只发 isDone（没有前序内容）
      useLlmStore.getState().onChunk({ conversationId: 'conv-1', chunk: 'Orphan', isDone: true });

      const state = useLlmStore.getState();
      expect(state.isStreaming).toBe(false);
      expect(state.streamBuffer).toBe(''); // isDone 本身不追加内容
    });

    it('error 时设置 streamError 并停止流', async () => {
      const unlistenFn = vi.fn();
      mockListen.mockResolvedValue(unlistenFn);

      const { useLlmStore } = await import('./llmStore');
      useLlmStore.getState().startStream('conv-1');
      await vi.waitFor(() => { expect(useLlmStore.getState().unlisten).toBeDefined(); });

      useLlmStore.getState().onChunk({ conversationId: 'conv-1', chunk: '', isDone: false, error: 'API timeout' });

      const state = useLlmStore.getState();
      expect(state.isStreaming).toBe(false);
      expect(state.streamError).toBe('API timeout');
      expect(unlistenFn).toHaveBeenCalledTimes(1);
    });

    it('忽略不匹配 conversationId 的 chunk', async () => {
      const unlistenFn = vi.fn();
      mockListen.mockResolvedValue(unlistenFn);

      const { useLlmStore } = await import('./llmStore');
      useLlmStore.getState().startStream('conv-1');
      await vi.waitFor(() => { expect(useLlmStore.getState().unlisten).toBeDefined(); });

      // 写入一些内容
      useLlmStore.getState().onChunk({ conversationId: 'conv-1', chunk: 'Keep', isDone: false });
      // 错误的 conversationId
      useLlmStore.getState().onChunk({ conversationId: 'conv-2', chunk: 'Ignore', isDone: false });

      expect(useLlmStore.getState().streamBuffer).toBe('Keep');
    });
  });

  describe('stopStream', () => {
    it('停止流并清理状态', async () => {
      const unlistenFn: UnlistenFn = vi.fn();
      mockListen.mockResolvedValue(unlistenFn);

      const { useLlmStore } = await import('./llmStore');
      useLlmStore.getState().startStream('conv-1');
      await vi.waitFor(() => { expect(useLlmStore.getState().unlisten).toBeDefined(); });

      // 先写入 buffer
      useLlmStore.getState().onChunk({ conversationId: 'conv-1', chunk: 'Partial', isDone: false });

      useLlmStore.getState().stopStream();

      const state = useLlmStore.getState();
      expect(state.isStreaming).toBe(false);
      expect(state.streamingConvId).toBeNull();
      expect(state.streamBuffer).toBe('Partial'); // buffer 保留
      expect(state.unlisten).toBeNull();
      expect(unlistenFn).toHaveBeenCalled();
    });
  });

  describe('reset', () => {
    it('完全重置所有流状态', async () => {
      const unlistenFn: UnlistenFn = vi.fn();
      mockListen.mockResolvedValue(unlistenFn);

      const { useLlmStore } = await import('./llmStore');
      useLlmStore.getState().startStream('conv-1');
      await vi.waitFor(() => { expect(useLlmStore.getState().unlisten).toBeDefined(); });

      useLlmStore.getState().onChunk({ conversationId: 'conv-1', chunk: 'Data', isDone: true });

      useLlmStore.getState().reset();

      const state = useLlmStore.getState();
      expect(state.isStreaming).toBe(false);
      expect(state.streamingConvId).toBeNull();
      expect(state.streamBuffer).toBe('');
      expect(state.streamError).toBeNull();
      expect(state.unlisten).toBeNull();
      expect(state.unlistenPromise).toBeNull();
    });
  });
});
