import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { pluginParam } from '@/test/pluginFixtures';
import { PluginRunParamsDialog } from './PluginRunParamsDialog';
vi.mock('@/lib/platform', () => ({ isAndroidSync: () => true }));
it('Android picker preserves required validation, defaults, and string submission', () => {
  const onSubmit = vi.fn(),
    onCancel = vi.fn();
  render(
    <PluginRunParamsDialog
      pluginName="Public"
      params={[
        pluginParam({
          id: 'required',
          label: 'Choice',
          type: 'select',
          required: true,
          options: [{ value: 'one', label: 'One' }],
        }),
        pluginParam({ id: 'enabled', label: 'Enabled', type: 'boolean', defaultValue: 'false' }),
        pluginParam({ id: 'count', label: 'Count', type: 'number', defaultValue: '0' }),
      ]}
      onSubmit={onSubmit}
      onCancel={onCancel}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Run' }));
  expect(onSubmit).not.toHaveBeenCalled();
  expect(screen.getByText('Required')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Choice' }));
  fireEvent.keyDown(document, { key: 'Escape' });
  expect(onCancel).not.toHaveBeenCalled();
  expect(onSubmit).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Choice' }));
  fireEvent.click(screen.getByRole('radio', { name: 'One' }));
  expect(screen.queryByText('Required')).not.toBeInTheDocument();
  expect(onSubmit).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Run' }));
  expect(onSubmit).toHaveBeenCalledExactlyOnceWith({
    required: 'one',
    enabled: 'false',
    count: '0',
  });
});
