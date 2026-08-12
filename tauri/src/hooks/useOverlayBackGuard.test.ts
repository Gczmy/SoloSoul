import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useOverlayBackGuard } from './useOverlayBackGuard';

/** 读取当前 history.state（测试专用辅助）。 */
function topState(): Record<string, unknown> {
  return (window.history.state as Record<string, unknown>) ?? {};
}

describe('useOverlayBackGuard', () => {
  beforeEach(() => {
    // jsdom 无真实导航：go/back 全部 mock，避免清理期触发真实 history 操作
    vi.spyOn(window.history, 'go').mockImplementation(() => {});
    vi.spyOn(window.history, 'back').mockImplementation(() => {});
    window.history.replaceState(null, '');
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('挂载时压入「浮层层」历史标记（URL 不变、idx 递增）', () => {
    // beforeEach 已 replaceState(null) 重置，prevIdx=0 → 挂载后 idx=1
    renderHook(() =>
      useOverlayBackGuard({ innerOpen: false, onCloseInner: vi.fn(), onClose: vi.fn() }),
    );
    const state = topState();
    expect(state.solosoulOverlayLayer).toBe(true);
    expect(state.idx).toBe(1);
    expect(window.location.pathname).toBe('/');
  });

  it('网格态硬件返回（popstate）：关闭整个浮层而非跳路由', () => {
    const onClose = vi.fn();
    const onCloseInner = vi.fn();
    renderHook(() => useOverlayBackGuard({ innerOpen: false, onCloseInner, onClose }));

    act(() => {
      window.dispatchEvent(new PopStateEvent('popstate'));
    });

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(onCloseInner).not.toHaveBeenCalled();
  });

  it('查看器打开时再压入「内层层」标记；返回先回浮层主体，再返回才关闭', () => {
    const onClose = vi.fn();
    const onCloseInner = vi.fn();
    const { rerender } = renderHook(
      ({ innerOpen }: { innerOpen: boolean }) =>
        useOverlayBackGuard({ innerOpen, onCloseInner, onClose }),
      { initialProps: { innerOpen: false } },
    );

    // 打开查看器 → 压入内层层
    act(() => rerender({ innerOpen: true }));
    expect(topState().solosoulOverlayInnerLayer).toBe(true);

    // 第一次返回：内层开着 → 回浮层主体（onCloseInner），不关浮层
    act(() => {
      window.dispatchEvent(new PopStateEvent('popstate'));
    });
    expect(onCloseInner).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();

    // 回到网格态后再返回 → 关闭浮层
    act(() => rerender({ innerOpen: false }));
    act(() => {
      window.dispatchEvent(new PopStateEvent('popstate'));
    });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('内层返回按钮：栈顶是内层标记时主动 history.back()（触发 popstate 回主体）', () => {
    const onClose = vi.fn();
    const onCloseInner = vi.fn();
    const { result, rerender } = renderHook(
      ({ innerOpen }: { innerOpen: boolean }) =>
        useOverlayBackGuard({ innerOpen, onCloseInner, onClose }),
      { initialProps: { innerOpen: false } },
    );

    act(() => rerender({ innerOpen: true }));
    expect(topState().solosoulOverlayInnerLayer).toBe(true);

    act(() => {
      result.current.handleInnerBack();
    });
    expect(window.history.back).toHaveBeenCalledTimes(1);
    // 浏览器随后弹出内层标记并派发 popstate → 回浮层主体
    act(() => {
      window.dispatchEvent(new PopStateEvent('popstate'));
    });
    expect(onCloseInner).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();
  });

  it('内层返回按钮兜底：栈顶无内层标记时直接关闭内层', () => {
    const onCloseInner = vi.fn();
    const { result } = renderHook(() =>
      useOverlayBackGuard({ innerOpen: true, onCloseInner, onClose: vi.fn() }),
    );

    // 模拟内层标记已被弹出（栈顶回到浮层层，无内层标记）
    const s = topState();
    delete s.solosoulOverlayInnerLayer;
    window.history.replaceState(s, '');

    act(() => {
      result.current.handleInnerBack();
    });
    expect(window.history.back).not.toHaveBeenCalled();
    expect(onCloseInner).toHaveBeenCalledTimes(1);
  });

  it('卸载时清理残留标记：仅当顶层仍是本浮层标记才 history.go(-n)', () => {
    const onClose = vi.fn();
    const onCloseInner = vi.fn();
    const { unmount, rerender } = renderHook(
      ({ innerOpen }: { innerOpen: boolean }) =>
        useOverlayBackGuard({ innerOpen, onCloseInner, onClose }),
      { initialProps: { innerOpen: false } },
    );

    // 查看器打开（2 层标记），随后卸载 → go(-2) 清理
    act(() => rerender({ innerOpen: true }));
    act(() => {
      unmount();
    });
    expect(window.history.go).toHaveBeenCalledWith(-2);

    // 外部条目叠加后卸载：顶层非本浮层标记 → 不清理（避免误弹外部条目）
    vi.mocked(window.history.go).mockClear();
    const { unmount: unmount2 } = renderHook(() =>
      useOverlayBackGuard({ innerOpen: false, onCloseInner: vi.fn(), onClose: vi.fn() }),
    );
    window.history.replaceState({ external: true }, '');
    act(() => {
      unmount2();
    });
    expect(window.history.go).not.toHaveBeenCalled();
  });
});
