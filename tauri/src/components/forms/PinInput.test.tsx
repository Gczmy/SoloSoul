import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { PinInput } from './PinInput';

describe('RF-923 PIN completion ownership', () => {
  it('does not submit the same full PIN again for an extra global digit', () => {
    const onComplete = vi.fn();
    render(<PinInput length={6} onComplete={onComplete} />);
    const input = screen.getByRole('textbox', { name: 'PIN 输入' });
    fireEvent.change(input, { target: { value: '123456' } });
    expect(onComplete).toHaveBeenCalledExactlyOnceWith('123456');

    input.blur();
    expect(document.activeElement).not.toBe(input);
    fireEvent.keyDown(document, { key: '7' });
    expect(onComplete).toHaveBeenCalledExactlyOnceWith('123456');
  });

  it('does not resubmit an unchanged full PIN through the input change path', () => {
    const onComplete = vi.fn();
    render(<PinInput length={6} onComplete={onComplete} />);
    const input = screen.getByRole('textbox', { name: 'PIN 输入' });
    fireEvent.change(input, { target: { value: '123456' } });
    fireEvent.change(input, { target: { value: '1234567' } });
    expect(onComplete).toHaveBeenCalledExactlyOnceWith('123456');
  });

  it('still accepts Enter for a legacy short PIN and a later edited PIN', () => {
    const onComplete = vi.fn();
    render(<PinInput length={6} onComplete={onComplete} />);
    const input = screen.getByRole('textbox', { name: 'PIN 输入' });
    fireEvent.change(input, { target: { value: '1234' } });
    input.blur();
    expect(document.activeElement).not.toBe(input);
    fireEvent.keyDown(document, { key: 'Enter' });
    expect(onComplete).toHaveBeenCalledExactlyOnceWith('1234');

    fireEvent.change(input, { target: { value: '123456' } });
    expect(onComplete).toHaveBeenCalledWith('123456');
  });
});
