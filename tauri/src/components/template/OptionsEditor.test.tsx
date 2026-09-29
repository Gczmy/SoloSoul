import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { OptionsEditor } from './OptionsEditor';

it('保存选项时去重并保留首次出现的顺序', () => {
  const onChange = vi.fn();
  render(
    <OptionsEditor
      options={['Red', 'Blue']}
      onChange={onChange}
      fieldName="颜色"
      fieldType="multiselect"
    />,
  );

  fireEvent.click(screen.getByRole('button', { name: '2 个选项' }));
  fireEvent.change(screen.getByRole('textbox'), {
    target: { value: ' Red \nBlue\nRed\n \nGreen\nBlue' },
  });
  fireEvent.click(screen.getByRole('button', { name: '确定' }));

  expect(onChange).toHaveBeenCalledExactlyOnceWith(['Red', 'Blue', 'Green']);
});
