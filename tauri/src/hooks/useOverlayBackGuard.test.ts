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

  it('内层返回/关闭按钮：直接关闭内层回主体，不依赖 history.back()', () => {
    // 回归（T014 用户反馈）：查看器点左上角返回/右上角关闭直接退出到相册上一层。
    // 原实现栈顶有内层标记时走 history.back()——安卓 WebView 对纯 pushState
    // 历史栈的 back() 导航不可靠（整栈弹出/页面重载），现改为直接关内层；
    // 内层标记保留在栈中，由后续硬件返回或卸载清理统一弹出。
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
    // 直接关内层（回网格），不触发任何浏览器导航
    expect(onCloseInner).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();
    expect(window.history.back).not.toHaveBeenCalled();

    // 内层标记仍保留在栈顶：网格态硬件返回 → popstate 关闭整个浮层（标记被弹）
    act(() => rerender({ innerOpen: false }));
    act(() => {
      window.dispatchEvent(new PopStateEvent('popstate'));
    });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('内层返回按钮：无内层标记时同样直接关闭内层（同一路径，无分支）', () => {
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

  it('vault 锁定压入外部条目后卸载：不误弹外部条目，残留标记在后续返回被 sweeper 跳过', () => {
    const onClose = vi.fn();
    const { unmount } = renderHook(() =>
      useOverlayBackGuard({ innerOpen: false, onCloseInner: vi.fn(), onClose }),
    );

    // 模拟 vault 锁定：外部 navigate('/login') 压入条目（顶层非本浮层标记）
    window.history.pushState({ login: true, idx: 2 }, '', '/login');
    act(() => {
      unmount();
    });
    // 卸载清理应跳过：顶层非标记 → 不误弹外部 /login 条目
    expect(window.history.go).not.toHaveBeenCalled();

    // 用户后续硬件返回：浏览器弹出外部条目，落到残留标记上 → sweeper 自动 go(-1) 跳过
    act(() => {
      window.dispatchEvent(
        new PopStateEvent('popstate', { state: { solosoulOverlayLayer: true, idx: 1 } }),
      );
    });
    expect(window.history.go).toHaveBeenCalledWith(-1);
    expect(onClose).not.toHaveBeenCalled();
  });

  it('活跃浮层返回时 sweeper 不跳过（标记仍被认领，由钩子自身监听关闭）', () => {
    const onClose = vi.fn();
    renderHook(() => useOverlayBackGuard({ innerOpen: false, onCloseInner: vi.fn(), onClose }));

    // 取当前顶层 state（即本钩子压入的标记对象，引用一致 → 仍在 ownedMarkers 中）
    const markerState = window.history.state as Record<string, unknown>;
    act(() => {
      window.dispatchEvent(new PopStateEvent('popstate', { state: markerState }));
    });

    // sweeper 不跳过（owned）；钩子自身 popstate 监听关闭浮层
    expect(window.history.go).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('vault 锁定埋藏两层标记：后续返回连续跳过全部残留标记', () => {
    const onClose = vi.fn();
    const onCloseInner = vi.fn();
    const { rerender, unmount } = renderHook(
      ({ innerOpen }: { innerOpen: boolean }) =>
        useOverlayBackGuard({ innerOpen, onCloseInner, onClose }),
      { initialProps: { innerOpen: false } },
    );

    // 打开查看器（两层标记）→ vault 锁定压入外部条目 → 卸载
    act(() => rerender({ innerOpen: true }));
    window.history.pushState({ login: true, idx: 3 }, '', '/login');
    act(() => {
      unmount();
    });
    expect(window.history.go).not.toHaveBeenCalled();

    // 返回：先落内层残留标记 → 跳过；再落浮层残留标记 → 跳过
    act(() => {
      window.dispatchEvent(
        new PopStateEvent('popstate', {
          state: { solosoulOverlayLayer: true, solosoulOverlayInnerLayer: true, idx: 2 },
        }),
      );
    });
    act(() => {
      window.dispatchEvent(
        new PopStateEvent('popstate', { state: { solosoulOverlayLayer: true, idx: 1 } }),
      );
    });
    expect(window.history.go).toHaveBeenCalledTimes(2);
    expect(window.history.go).toHaveBeenLastCalledWith(-1);
    expect(onClose).not.toHaveBeenCalled();
    expect(onCloseInner).not.toHaveBeenCalled();

    // 级联终止：落到真实条目（无标记）后不再继续 go(-1)
    vi.mocked(window.history.go).mockClear();
    act(() => {
      window.dispatchEvent(new PopStateEvent('popstate', { state: { real: true, idx: 0 } }));
    });
    expect(window.history.go).not.toHaveBeenCalled();
  });

  it('卸载时清理残留标记：仅当顶层仍是本浮层标记才 history.go(-n)', async () => {
    const onClose = vi.fn();
    const onCloseInner = vi.fn();
    const { unmount, rerender } = renderHook(
      ({ innerOpen }: { innerOpen: boolean }) =>
        useOverlayBackGuard({ innerOpen, onCloseInner, onClose }),
      { initialProps: { innerOpen: false } },
    );

    // 查看器打开（2 层标记），随后卸载 → 清理延迟到微任务，冲洗后 go(-2)
    act(() => rerender({ innerOpen: true }));
    act(() => {
      unmount();
    });
    await act(async () => {
      await Promise.resolve();
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
    await act(async () => {
      await Promise.resolve();
    });
    expect(window.history.go).not.toHaveBeenCalled();
  });

  it('dev StrictMode 重挂载：清理的延迟 go 不误弹新实例标记（相册打开即关闭回归）', async () => {
    const onClose1 = vi.fn();
    const onClose2 = vi.fn();

    // StrictMode 开发模式：挂载 → 清理 → 重挂载（同步连续执行）。
    // 浏览器中清理期的 history.go 是异步排队的遍历，会在重挂载实例的 pushState
    // 之后才执行——若直接调用会从重挂载后的位置误弹新标记并触发新实例 popstate。
    const { unmount } = renderHook(() =>
      useOverlayBackGuard({ innerOpen: false, onCloseInner: vi.fn(), onClose: onClose1 }),
    );
    unmount();
    // 重挂载新实例：压入自己的新标记，接管浮层
    renderHook(() =>
      useOverlayBackGuard({ innerOpen: false, onCloseInner: vi.fn(), onClose: onClose2 }),
    );

    // 冲洗微任务：首实例清理的延迟 go 此时执行——栈顶已是新实例标记（身份不等）
    // → 必须跳过，不得误弹；浮层保持打开，onClose2 不被误触发。
    await act(async () => {
      await Promise.resolve();
    });
    expect(window.history.go).not.toHaveBeenCalled();
    expect(onClose2).not.toHaveBeenCalled();

    // 用户真实硬件返回（浏览器弹出新实例标记）：正常关闭浮层
    act(() => {
      window.dispatchEvent(new PopStateEvent('popstate', { state: window.history.state }));
    });
    expect(onClose2).toHaveBeenCalledTimes(1);
    expect(onClose1).not.toHaveBeenCalled();
  });
});
