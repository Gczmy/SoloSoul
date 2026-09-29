import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { DropdownSelect } from './DropdownSelect';

describe('DropdownSelect', () => {
  const options = [
    { value: 'month', label: 'By month' },
    { value: 'year', label: 'By year' },
    { value: 'archive', label: 'Long archive format', disabled: true },
  ];

  it('marks the selected option, supports keyboard escape and selects only enabled options', () => {
    const onChange = vi.fn();
    render(
      <DropdownSelect
        value="month"
        onChange={onChange}
        options={options}
        triggerLabel="By month"
        ariaLabel="Group by"
      />,
    );
    const trigger = screen.getByRole('button', { name: 'Group by' });
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('button', { name: 'By month' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    const disabled = screen.getByRole('button', { name: 'Long archive format' });
    expect(disabled).toBeDisabled();
    fireEvent.click(disabled);
    expect(onChange).not.toHaveBeenCalled();

    const year = screen.getByRole('button', { name: 'By year' });
    year.focus();
    fireEvent.keyDown(year, { key: 'Escape' });
    expect(trigger).toHaveFocus();
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(trigger);
    fireEvent.click(screen.getByRole('button', { name: 'By year' }));
    expect(onChange).toHaveBeenCalledExactlyOnceWith('year');
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
  });

  it('keeps the trigger inert when disabled', () => {
    render(
      <DropdownSelect
        value="month"
        onChange={vi.fn()}
        options={options}
        triggerLabel="By month"
        disabled
      />,
    );
    const trigger = screen.getByRole('button', { name: 'By month' });
    expect(trigger).toBeDisabled();
    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('button', { name: 'By year' })).not.toBeInTheDocument();
  });
});
