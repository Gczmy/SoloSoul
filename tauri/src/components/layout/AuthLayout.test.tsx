import { act, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthLayout } from './AuthLayout';
import { isAndroidSync } from '@/lib/platform';

const notification = vi.hoisted(() => ({ visible: false }));
vi.mock('./ShellNotifications', () => ({
  ShellNotificationSlot: () => (notification.visible ? <div>发现新版本</div> : null),
}));
vi.mock('@/lib/platform', () => ({ isAndroidSync: vi.fn(() => false) }));

function AuthRoutes() {
  return (
    <MemoryRouter initialEntries={['/login']}>
      <Routes>
        <Route element={<AuthLayout />}>
          <Route path="/login" element={<input aria-label="主密码" type="password" />} />
        </Route>
      </Routes>
    </MemoryRouter>
  );
}

describe('AuthLayout', () => {
  beforeEach(() => {
    notification.visible = false;
    vi.mocked(isAndroidSync).mockReturnValue(false);
  });
  afterEach(() => vi.unstubAllGlobals());

  it('places notifications before the route and preserves an in-progress login when they change', () => {
    const { rerender } = render(<AuthRoutes />);
    const password = screen.getByLabelText('主密码');
    fireEvent.change(password, { target: { value: 'unfinished-password' } });
    password.focus();
    notification.visible = true;
    rerender(<AuthRoutes />);
    const content = password.closest('[data-auth-content]');
    expect(content?.previousElementSibling).toBe(screen.getByText('发现新版本'));
    expect(screen.getByLabelText('主密码')).toBe(password);
    expect(password).toHaveValue('unfinished-password');
    expect(password).toHaveFocus();

    notification.visible = false;
    rerender(<AuthRoutes />);
    expect(screen.queryByText('发现新版本')).not.toBeInTheDocument();
    expect(screen.getByLabelText('主密码')).toBe(password);
    expect(password).toHaveValue('unfinished-password');
  });

  it('resizes the Android auth area to the visible viewport and releases its listeners on unmount', () => {
    vi.mocked(isAndroidSync).mockReturnValue(true);
    const viewport = new EventTarget();
    Object.defineProperty(viewport, 'height', { value: 640, configurable: true });
    const removeListener = vi.spyOn(viewport, 'removeEventListener');
    vi.stubGlobal('visualViewport', viewport);
    const { container, unmount } = render(<AuthRoutes />);
    const layout = container.querySelector<HTMLElement>('[data-auth-layout]')!;
    expect(layout.style.getPropertyValue('--auth-viewport-height')).toBe('640px');

    Object.defineProperty(viewport, 'height', { value: 320 });
    act(() => viewport.dispatchEvent(new Event('resize')));
    expect(layout.style.getPropertyValue('--auth-viewport-height')).toBe('320px');
    unmount();
    expect(removeListener).toHaveBeenCalledWith('resize', expect.any(Function));
    expect(removeListener).toHaveBeenCalledWith('scroll', expect.any(Function));
    expect(layout.style.getPropertyValue('--auth-viewport-height')).toBe('');
  });
});
