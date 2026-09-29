import { fireEvent, render, screen } from '@testing-library/react';
import { Trash2 } from 'lucide-react';
import { describe, expect, it, vi } from 'vitest';
import { BadgeIconButton } from './BadgeIconButton';

describe('BadgeIconButton', () => {
  it('exposes the count, tooltip and toggle state without duplicate clicks', () => {
    const onClick = vi.fn();
    const { rerender } = render(
      <BadgeIconButton Icon={Trash2} title="Remove" count={3} onClick={onClick} pressed />,
    );
    const button = screen.getByRole('button', { name: 'Remove (3)' });
    expect(button).toHaveAttribute('title', 'Remove');
    expect(button).toHaveAttribute('aria-pressed', 'true');
    expect(button).toHaveAttribute('data-ui-icon-intent', 'neutral');
    expect(screen.getByTestId('count-badge-remove')).toHaveTextContent('3');
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();

    rerender(
      <BadgeIconButton Icon={Trash2} title="Remove" onClick={onClick} dangerOutline disabled />,
    );
    expect(button).toHaveAttribute('data-ui-icon-intent', 'danger-soft');
    expect(button).not.toHaveAttribute('aria-pressed');
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
  });
});
