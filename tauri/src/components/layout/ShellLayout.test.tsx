import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import { MemoryRouter, Routes, Route, Link, useParams } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { ObjectInspectorPanel } from '@/components/object/ObjectInspectorPanel';
import { useMemo } from 'react';
import { useUiStore, type ObjectInspectorTab } from '@/stores/uiStore';
import { ShellLayout } from './ShellLayout';
import { PageShell } from './PageShell';
import {
  EMPTY_SHELL_CONFIG,
  useShellConfigStore,
  type ObjectInspectorContext,
} from './shellConfigStore';

vi.mock('@/components/layout/AppShell', () => ({
  AppShell: ({
    children,
    title,
    objectInspector,
  }: {
    children: React.ReactNode;
    title: string;
    objectInspector?: ObjectInspectorContext;
  }) => (
    <div data-testid="mock-app-shell" data-title={title}>
      {objectInspector && <ObjectInspectorPanel context={objectInspector} onClose={() => {}} />}
      {children}
    </div>
  ),
}));

vi.mock('@/hooks/useWorkspacePasswordGuard', () => ({
  useWorkspacePasswordGuard: () => ({
    showPwDialog: false,
    bioAvailable: { available: false },
    passwordHint: null,
    passwordVerify: vi.fn(),
  }),
}));
vi.mock('@/components/forms/PasswordVerificationDialog', () => ({
  PasswordVerificationDialog: () => null,
}));

vi.mock('@/components/ui/ContentLoadingSkeleton', () => ({
  ContentLoadingSkeleton: () => <div data-testid="content-skeleton" />,
}));

describe('ShellLayout（B1 常驻壳布局）', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    useShellConfigStore.setState({
      ...EMPTY_SHELL_CONFIG,
      registration: null,
    });
  });

  it.each<ObjectInspectorTab>(['history', 'attachments'])(
    'reselecting the current object keeps the %s list mounted without another request',
    async (tab) => {
      vi.mocked(invoke).mockResolvedValue([]);
      useUiStore.setState({ desktopObjectInspectorTab: tab });
      function Editor() {
        const { objectId } = useParams();
        const objectInspector = useMemo(
          () => ({ objectId: objectId!, objectName: 'Current object', ready: true }),
          [objectId],
        );
        return (
          <PageShell
            title="Editor"
            continuityScope="/editor/:objectId"
            objectInspector={objectInspector}
          >
            <Link to={`/editor/${objectId}`}>Reselect object</Link>
          </PageShell>
        );
      }
      render(
        <MemoryRouter initialEntries={['/editor/one']}>
          <Routes>
            <Route element={<ShellLayout />}>
              <Route path="/editor/:objectId" element={<Editor />} />
            </Route>
          </Routes>
        </MemoryRouter>,
      );
      await waitFor(() =>
        expect(screen.getByRole('tabpanel')).toHaveAttribute('aria-busy', 'false'),
      );
      const list = screen.getByRole('tabpanel');
      list.scrollTop = 120;
      const calls = vi.mocked(invoke).mock.calls.length;
      for (let click = 0; click < 3; click++) {
        fireEvent.click(screen.getByRole('link', { name: 'Reselect object' }));
      }
      await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());
      expect(screen.getByRole('tabpanel')).toBe(list);
      expect(list.scrollTop).toBe(120);
      expect(vi.mocked(invoke)).toHaveBeenCalledTimes(calls);
    },
  );

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
