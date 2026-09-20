import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { TransferButton } from './TransferButton';

describe('TransferButton', () => {
  it('忙碌期间禁用重复操作，结束后恢复点击', () => {
    const onClick = vi.fn();
    const { rerender } = render(
      <TransferButton busy onClick={onClick}>
        Export
      </TransferButton>,
    );
    const button = screen.getByRole('button', { name: 'Export' });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute('aria-busy', 'true');
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
    rerender(<TransferButton onClick={onClick}>Export</TransferButton>);
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
  });

  it('忙碌结束后仍保留业务禁用条件', () => {
    render(
      <TransferButton disabled onClick={vi.fn()}>
        Import
      </TransferButton>,
    );
    expect(screen.getByRole('button', { name: 'Import' })).toBeDisabled();
  });
});
