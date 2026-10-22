import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { TemplateFieldInput } from './TemplateFieldInput';

it.each(['text', 'multiline', 'number', 'email'] as const)(
  '只读 %s 仍可聚焦选取，拒绝值变更',
  (type) => {
    const onChange = vi.fn();
    render(
      <TemplateFieldInput
        propertyId="locked"
        label="Locked value"
        type={type}
        value={type === 'number' ? '42' : 'Public text'}
        readOnly
        onChange={onChange}
      />,
    );
    const input = screen.getByLabelText('Locked value');
    expect(input).toHaveAttribute('readonly');
    expect(input).not.toBeDisabled();
    fireEvent.change(input, { target: { value: '999' } });
    expect(onChange).not.toHaveBeenCalled();
  },
);

it('只读选择和复选框禁止修改但保持原值', () => {
  const onChange = vi.fn();
  render(
    <>
      <TemplateFieldInput
        propertyId="choice"
        label="Choice"
        type="select"
        value="A"
        options={['A', 'B']}
        readOnly
        onChange={onChange}
      />
      <TemplateFieldInput
        propertyId="check"
        label="Check"
        type="boolean"
        value={true}
        readOnly
        onChange={onChange}
      />
    </>,
  );
  expect(screen.getByRole('combobox')).toBeDisabled();
  expect(screen.getByRole('combobox')).toHaveValue('A');
  expect(screen.getByRole('checkbox')).toBeDisabled();
  expect(screen.getByRole('checkbox')).toBeChecked();
  expect(onChange).not.toHaveBeenCalled();
});

it('旧模板重复多选项只显示一次，并按模板顺序回填唯一值', () => {
  const onChange = vi.fn();
  render(
    <TemplateFieldInput
      propertyId="colors"
      label="颜色"
      type="multiselect"
      value={['Blue', 'Blue']}
      options={['Red', 'Blue', 'Red']}
      onChange={onChange}
    />,
  );

  expect(screen.getAllByRole('checkbox')).toHaveLength(2);
  fireEvent.click(screen.getByRole('checkbox', { name: 'Red' }));
  expect(onChange).toHaveBeenCalledExactlyOnceWith(['Red', 'Blue']);
});

it('旧模板重复单选项只显示一次', () => {
  render(
    <TemplateFieldInput
      propertyId="color"
      label="颜色"
      type="select"
      value="Red"
      options={['Red', 'Blue', 'Red']}
      onChange={vi.fn()}
    />,
  );

  expect(screen.getAllByRole('option')).toHaveLength(3);
  expect(screen.getByRole('combobox', { name: '颜色' })).toHaveValue('Red');
});
