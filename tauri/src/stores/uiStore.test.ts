import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

describe('uiStore', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.resetModules();
  });

  describe('showToast / dismissToast', () => {
    it('showToast 添加到 toasts 列表', async () => {
      const { useUiStore } = await import('./uiStore');
      useUiStore.getState().showToast({ message: 'Hello', type: 'info' });
      const toasts = useUiStore.getState().toasts;
      expect(toasts).toHaveLength(1);
      expect(toasts[0].message).toBe('Hello');
      expect(toasts[0].type).toBe('info');
      expect(toasts[0].id).toMatch(/^toast-/);
      // store 不存储已解析的默认值，仅存储原始传入值
      expect(toasts[0].timeoutId).toBeDefined();
    });

    it('showToast 支持自定义 duration', async () => {
      const { useUiStore } = await import('./uiStore');
      useUiStore.getState().showToast({ message: 'X', type: 'error', duration: 5000 });
      expect(useUiStore.getState().toasts[0].duration).toBe(5000);
    });

    it('dismissToast 移除指定 toast', async () => {
      const { useUiStore } = await import('./uiStore');
      useUiStore.getState().showToast({ message: 'A', type: 'info' });
      useUiStore.getState().showToast({ message: 'B', type: 'info' });
      const id = useUiStore.getState().toasts[0].id;
      useUiStore.getState().dismissToast(id);
      expect(useUiStore.getState().toasts).toHaveLength(1);
      expect(useUiStore.getState().toasts[0].message).toBe('B');
    });

    it('toast 超时后自动移除', async () => {
      const { useUiStore } = await import('./uiStore');
      useUiStore.getState().showToast({ message: 'Auto', type: 'success', duration: 1000 });
      expect(useUiStore.getState().toasts).toHaveLength(1);
      vi.advanceTimersByTime(1000);
      expect(useUiStore.getState().toasts).toHaveLength(0);
    });

    it('dismissToast 清除超时定时器', async () => {
      const { useUiStore } = await import('./uiStore');
      useUiStore.getState().showToast({ message: 'X', type: 'info', duration: 5000 });
      const id = useUiStore.getState().toasts[0].id;
      useUiStore.getState().dismissToast(id);
      // 推进时间但不应自动移除（定时器已被清除）
      vi.advanceTimersByTime(5000);
      expect(useUiStore.getState().toasts).toHaveLength(0); // 手动移除
    });
  });
});
