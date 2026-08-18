import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MemoryRouter, Routes, Route, Link } from 'react-router-dom';
import { ShellLayout } from './ShellLayout';
import { PageShell } from './PageShell';
import { useShellConfigStore } from './shellConfigStore';

vi.mock('@/components/layout/AppShell', () => ({
  AppShell: ({ children, title }: { children: React.ReactNode; title: string }) => (
    <div data-testid="mock-app-shell" data-title={title}>
      {children}
    </div>
  ),
}));

vi.mock('@/components/ui/ContentLoadingSkeleton', () => ({
  ContentLoadingSkeleton: () => <div data-testid="content-skeleton" />,
}));

describe('ShellLayout（B1 常驻壳布局）', () => {
  beforeEach(() => {
    useShellConfigStore.setState({ title: '', actions: undefined, onBack: undefined });
  });

  it('渲染常驻壳，并展示当前页面注册的标题', () => {
    render(
      <MemoryRouter initialEntries={['/a']}>
        <Routes>
          <Route element={<ShellLayout />}>
            <Route
              path="/a"
              element={
                <PageShell title="页面A">
                  <div>内容A</div>
                </PageShell>
              }
            />
          </Route>
        </Routes>
      </MemoryRouter>,
    );
    expect(screen.getByTestId('mock-app-shell')).toHaveAttribute('data-title', '页面A');
    expect(screen.getByText('内容A')).toBeTruthy();
  });

  it('切页时壳保持挂载不卸载，仅内容区切换并更新标题', () => {
    render(
      <MemoryRouter initialEntries={['/a']}>
        <Routes>
          <Route element={<ShellLayout />}>
            <Route
              path="/a"
              element={
                <PageShell title="页面A">
                  <Link to="/b">去B</Link>
                </PageShell>
              }
            />
            <Route
              path="/b"
              element={
                <PageShell title="页面B">
                  <div>内容B</div>
                </PageShell>
              }
            />
          </Route>
        </Routes>
      </MemoryRouter>,
    );
    const shell = screen.getByTestId('mock-app-shell');
    fireEvent.click(screen.getByText('去B'));
    // 内容区切换到 B，壳仍是同一个 DOM 实例（未重新挂载）
    expect(screen.getByText('内容B')).toBeTruthy();
    expect(screen.getByTestId('mock-app-shell')).toBe(shell);
    // 标题随新页面注册的配置更新
    expect(screen.getByTestId('mock-app-shell')).toHaveAttribute('data-title', '页面B');
  });
});
