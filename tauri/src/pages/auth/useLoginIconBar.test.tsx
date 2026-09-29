import { useState } from 'react';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { LoginIconBar } from './LoginIconBar';
import { useLoginIconBar } from './useLoginIconBar';
import type { LoginMethod } from './useLoginPage';

const hoverSupport = vi.hoisted(() => ({ enabled: true }));
vi.mock('@/lib/platform', () => ({ supportsHover: () => hoverSupport.enabled }));

function Harness({
  bioAvailable = false,
  biometryTypeRaw = '',
  pinAvailable = false,
  onSelectMethod,
}: {
  bioAvailable?: boolean;
  biometryTypeRaw?: string;
  pinAvailable?: boolean;
  onSelectMethod: (method: LoginMethod) => void;
}) {
  const [loginMethod, setLoginMethod] = useState<LoginMethod>('password');
  const bar = useLoginIconBar({
    bioAvailable,
    biometryTypeRaw,
    pinAvailable,
    onSelectMethod: (method) => {
      setLoginMethod(method);
      onSelectMethod(method);
    },
  });
  return (
    <>
      <LoginIconBar
        loginMethod={loginMethod}
        iconMethods={bar.iconMethods}
        hoveredIcon={bar.hoveredIcon}
        committedIcon={bar.committedIcon}
        onIconEnter={bar.handleIconEnter}
        onIconLeave={bar.handleIconLeave}
        onIconClick={bar.handleIconClick}
      />
      <output data-testid="selected-method">{loginMethod}</output>
      <output data-testid="hovered-icon">{bar.hoveredIcon}</output>
      <output data-testid="committed-icon">{bar.committedIcon}</output>
    </>
  );
}

describe('login method icon bar interactions', () => {
  afterEach(() => {
    vi.useRealTimers();
    hoverSupport.enabled = true;
  });

  it('only offers available methods and selects password, biometrics and PIN', () => {
    const onSelectMethod = vi.fn();
    const { rerender } = render(<Harness onSelectMethod={onSelectMethod} />);
    expect(screen.getAllByRole('button')).toHaveLength(1);
    fireEvent.click(screen.getAllByRole('button')[0]);
    expect(onSelectMethod).toHaveBeenLastCalledWith('password');

    for (const biometryTypeRaw of ['faceId', 'touchId', 'windowsHello']) {
      rerender(
        <Harness
          bioAvailable
          biometryTypeRaw={biometryTypeRaw}
          pinAvailable
          onSelectMethod={onSelectMethod}
        />,
      );
      const buttons = screen.getAllByRole('button');
      expect(buttons).toHaveLength(3);
      fireEvent.click(buttons[1]);
      expect(screen.getByTestId('selected-method')).toHaveTextContent(biometryTypeRaw);
      expect(onSelectMethod).toHaveBeenLastCalledWith(biometryTypeRaw);
      fireEvent.click(buttons[2]);
      expect(onSelectMethod).toHaveBeenLastCalledWith('pin');
    }
  });

  it('delays label expansion and cancels it on leave or method selection', () => {
    vi.useFakeTimers();
    const onSelectMethod = vi.fn();
    render(<Harness pinAvailable onSelectMethod={onSelectMethod} />);
    const buttons = screen.getAllByRole('button');

    fireEvent.mouseEnter(buttons[1]);
    expect(screen.getByTestId('hovered-icon')).toHaveTextContent('pin');
    expect(screen.getByTestId('committed-icon')).toBeEmptyDOMElement();
    act(() => vi.advanceTimersByTime(299));
    expect(screen.getByTestId('committed-icon')).toBeEmptyDOMElement();
    act(() => vi.advanceTimersByTime(1));
    expect(screen.getByTestId('committed-icon')).toHaveTextContent('pin');

    fireEvent.mouseLeave(buttons[1]);
    expect(screen.getByTestId('hovered-icon')).toBeEmptyDOMElement();
    expect(screen.getByTestId('committed-icon')).toBeEmptyDOMElement();
    fireEvent.mouseEnter(buttons[1]);
    fireEvent.click(buttons[1]);
    expect(onSelectMethod).toHaveBeenLastCalledWith('pin');
    act(() => vi.advanceTimersByTime(300));
    expect(screen.getByTestId('committed-icon')).toBeEmptyDOMElement();
  });

  it('does not stick in hover state on touch devices and cleans up pending timers', () => {
    vi.useFakeTimers();
    hoverSupport.enabled = false;
    const { rerender, unmount } = render(<Harness onSelectMethod={vi.fn()} />);
    fireEvent.mouseEnter(screen.getByRole('button'));
    expect(screen.getByTestId('hovered-icon')).toBeEmptyDOMElement();
    expect(vi.getTimerCount()).toBe(0);

    hoverSupport.enabled = true;
    rerender(<Harness onSelectMethod={vi.fn()} />);
    fireEvent.mouseEnter(screen.getByRole('button'));
    expect(vi.getTimerCount()).toBe(1);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});
