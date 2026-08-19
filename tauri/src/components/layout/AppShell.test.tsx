import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, fireEvent, act, screen } from '@testing-library/react';
import { MemoryRouter, Routes, Route, Link } from 'react-router-dom';
import { AppShell } from './AppShell';

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
  beforeEach(() => {
    // 全局 reset：jsdom 的 scrollTop 读写存在但无实际布局，直接断言调用即可。
    // 用 spy 确认 useLayoutEffect 在导航后设置了 scrollTop=0。
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
