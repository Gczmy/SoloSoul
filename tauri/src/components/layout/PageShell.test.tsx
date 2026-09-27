import { describe, it, expect, beforeEach } from 'vitest';
import { render as renderComponent, screen } from '@testing-library/react';
import { StrictMode, type ReactElement } from 'react';
import { MemoryRouter } from 'react-router-dom';
import { PageShell } from './PageShell';
import { useShellConfigStore } from './shellConfigStore';

function render(ui: ReactElement) {
  return renderComponent(ui, { wrapper: MemoryRouter });
}

describe('PageShell（B1 壳配置桥）', () => {
  beforeEach(() => {
    useShellConfigStore.setState({
      title: '',
      actions: undefined,
      primaryActions: undefined,
      onBack: undefined,
    });
  });

  it('将 title/actions/onBack 注册到壳配置 store 并渲染 children', () => {
    const onBack = () => {};
    const actions = <button type="button">动作</button>;
    render(
      <PageShell title="设置" actions={actions} onBack={onBack}>
        <div>页面内容</div>
      </PageShell>,
    );
    const s = useShellConfigStore.getState();
    expect(s.title).toBe('设置');
    expect(s.onBack).toBe(onBack);
    // 页面内容照常渲染（壳在布局层，children 渲染进内容区）
    expect(screen.getByText('页面内容')).toBeTruthy();
  });

  it('StrictMode 的建立、清理、重建后配置仍有效，最终卸载释放全部节点与回调', () => {
    const onBack = () => {};
    const actions = <button>Strict actions</button>;
    const primaryActions = <button>Strict primary</button>;
    const { unmount } = render(
      <StrictMode>
        <PageShell
          title="Strict title"
          actions={actions}
          primaryActions={primaryActions}
          onBack={onBack}
        >
          <div>Strict content</div>
        </PageShell>
      </StrictMode>,
    );
    expect(useShellConfigStore.getState()).toMatchObject({
      title: 'Strict title',
      actions,
      primaryActions,
      onBack,
    });
    expect(screen.getByText('Strict content')).toBeVisible();
    unmount();
    expect(useShellConfigStore.getState()).toMatchObject({
      title: '',
      actions: undefined,
      primaryActions: undefined,
      onBack: undefined,
    });
  });

  it('配置不变时不重复通知订阅者（避免页面每次重渲染都触发壳重渲染）', () => {
    let notifies = 0;
    const unsub = useShellConfigStore.subscribe(() => {
      notifies += 1;
    });
    const { rerender } = render(
      <PageShell title="A">
        <div />
      </PageShell>,
    );
    expect(notifies).toBe(1);
    // 相同配置重渲染：跳过更新，不重复通知
    rerender(
      <PageShell title="A">
        <div />
      </PageShell>,
    );
    expect(notifies).toBe(1);
    // 标题变化才通知
    rerender(
      <PageShell title="B">
        <div />
      </PageShell>,
    );
    expect(notifies).toBe(2);
    unsub();
  });
});
