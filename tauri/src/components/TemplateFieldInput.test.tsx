import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { TemplateFieldInput } from './TemplateFieldInput';

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
