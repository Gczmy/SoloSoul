import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { PageShell } from './PageShell';
import { useShellConfigStore } from './shellConfigStore';

describe('PageShell（B1 壳配置桥）', () => {
  beforeEach(() => {
    useShellConfigStore.setState({ title: '', actions: undefined, onBack: undefined });
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
    // 相同配置重渲染：setConfig 跳过更新，不重复通知
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
