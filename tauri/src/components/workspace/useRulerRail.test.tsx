import { act, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { resizeObserverInstances } from '@/test/setup';
import { useRulerRail } from './useRulerRail';

function createRail() {
  const container = document.createElement('div');
  const rail = document.createElement('div');
  let containerHeight = 200;
  Object.defineProperty(container, 'clientHeight', {
    configurable: true,
    get: () => containerHeight,
  });
  Object.defineProperty(rail, 'clientHeight', { configurable: true, value: 64 });
  Object.defineProperty(rail, 'scrollHeight', { configurable: true, value: 400 });
  const scrollTo = vi.fn(({ top }: ScrollToOptions) => {
    rail.scrollTop = top ?? 0;
  });
  Object.defineProperty(rail, 'scrollTo', { configurable: true, value: scrollTo });
  return {
    container,
    rail,
    scrollTo,
    setContainerHeight: (height: number) => {
      containerHeight = height;
    },
  };
}

describe('useRulerRail scroll behavior', () => {
  it('accumulates small wheel movements into rows and respects line/page units', () => {
    const { container, rail, scrollTo } = createRail();
    const { result, unmount } = renderHook(() =>
      useRulerRail({ current: container }, { current: rail }, 20),
    );
    expect(result.current.step).toBe(16);
    expect(rail.style.height).toBe('176px');
    expect(rail.scrollTop).toBe(0);

    const wheel = (deltaY: number, deltaMode = 0, ctrlKey = false) => {
      const event = new WheelEvent('wheel', { deltaY, deltaMode, ctrlKey, cancelable: true });
      rail.dispatchEvent(event);
      return event;
    };
    expect(wheel(8).defaultPrevented).toBe(true);
    expect(rail.scrollTop).toBe(0);
    wheel(8);
    expect(rail.scrollTop).toBe(16);
    wheel(-8);
    expect(rail.scrollTop).toBe(16);
    wheel(-8);
    expect(rail.scrollTop).toBe(0);
    wheel(2, 1);
    expect(rail.scrollTop).toBe(32);
    wheel(1, 2);
    expect(rail.scrollTop).toBe(96);
    expect(wheel(30, 0, true).defaultPrevented).toBe(false);
    expect(wheel(0).defaultPrevented).toBe(false);
    expect(rail.scrollTop).toBe(96);

    const callsBeforeUnmount = scrollTo.mock.calls.length;
    unmount();
    wheel(16);
    expect(scrollTo).toHaveBeenCalledTimes(callsBeforeUnmount);
  });

  it('centers a selected index, clamps boundaries and realigns on resize', () => {
    const { container, rail, setContainerHeight } = createRail();
    const previousObserverCount = resizeObserverInstances.length;
    const { result, unmount } = renderHook(() =>
      useRulerRail({ current: container }, { current: rail }, 20),
    );
    const observer = resizeObserverInstances[previousObserverCount];
    expect(observer.observe).toHaveBeenCalledWith(container);

    act(() => result.current.scrollToIndex(12));
    expect(rail.scrollTop).toBe(160);
    act(() => result.current.scrollToIndex(100));
    expect(rail.scrollTop).toBe(336);
    act(() => result.current.scrollToIndex(-10));
    expect(rail.scrollTop).toBe(0);

    rail.scrollTop = 25;
    setContainerHeight(100);
    act(() => observer.trigger());
    expect(rail.style.height).toBe('64px');
    expect(rail.scrollTop).toBe(32);
    unmount();
    expect(observer.disconnect).toHaveBeenCalled();
  });

  it('handles absent refs without registering a resize observer', () => {
    const previousObserverCount = resizeObserverInstances.length;
    const { result } = renderHook(() => useRulerRail({ current: null }, { current: null }, 0));
    expect(result.current.step).toBe(18);
    expect(() => result.current.scrollToIndex(4)).not.toThrow();
    expect(resizeObserverInstances).toHaveLength(previousObserverCount);
  });
});
