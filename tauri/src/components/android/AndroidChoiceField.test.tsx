import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { AndroidChoiceField } from './AndroidChoiceField';

const choices = [
  { value: 'a', label: 'Alpha', description: 'First' },
  { value: 'b', label: 'Beta' },
  { value: 'c', label: 'Disabled', disabled: true },
];
describe('AndroidChoiceField', () => {
  it('chooses once without submitting its parent form', () => {
    const onChange = vi.fn();
    const submit = vi.fn();
    render(
      <form onSubmit={submit}>
        <AndroidChoiceField title="Choice" choices={choices} value="a" onChange={onChange} />
      </form>,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Choice' }));
    expect(screen.getByRole('radio', { name: /Alpha.*First/ })).toHaveAttribute(
      'aria-checked',
      'true',
    );
    fireEvent.click(screen.getByRole('radio', { name: 'Beta' }));
    expect(onChange).toHaveBeenCalledExactlyOnceWith('b');
    expect(submit).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
  it('cancels a multi-select draft and commits in option order', () => {
    const onChange = vi.fn();
    render(
      <AndroidChoiceField
        multiple
        title="Choice"
        choices={choices}
        value={['b']}
        onChange={onChange}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Choice' }));
    fireEvent.click(screen.getByRole('checkbox', { name: /Alpha.*First/ }));
    fireEvent.click(screen.getByRole('button', { name: 'cancel' }));
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Choice' }));
    expect(screen.getByRole('checkbox', { name: /Alpha.*First/ })).toHaveAttribute(
      'aria-checked',
      'false',
    );
    fireEvent.click(screen.getByRole('checkbox', { name: /Alpha.*First/ }));
    fireEvent.click(screen.getByRole('button', { name: 'confirm' }));
    expect(onChange).toHaveBeenCalledExactlyOnceWith(['a', 'b']);
  });
  it('does not choose disabled options or open a disabled field', () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <AndroidChoiceField title="Choice" choices={choices} value="" onChange={onChange} />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Choice' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Disabled' }));
    expect(onChange).not.toHaveBeenCalled();
    rerender(
      <AndroidChoiceField title="Choice" choices={choices} value="" onChange={onChange} disabled />,
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Choice' })).toBeDisabled();
  });
  it('preserves unknown stored values and escapes without a write', () => {
    const onChange = vi.fn();
    render(
      <AndroidChoiceField title="Choice" choices={choices} value="old-value" onChange={onChange} />,
    );
    expect(screen.getByRole('button', { name: 'Choice' })).toHaveTextContent('old-value');
    fireEvent.click(screen.getByRole('button', { name: 'Choice' }));
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(onChange).not.toHaveBeenCalled();
  });
});
