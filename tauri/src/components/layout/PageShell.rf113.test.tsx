import { useState, type ReactNode } from 'react';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Link, MemoryRouter, Route, Routes, useLocation, useParams } from 'react-router-dom';
import { ErrorBoundary } from '@/components/ui/ErrorBoundary';
import { useAuthStore } from '@/stores/authStore';
import { PageShell } from './PageShell';
import { ShellLayout } from './ShellLayout';
import { useShellConfigStore } from './shellConfigStore';

// 保留真实壳、顶栏、配置 Store 与认证 Store；仅省去无关导航和配对 UI。
// 原生 IPC / 事件使用 test/setup.ts 的边界替身，不替换注册、注销或会话逻辑。
vi.mock('./SideNavigation', () => ({ SideNavigation: () => <nav /> }));
vi.mock('./TopFunctionBar', () => ({ TopFunctionBar: () => <nav /> }));
vi.mock('./MobileBottomNav', () => ({ MobileBottomNav: () => <nav /> }));
vi.mock('@/components/android/AndroidNavigation', () => ({
  AndroidNavigation: () => <nav />,
}));
vi.mock('@/components/sync/PairingDialog', () => ({ PairingDialog: () => null }));

const ACCOUNT_A = { id: 'rf113-account-a', name: 'RF113 A' };
const ACCOUNT_B = { id: 'rf113-account-b', name: 'RF113 B' };

function configuredPage(label: string) {
  const action = vi.fn();
  const primaryAction = vi.fn();
  const onBack = vi.fn();
  return {
    label,
    action,
    primaryAction,
    props: {
      title: `${label} 标题`,
      actions: <button onClick={action}>{label} 次要操作</button>,
      primaryActions: <button onClick={primaryAction}>{label} 主操作</button>,
      onBack,
    },
  };
}

function renderShell(routes: ReactNode, initialEntry = '/a') {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <Routes>
        <Route element={<ShellLayout />}>{routes}</Route>
      </Routes>
    </MemoryRouter>,
  );
}

function shellElement() {
  const shell = document.querySelector('[data-navigation]');
  expect(shell).not.toBeNull();
  return shell;
}

function expectPageConfig(page: ReturnType<typeof configuredPage>) {
  expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(page.props.title);
  expect(screen.getByRole('button', { name: `${page.label} 次要操作` })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: `${page.label} 主操作` })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'back' })).toBeInTheDocument();
  expect(useShellConfigStore.getState()).toMatchObject(page.props);
}

function expectEmptyConfig() {
  expect(useShellConfigStore.getState()).toMatchObject({
    title: '',
    actions: undefined,
    primaryActions: undefined,
    onBack: undefined,
  });
}

function expectEmptyHeader() {
  expectEmptyConfig();
  expect(screen.getByRole('heading', { level: 1 })).toBeEmptyDOMElement();
  expect(document.querySelector('[data-appbar] button')).toBeNull();
}

function clickPageActions(page: ReturnType<typeof configuredPage>) {
  fireEvent.click(screen.getByRole('button', { name: `${page.label} 次要操作` }));
  fireEvent.click(screen.getByRole('button', { name: `${page.label} 主操作` }));
  fireEvent.click(screen.getByRole('button', { name: 'back' }));
  expect(page.action).toHaveBeenCalledOnce();
  expect(page.primaryAction).toHaveBeenCalledOnce();
  expect(page.props.onBack).toHaveBeenCalledOnce();
}

function ThrowingPage(): ReactNode {
  throw new Error('RF113 synthetic render failure');
}

describe('RF113 常驻壳配置的页面所有权', () => {
  beforeEach(() => {
    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
    useAuthStore.getState().completeUnlock(ACCOUNT_A, [ACCOUNT_A, ACCOUNT_B]);
    useShellConfigStore.setState({
      title: '',
      actions: undefined,
      primaryActions: undefined,
      onBack: undefined,
    });
    // 给真实 ToolbarActions 足够的桌面宽度，不以折叠菜单掩盖旧操作残留。
    vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(1280);
  });

  afterEach(() => {
    cleanup();
    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
    useShellConfigStore.setState({
      title: '',
      actions: undefined,
      primaryActions: undefined,
      onBack: undefined,
    });
    vi.restoreAllMocks();
  });

  it('完整 A 页面导航到抛错 B 后清空四类配置，错误边界保留同一个壳', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const pageA = configuredPage('A');
    renderShell(
      <>
        <Route
          path="/a"
          element={
            <ErrorBoundary key="a" label="RF113 A">
              <PageShell {...pageA.props}>
                <Link to="/b">前往出错页面</Link>
              </PageShell>
            </ErrorBoundary>
          }
        />
        <Route
          path="/b"
          element={
            <ErrorBoundary key="b" label="RF113 B">
              <ThrowingPage />
            </ErrorBoundary>
          }
        />
      </>,
    );
    expectPageConfig(pageA);
    const shell = shellElement();
    const appbar = document.querySelector('[data-appbar]');
    const content = document.querySelector('[data-shell-content]');

    fireEvent.click(screen.getByRole('link', { name: '前往出错页面' }));

    expect(screen.getByText('RF113 synthetic render failure')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '重试' })).toBeInTheDocument();
    expect(shellElement()).toBe(shell);
    expect(document.querySelector('[data-appbar]')).toBe(appbar);
    expect(document.querySelector('[data-shell-content]')).toBe(content);
    expectEmptyHeader();
    expect(screen.queryByText('A 标题')).not.toBeInTheDocument();
    expect(screen.queryByText('A 次要操作')).not.toBeInTheDocument();
    expect(screen.queryByText('A 主操作')).not.toBeInTheDocument();
    expect(pageA.action).not.toHaveBeenCalled();
    expect(pageA.primaryAction).not.toHaveBeenCalled();
    expect(pageA.props.onBack).not.toHaveBeenCalled();
  });

  it('当前页面卸载后不在配置 Store 中保留标题、ReactNode 或返回闭包', () => {
    const page = configuredPage('将卸载页面');
    const view = renderShell(
      <Route path="/a" element={<PageShell {...page.props}>页面正文</PageShell>} />,
    );
    expectPageConfig(page);

    view.unmount();

    expectEmptyConfig();
    expect(document.querySelector('[data-appbar]')).toBeNull();
  });

  it.each(['锁定保留账户 ID', '同账户重新登录', '切换账户'] as const)(
    '%s 清空旧配置；旧页面新 props 不能续用失效会话，新挂载页面可以注册',
    (transition) => {
      const initial = configuredPage('原会话页面');
      const staleOne = configuredPage('旧页面第一次更新');
      const staleTwo = configuredPage('旧页面第二次更新');
      const fresh = configuredPage('新挂载页面');
      const oldConfigurations = [initial, staleOne, staleTwo];
      function SessionPage() {
        const [revision, setRevision] = useState(0);
        const [remounted, setRemounted] = useState(false);
        return (
          <>
            <button onClick={() => setRevision((value) => value + 1)}>更新旧页面</button>
            <button onClick={() => setRemounted(true)}>重新进入页面</button>
            <PageShell
              key={remounted ? 'fresh-page' : 'original-page'}
              {...(remounted ? fresh : oldConfigurations[revision]).props}
            >
              <div>会话页面正文</div>
            </PageShell>
          </>
        );
      }
      renderShell(<Route path="/a" element={<SessionPage />} />);
      expectPageConfig(initial);
      const shell = shellElement();
      const originalContent = screen.getByText('会话页面正文');

      if (transition === '切换账户') {
        act(() => useAuthStore.getState().completeUnlock(ACCOUNT_B));
      } else {
        act(() => useAuthStore.setState({ isAuthenticated: false }));
        expect(useAuthStore.getState().currentAccount?.id).toBe(ACCOUNT_A.id);
        if (transition === '同账户重新登录') {
          act(() => useAuthStore.getState().completeUnlock(ACCOUNT_A));
        }
      }
      expectEmptyHeader();
      fireEvent.click(screen.getByRole('button', { name: '更新旧页面' }));
      expect(screen.getByText('会话页面正文')).toBe(originalContent);
      expectEmptyHeader();

      if (transition === '锁定保留账户 ID') {
        act(() => useAuthStore.getState().completeUnlock(ACCOUNT_A));
      }
      fireEvent.click(screen.getByRole('button', { name: '更新旧页面' }));
      expectEmptyHeader();
      expect(shellElement()).toBe(shell);
      for (const page of oldConfigurations) {
        expect(page.action).not.toHaveBeenCalled();
        expect(page.primaryAction).not.toHaveBeenCalled();
        expect(page.props.onBack).not.toHaveBeenCalled();
      }

      fireEvent.click(screen.getByRole('button', { name: '重新进入页面' }));
      expectPageConfig(fresh);
      expect(shellElement()).toBe(shell);
      clickPageActions(fresh);
    },
  );

  it('A 注册、B 注册后再卸载 A，不清除 B 的标题、两类操作或返回闭包', () => {
    const pageA = configuredPage('A');
    const pageB = configuredPage('B');
    function OverlappingPages() {
      const [showA, setShowA] = useState(true);
      const [showB, setShowB] = useState(false);
      return (
        <>
          <button onClick={() => setShowB(true)}>打开 B</button>
          <button onClick={() => setShowA(false)}>关闭 A</button>
          {showA && (
            <PageShell key="a" {...pageA.props}>
              A 正文
            </PageShell>
          )}
          {showB && (
            <PageShell key="b" {...pageB.props}>
              B 正文
            </PageShell>
          )}
        </>
      );
    }
    renderShell(<Route path="/a" element={<OverlappingPages />} />);
    expectPageConfig(pageA);
    const shell = shellElement();

    fireEvent.click(screen.getByRole('button', { name: '打开 B' }));
    expectPageConfig(pageB);
    fireEvent.click(screen.getByRole('button', { name: '关闭 A' }));

    expect(screen.queryByText('A 正文')).not.toBeInTheDocument();
    expect(screen.getByText('B 正文')).toBeInTheDocument();
    expectPageConfig(pageB);
    expect(shellElement()).toBe(shell);
    clickPageActions(pageB);
    expect(pageA.action).not.toHaveBeenCalled();
    expect(pageA.primaryAction).not.toHaveBeenCalled();
    expect(pageA.props.onBack).not.toHaveBeenCalled();
  });

  it('同一路由组件更新对象参数与 query 后使用新配置和回调，壳保持同实例', () => {
    const invoked = vi.fn();
    function ItemPage() {
      const { id } = useParams();
      const { search } = useLocation();
      const view = new URLSearchParams(search).get('view');
      const label = `${id}/${view}`;
      return (
        <PageShell
          title={label}
          actions={<button onClick={() => invoked('action', id, view)}>操作 {label}</button>}
          primaryActions={
            <button onClick={() => invoked('primary', id, view)}>主操作 {label}</button>
          }
          onBack={() => invoked('back', id, view)}
        >
          <Link to="/item/two?view=detail">切换对象和视图</Link>
          <Link to="/item/two?view=audit">仅切换视图</Link>
        </PageShell>
      );
    }
    renderShell(<Route path="/item/:id" element={<ItemPage />} />, '/item/one?view=summary');
    const shell = shellElement();
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('one/summary');

    for (const [link, view, previous] of [
      ['切换对象和视图', 'detail', 'one/summary'],
      ['仅切换视图', 'audit', 'two/detail'],
    ]) {
      invoked.mockClear();
      fireEvent.click(screen.getByRole('link', { name: link }));
      expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(`two/${view}`);
      expect(screen.queryByRole('button', { name: `操作 ${previous}` })).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: `主操作 ${previous}` })).not.toBeInTheDocument();
      fireEvent.click(screen.getByRole('button', { name: `操作 two/${view}` }));
      fireEvent.click(screen.getByRole('button', { name: `主操作 two/${view}` }));
      fireEvent.click(screen.getByRole('button', { name: 'back' }));
      expect(invoked.mock.calls).toEqual([
        ['action', 'two', view],
        ['primary', 'two', view],
        ['back', 'two', view],
      ]);
      expect(useShellConfigStore.getState().title).toBe(`two/${view}`);
      expect(shellElement()).toBe(shell);
    }
  });
});
