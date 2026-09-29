import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { RefObject } from 'react';
import type { ObjectSummary } from '@/stores/objectStore';
import { useObjectRulerPosition } from './useObjectRulerPosition';
import { objectRulerAnchorId } from './objectRuler';

const objects: ObjectSummary[] = ['first', 'second', 'third'].map((id) => ({
  id,
  name: id,
  typeId: 'note',
  sensitivityLevel: 'public',
  createdAt: '2026-09-29T00:00:00Z',
  updatedAt: '2026-09-29T00:00:00Z',
}));

function fixture() {
  const scroller = document.createElement('main');
  const list = document.createElement('div');
  scroller.append(list);
  document.body.append(scroller);
  Object.defineProperties(scroller, {
    clientHeight: { configurable: true, value: 400 },
    scrollHeight: { configurable: true, value: 1200 },
  });
  scroller.getBoundingClientRect = () => ({ top: 0 }) as DOMRect;
  const scrollTo = vi.fn();
  scroller.scrollTo = scrollTo;
  const listRef = { current: list } as RefObject<HTMLDivElement>;
  const addAnchor = (id: string, top: number) => {
    const anchor = document.createElement('div');
    anchor.id = objectRulerAnchorId(id);
    anchor.getBoundingClientRect = () => ({ top }) as DOMRect;
    const button = document.createElement('button');
    button.setAttribute('role', 'button');
    anchor.append(button);
    list.append(anchor);
    return { anchor, button };
  };
  return { scroller, listRef, scrollTo, addAnchor };
}

let animationFrames: FrameRequestCallback[];
beforeEach(() => {
  animationFrames = [];
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    animationFrames.push(callback);
    return animationFrames.length;
  });
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
});

afterEach(() => {
  document.body.innerHTML = '';
  delete document.documentElement.dataset.reduceMotion;
  vi.unstubAllGlobals();
});

describe('RF-1013 object ruler positioning', () => {
  it('tracks the card at the reading line and selects the last card at the bottom', () => {
    const { scroller, listRef, addAnchor } = fixture();
    addAnchor('first', 100);
    addAnchor('second', 500);
    addAnchor('third', 900);
    const { result } = renderHook(() =>
      useObjectRulerPosition({ objects, renderedCount: 3, listRef, revealObject: vi.fn() }),
    );
    expect(result.current.activeId).toBe('first');

    act(() => {
      scroller.scrollTop = 450;
      scroller.dispatchEvent(new Event('scroll'));
      animationFrames.splice(0).forEach((callback) => callback(0));
    });
    expect(result.current.activeId).toBe('second');

    act(() => {
      scroller.scrollTop = 800;
      scroller.dispatchEvent(new Event('scroll'));
      animationFrames.splice(0).forEach((callback) => callback(0));
    });
    expect(result.current.activeId).toBe('third');
  });

  it('reveals a virtualized target, scrolls to it, focuses it, and clears its highlight on unmount', () => {
    const { listRef, scrollTo, addAnchor } = fixture();
    addAnchor('first', 100);
    const revealObject = vi.fn();
    document.documentElement.dataset.reduceMotion = 'true';
    const { result, rerender, unmount } = renderHook(
      ({ renderedCount }) =>
        useObjectRulerPosition({ objects, renderedCount, listRef, revealObject }),
      { initialProps: { renderedCount: 1 } },
    );

    act(() => result.current.navigateTo(2, true));
    expect(revealObject).toHaveBeenCalledWith(2);
    expect(scrollTo).not.toHaveBeenCalled();

    const { anchor, button } = addAnchor('third', 900);
    rerender({ renderedCount: 3 });
    expect(scrollTo).toHaveBeenCalledWith({ top: 884, behavior: 'instant' });
    expect(result.current.activeId).toBe('third');
    expect(anchor.dataset.rulerTarget).toBe('true');
    expect(document.activeElement).toBe(button);

    unmount();
    expect(anchor.dataset.rulerTarget).toBeUndefined();
  });
});
