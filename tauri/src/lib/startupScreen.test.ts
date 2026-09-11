import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { dismissStartupScreen } from './startupScreen';

describe('静态启动层交接', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML = '<div id="startup-screen" role="status">SoloSoul</div>';
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) =>
      setTimeout(() => callback(performance.now()), 16),
    );
    vi.stubGlobal('cancelAnimationFrame', clearTimeout);
  });

  afterEach(() => {
    document.body.innerHTML = '';
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('目标页面获得绘制机会后才淡出，再移除启动层', () => {
    dismissStartupScreen();
    vi.advanceTimersByTime(16);
    expect(document.getElementById('startup-screen')).not.toHaveAttribute('aria-hidden');
    vi.advanceTimersByTime(16);
    expect(document.getElementById('startup-screen')).toHaveAttribute('aria-hidden', 'true');
    vi.advanceTimersByTime(180);
    expect(document.getElementById('startup-screen')).toBeNull();
  });

  it('StrictMode 撤销第一次 effect 不会提前移除覆盖层，第二次可正常交接', () => {
    const cancel = dismissStartupScreen();
    cancel();
    vi.runAllTimers();
    expect(document.getElementById('startup-screen')).toBeVisible();
    dismissStartupScreen();
    vi.runAllTimers();
    expect(document.getElementById('startup-screen')).toBeNull();
  });

  it('减少动态效果时不等待淡出动画', () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true }));
    dismissStartupScreen();
    vi.advanceTimersByTime(33);
    expect(document.getElementById('startup-screen')).toBeNull();
  });
});
