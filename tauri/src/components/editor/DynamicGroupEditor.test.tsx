import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { DynamicGroupEditor } from './DynamicGroupEditor';

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
    const buttons = screen.getAllByRole('button');
    const deleteBtn = buttons.find((b) => b.className.includes('danger'))!;
    fireEvent.click(deleteBtn);
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
});
