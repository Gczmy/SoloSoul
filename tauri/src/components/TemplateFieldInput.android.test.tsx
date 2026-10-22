import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { TemplateFieldInput } from './TemplateFieldInput';
import { DynamicGroupEditor } from './editor/DynamicGroupEditor';
vi.mock('@/lib/platform', () => ({ isAndroidSync: () => true }));

it('Android multiselect cancels without writing and confirms unique values in template order', () => {
  const onChange = vi.fn();
  render(
    <TemplateFieldInput
      propertyId="color"
      label="Color"
      type="multiselect"
      value={['Blue', 'Blue']}
      options={['Red', 'Blue', 'Red']}
      onChange={onChange}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Color' }));
  expect(screen.getAllByRole('checkbox')).toHaveLength(2);
  fireEvent.click(screen.getByRole('checkbox', { name: 'Red' }));
  fireEvent.click(screen.getByRole('button', { name: 'cancel' }));
  expect(onChange).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Color' }));
  fireEvent.click(screen.getByRole('checkbox', { name: 'Red' }));
  fireEvent.click(screen.getByRole('button', { name: 'confirm' }));
  expect(onChange).toHaveBeenCalledExactlyOnceWith(['Red', 'Blue']);
});
it('Android disabled select cannot open or change protected values', () => {
  const onChange = vi.fn();
  render(
    <TemplateFieldInput
      propertyId="color"
      label="Color"
      type="select"
      value="Red"
      options={['Red', 'Blue']}
      onChange={onChange}
      disabled
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Color' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(onChange).not.toHaveBeenCalled();
});
it('Android dynamic type selection retains type-change reset semantics', () => {
  const onChange = vi.fn();
  render(
    <DynamicGroupEditor
      propertyId="group"
      label="Group"
      value={[{ id: 'a', name: 'Public', type: 'text', value: 'old' }]}
      allowedTypes={['text', 'number']}
      onChange={onChange}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Field type' }));
  fireEvent.click(screen.getByRole('radio', { name: 'editor:field_types.number' }));
  expect(onChange).toHaveBeenCalledExactlyOnceWith([
    { id: 'a', name: 'Public', type: 'number', value: 0 },
  ]);
});
