import { useState } from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import type { PropertyType } from '@/types/template';
import { DynamicGroupEditor, type DynamicGroupItem } from './DynamicGroupEditor';

function ControlledEditor({
  initialItems,
  allowedTypes,
  maxItems,
}: {
  initialItems: DynamicGroupItem[];
  allowedTypes?: PropertyType[];
  maxItems?: number;
}) {
  const [value, setValue] = useState(initialItems);

  return (
    <DynamicGroupEditor
      propertyId="contactMethods"
      label="联系方式"
      value={value}
      allowedTypes={allowedTypes}
      maxItems={maxItems}
      onChange={setValue}
    />
  );
}

describe('DynamicGroupEditor', () => {
  it('renders empty group with add button', () => {
    const onChange = vi.fn();
    render(
      <DynamicGroupEditor
        propertyId="contactMethods"
        label="联系方式"
        value={[]}
        onChange={onChange}
      />,
    );
    expect(screen.getByText('联系方式')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /添加字段/i })).toBeInTheDocument();
  });

  it('renders existing sub-fields', () => {
    const onChange = vi.fn();
    render(
      <DynamicGroupEditor
        propertyId="contactMethods"
        label="联系方式"
        value={[
          { id: '1', name: '手机', type: 'phone', value: '13800138000' },
          { id: '2', name: '邮箱', type: 'email', value: 'a@b.com' },
        ]}
        onChange={onChange}
      />,
    );
    expect(screen.getByText('手机')).toBeInTheDocument();
    expect(screen.getByDisplayValue('13800138000')).toBeInTheDocument();
    expect(screen.getByText('邮箱')).toBeInTheDocument();
    expect(screen.getByDisplayValue('a@b.com')).toBeInTheDocument();
  });

  it('removes a sub-field when delete clicked', () => {
    const onChange = vi.fn();
    render(
      <DynamicGroupEditor
        propertyId="contactMethods"
        label="联系方式"
        value={[{ id: '1', name: '手机', type: 'phone', value: '13800138000' }]}
        onChange={onChange}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'common:delete' }));
    expect(onChange).toHaveBeenCalledWith([]);
  });

  it('updates sub-field value on input change', () => {
    const onChange = vi.fn();
    render(
      <DynamicGroupEditor
        propertyId="contactMethods"
        label="联系方式"
        value={[{ id: '1', name: '手机', type: 'phone', value: '13800138000' }]}
        onChange={onChange}
      />,
    );
    const input = screen.getByDisplayValue('13800138000');
    fireEvent.change(input, { target: { value: '13900139000' } });
    expect(onChange).toHaveBeenCalledWith([
      { id: '1', name: '手机', type: 'phone', value: '13900139000' },
    ]);
  });

  it('hides add button when max items reached', () => {
    const onChange = vi.fn();
    render(
      <DynamicGroupEditor
        propertyId="contactMethods"
        label="联系方式"
        value={[{ id: '1', name: '手机', type: 'phone', value: '13800138000' }]}
        maxItems={1}
        onChange={onChange}
      />,
    );
    expect(screen.queryByRole('button', { name: /添加字段/i })).not.toBeInTheDocument();
  });

  it('adds an allowed numeric field with its default and stops at the item limit', () => {
    render(<ControlledEditor initialItems={[]} allowedTypes={['number']} maxItems={1} />);

    fireEvent.click(screen.getByRole('button', { name: '添加字段' }));

    expect(screen.getByText('1/1')).toBeInTheDocument();
    expect(screen.getByRole('combobox')).toHaveValue('number');
    expect(screen.getByDisplayValue('0')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '添加字段' })).toBeNull();
  });

  it('moves and renames a field while preserving its value', () => {
    render(
      <ControlledEditor
        initialItems={[
          { id: 'first', name: 'First', type: 'text', value: 'first-value' },
          { id: 'second', name: 'Second', type: 'text', value: 'second-value' },
        ]}
      />,
    );

    fireEvent.click(screen.getAllByRole('button', { name: 'common:move_down' })[0]);
    expect(
      screen
        .getAllByRole('button', { name: /^(First|Second)$/ })
        .map((button) => button.textContent),
    ).toEqual(['Second', 'First']);

    fireEvent.click(screen.getByRole('button', { name: 'First' }));
    const nameInput = screen.getByDisplayValue('First');
    fireEvent.change(nameInput, { target: { value: 'Renamed' } });
    fireEvent.keyDown(nameInput, { key: 'Enter' });

    expect(screen.getByRole('button', { name: 'Renamed' })).toBeInTheDocument();
    expect(screen.getByDisplayValue('first-value')).toBeInTheDocument();
  });

  it('clears the previous value when a field changes type', () => {
    render(
      <ControlledEditor
        initialItems={[{ id: 'first', name: 'First', type: 'text', value: 'old-secret' }]}
        allowedTypes={['text', 'boolean']}
      />,
    );

    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'boolean' } });

    expect(screen.queryByDisplayValue('old-secret')).toBeNull();
    expect(screen.getByRole('combobox')).toHaveValue('boolean');
    expect(screen.getByRole('checkbox')).not.toBeChecked();
  });
});
