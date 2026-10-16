import { createRef } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Input } from './Input';

describe('Input label association', () => {
  it('exposes the visible label as the accessible name without an aria override', () => {
    render(<Input label="Host fingerprint" />);
    expect(screen.getByRole('textbox', { name: 'Host fingerprint' })).toBe(
      screen.getByLabelText('Host fingerprint'),
    );
  });

  it('preserves an explicit id and the caller description', () => {
    render(
      <>
        <span id="host-help">Public connection address</span>
        <Input id="connection-host" label="Host" aria-describedby="host-help" />
      </>,
    );
    const input = screen.getByLabelText('Host');
    expect(input).toHaveAttribute('id', 'connection-host');
    expect(input).toHaveAccessibleDescription('Public connection address');
  });

  it('keeps repeated labels associated with their own distinct inputs', () => {
    render(
      <>
        <Input label="Address" />
        <Input label="Address" />
      </>,
    );
    const fields = screen.getAllByRole('textbox', { name: 'Address' });
    expect(fields).toHaveLength(2);
    expect(new Set(fields.map((field) => field.id)).size).toBe(2);
    expect(fields.every((field) => field.id !== '')).toBe(true);
    const labels = screen.getAllByText('Address').map((text) => text.closest('label'));
    expect(labels.map((label) => label?.control)).toEqual(fields);
  });

  it('updates the accessible name without replacing the input or generated id', () => {
    const { rerender } = render(<Input label="Before" defaultValue="public" />);
    const field = screen.getByLabelText('Before');
    const originalId = field.id;
    rerender(<Input label="After" defaultValue="public" />);
    expect(screen.getByRole('textbox', { name: 'After' })).toBe(field);
    expect(field).toHaveAttribute('id', originalId);
    expect(field).toHaveValue('public');
  });

  it('follows explicit id changes and restores the original generated association', () => {
    const { rerender } = render(<Input label="Field" />);
    const field = screen.getByLabelText('Field');
    const originalId = field.id;
    rerender(<Input label="Field" id="explicit-field" />);
    expect(screen.getByLabelText('Field')).toBe(field);
    expect(field).toHaveAttribute('id', 'explicit-field');
    rerender(<Input label="Field" />);
    expect(screen.getByLabelText('Field')).toBe(field);
    expect(field).toHaveAttribute('id', originalId);
  });

  it('supports adding a visible label while preserving an explicit aria name', () => {
    const { rerender } = render(<Input aria-label="Caller name" />);
    const field = screen.getByRole('textbox', { name: 'Caller name' });
    rerender(<Input aria-label="Caller name" label="Visible name" />);
    expect(screen.getByRole('textbox', { name: 'Caller name' })).toBe(field);
    expect(screen.getByText('Visible name').closest('label')?.control).toBe(field);
    rerender(<Input aria-label="Caller name" />);
    expect(screen.getByRole('textbox', { name: 'Caller name' })).toBe(field);
  });

  it('keeps ref, form name and change behavior on the associated input', () => {
    const ref = createRef<HTMLInputElement>();
    const changed = vi.fn();
    render(
      <Input
        ref={ref}
        label="Host"
        name="connectionHost"
        defaultValue="public"
        onChange={(event) => changed(event.target.value)}
      />,
    );
    const field = screen.getByLabelText('Host');
    expect(ref.current).toBe(field);
    expect(field).toHaveAttribute('name', 'connectionHost');
    fireEvent.change(field, { target: { value: 'new public address' } });
    expect(changed).toHaveBeenCalledTimes(1);
    expect(changed).toHaveBeenCalledWith('new public address');
    ref.current?.focus();
    expect(field).toHaveFocus();
  });

  it('retains externally provided labels when no built-in label is requested', () => {
    render(
      <>
        <label htmlFor="external-field">External name</label>
        <Input id="external-field" />
      </>,
    );
    expect(screen.getByRole('textbox', { name: 'External name' })).toHaveAttribute(
      'id',
      'external-field',
    );
  });
});
