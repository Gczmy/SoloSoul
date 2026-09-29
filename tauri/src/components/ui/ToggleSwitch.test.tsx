import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ToggleSwitch } from './ToggleSwitch';

describe('ToggleSwitch', () => {
  it('reports one boolean change for the input and label, and ignores disabled clicks', () => {
    const onChange = vi.fn();
    const view = render(
      <ToggleSwitch checked={false} onChange={onChange} ariaLabel="Automatic Sync" />,
    );
    const input = screen.getByRole('switch', { name: 'Automatic Sync' });
    fireEvent.click(input);
    expect(onChange).toHaveBeenCalledExactlyOnceWith(true);

    onChange.mockClear();
    view.rerender(<ToggleSwitch checked onChange={onChange} ariaLabel="Automatic Sync" />);
    fireEvent.click(input.closest('label')!);
    expect(onChange).toHaveBeenCalledExactlyOnceWith(false);

    onChange.mockClear();
    view.rerender(<ToggleSwitch checked disabled onChange={onChange} ariaLabel="Automatic Sync" />);
    fireEvent.click(input.closest('label')!);
    expect(input).toBeDisabled();
    expect(onChange).not.toHaveBeenCalled();
  });
});
