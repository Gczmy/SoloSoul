import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useUiStore } from '@/stores/uiStore';
import { useToastOutletStore } from '@/stores/toastOutletStore';
import { ToastOutlet } from './ToastOutlet';
import { ToastContainer } from './ToastContainer';
import { Dialog } from './Dialog';

const runtime = vi.hoisted(() => ({ android: true }));
vi.mock('@/lib/platform', () => ({ isAndroidSync: () => runtime.android }));

beforeEach(() => {
  runtime.android = true;
  vi.useFakeTimers();
});
afterEach(() => {
  cleanup();
  useUiStore.getState().toasts.forEach((toast) => useUiStore.getState().dismissToast(toast.id));
  expect(useToastOutletStore.getState().outlets).toHaveLength(0);
  vi.useRealTimers();
});

describe('移动端通知宿主交接', () => {
  it('多层弹窗优先于页面，关闭后恢复原提醒且不重置计时', () => {
    function View({ menu, auth }: { menu: boolean; auth: boolean }) {
      return (
        <>
          <div data-testid="page">
            <ToastOutlet />
          </div>
          <Dialog isOpen={menu} onClose={() => {}}>
            <span>菜单</span>
          </Dialog>
          <Dialog isOpen={auth} priority="auth" onClose={() => {}}>
            <span>验证</span>
          </Dialog>
          <ToastContainer />
        </>
      );
    }
    const view = render(<View menu={false} auth={false} />);
    act(() =>
      useUiStore.getState().showToast({ message: '备份提醒', type: 'warning', duration: 8000 }),
    );
    const original = useUiStore.getState().toasts[0];
    expect(screen.getByTestId('page')).toContainElement(screen.getByText('备份提醒'));
    act(() => vi.advanceTimersByTime(2000));
    view.rerender(<View menu auth />);
    expect(screen.getByText('验证').closest('[role="dialog"]')).toContainElement(
      screen.getByText('备份提醒'),
    );
    expect(useUiStore.getState().toasts[0]).toBe(original);
    view.rerender(<View menu auth={false} />);
    expect(screen.getByText('菜单').closest('[role="dialog"]')).toContainElement(
      screen.getByText('备份提醒'),
    );
    view.rerender(<View menu={false} auth={false} />);
    expect(screen.getByTestId('page')).toContainElement(screen.getByText('备份提醒'));
    act(() => vi.advanceTimersByTime(6000));
    expect(screen.queryByText('备份提醒')).not.toBeInTheDocument();
  });

  it('低层级新宿主不会抢占浮层通知；同级后打开者关闭后交还', () => {
    const page = document.createElement('div');
    const first = document.createElement('div');
    const second = document.createElement('div');
    const store = useToastOutletStore.getState();
    const removeFirst = store.register(first, 4000);
    const removePage = store.register(page, 0);
    expect(useToastOutletStore.getState().target).toBe(first);
    const removeSecond = store.register(second, 4000);
    expect(useToastOutletStore.getState().target).toBe(second);
    removeSecond();
    expect(useToastOutletStore.getState().target).toBe(first);
    removeFirst();
    expect(useToastOutletStore.getState().target).toBe(page);
    removePage();
    expect(useToastOutletStore.getState().target).toBeNull();
  });

  it('Toast 操作保持可用，键盘不会重复执行或误关所在对话框', () => {
    const action = vi.fn();
    const close = vi.fn();
    render(
      <>
        <Dialog isOpen onClose={close}>
          内容
        </Dialog>
        <ToastContainer />
      </>,
    );
    act(() =>
      useUiStore.getState().showToast({
        message: '备份提醒',
        type: 'warning',
        action: { label: '去备份', onClick: action },
      }),
    );
    const button = screen.getByRole('button', { name: '去备份' });
    fireEvent.keyDown(button, { key: 'Enter' });
    expect(action).not.toHaveBeenCalled();
    fireEvent.click(button);
    expect(action).toHaveBeenCalledTimes(1);
    expect(close).not.toHaveBeenCalled();
    expect(screen.queryByText('备份提醒')).not.toBeInTheDocument();
  });

  it('桌面不注册占位槽，维持全局通知位置', () => {
    runtime.android = false;
    render(
      <>
        <ToastOutlet />
        <Dialog isOpen onClose={() => {}}>
          内容
        </Dialog>
        <ToastContainer />
      </>,
    );
    act(() => useUiStore.getState().showToast({ message: '桌面提醒', type: 'info' }));
    expect(document.querySelector('[data-toast-outlet]')).toBeNull();
    expect(screen.getByText('内容').closest('[role="dialog"]')).not.toContainElement(
      screen.getByText('桌面提醒'),
    );
  });
});
