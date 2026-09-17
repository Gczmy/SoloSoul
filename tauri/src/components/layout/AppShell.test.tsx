import { describe, it, expect, afterEach, vi } from 'vitest';
import { render, fireEvent, act, screen } from '@testing-library/react';
import { MemoryRouter, Routes, Route, Link } from 'react-router-dom';
import { AppShell } from './AppShell';
import { ShellNotificationsProvider } from './ShellNotifications';
import { resizeObserverInstances } from '@/test/setup';

vi.mock('@/hooks/useIsNarrowViewport', () => ({
  useIsNarrowViewport: () => false,
}));

vi.mock('@/stores/settingsStore', () => ({
  useSettingsStore: (selector: (s: unknown) => unknown) =>
    selector({ settings: { sidebarPosition: 'left' } }),
}));

vi.mock('@/stores/syncStore', () => {
  const stubState = {
    incomingPairingRequest: null,
    initPairingRequestListener: async () => () => {},
    initSyncCompletedListener: async () => () => {},
  };
  const hook = (selector: (s: unknown) => unknown) => selector(stubState);
  return {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    useSyncStore: Object.assign(hook, { getState: () => stubState }) as any,
  };
});

vi.mock('./SideNavigation', () => ({
  SideNavigation: () => <div data-testid="side-nav" />,
}));
vi.mock('./TopFunctionBar', () => ({
  TopFunctionBar: () => <div data-testid="top-bar" />,
}));
vi.mock('./MobileBottomNav', () => ({
  MobileBottomNav: () => <div data-testid="bottom-nav" />,
}));
vi.mock('./AppBar', () => ({
  AppBar: () => <div data-testid="app-bar" />,
}));
vi.mock('@/components/sync/PairingDialog', () => ({
  PairingDialog: () => null,
}));

// 页面 A 渲染一个可滚动容器，模拟「在页面中部」的场景
function TallPageA() {
  return (
    <div>
      <div style={{ height: 2000 }}>内容 A（可滚动）</div>
      <Link to="/b">去 B</Link>
    </div>
  );
}

describe('AppShell 路由导航后内容区滚动重置', () => {
  afterEach(() => vi.restoreAllMocks());

  it('通知按正常顺序插入，不重建正文或重置输入、焦点和滚动位置', () => {
    const layout = (notifications: React.ReactNode) => (
      <MemoryRouter>
        <ShellNotificationsProvider notifications={notifications}>
          <AppShell title="首页">
            <input aria-label="未完成的编辑" />
          </AppShell>
        </ShellNotificationsProvider>
      </MemoryRouter>
    );
    const { rerender } = render(layout(null));
    const content = document.querySelector<HTMLElement>('[data-shell-content]')!;
    const input = screen.getByLabelText('未完成的编辑');
    fireEvent.change(input, { target: { value: '保留草稿' } });
    input.focus();
    content.scrollTop = 500;
    rerender(layout(<button>取消下载</button>));
    expect(content.previousElementSibling).toBe(
      document.querySelector('[data-shell-notifications]'),
    );
    expect(content.previousElementSibling).toContainElement(screen.getByText('取消下载'));
    expect(document.querySelector('[data-shell-content]')).toBe(content);
    expect(content.scrollTop).toBe(500);
    expect(input).toHaveValue('保留草稿');
    expect(input).toHaveFocus();
    rerender(layout(null));
    expect(content.scrollTop).toBe(500);
    expect(input).toHaveFocus();
  });

  it('通知换行后的正文边界以像素同步给固定面板，卸载后释放观察器与变量', () => {
    let rect = { top: 48, bottom: 700, left: 96, right: 1024, height: 652 };
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(
      () => rect as DOMRect,
    );
    const { unmount } = render(
      <MemoryRouter>
        <AppShell title="首页">内容</AppShell>
      </MemoryRouter>,
    );
    const content = document.querySelector('[data-shell-content]');
    const observer = [...resizeObserverInstances].reverse().find((instance) =>
      instance.observe.mock.calls.some(([node]) => node === content),
    )!;
    const root = document.documentElement;
    expect(root.style.getPropertyValue('--shell-content-top')).toBe('48px');
    const chrome = root.style.getPropertyValue('--shell-chrome-bottom');
    rect = { ...rect, top: 144, height: 556 };
    act(() => observer.trigger());
    expect(root.style.getPropertyValue('--shell-content-top')).toBe('144px');
    expect(root.style.getPropertyValue('--shell-content-height')).toBe('556px');
    expect(root.style.getPropertyValue('--shell-chrome-bottom')).toBe(chrome);
    unmount();
    expect(observer.disconnect).toHaveBeenCalledOnce();
    expect(root.style.getPropertyValue('--shell-content-top')).toBe('');
  });

  it('切页后 .content 滚动位置重置到顶部（继承的 scrollTop 被清零）', () => {
    render(
      <MemoryRouter initialEntries={['/a']}>
        <AppShell title="" onBack={undefined}>
          <Routes>
            <Route path="/a" element={<TallPageA />} />
            <Route path="/b" element={<div>内容 B</div>} />
          </Routes>
        </AppShell>
      </MemoryRouter>,
    );
    // AppShell 的 children 由 Routes 提供，找到内容滚动容器
    const main = document.querySelector('main') as HTMLElement;
    expect(main).toBeTruthy();

    // 模拟旧页面留下的滚动位置
    act(() => {
      main.scrollTop = 1234;
    });
    expect(main.scrollTop).toBe(1234);

    // 切页 → useLayoutEffect 应把 scrollTop 重置为 0
    act(() => {
      fireEvent.click(screen.getByText('去 B'));
    });
    expect(main.scrollTop).toBe(0);
  });
});
