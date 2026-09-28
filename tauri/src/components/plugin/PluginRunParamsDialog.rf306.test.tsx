import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { pluginParam } from '@/test/pluginFixtures';
import { PluginRunParamsDialog } from './PluginRunParamsDialog';

afterEach(cleanup);

describe('RF306 PluginRunParamsDialog wire defaults', () => {
  it('submits strings for every null default, including false for a boolean', () => {
    const onSubmit = vi.fn<(values: Record<string, string>) => void>();
    render(
      <PluginRunParamsDialog
        pluginName="Synthetic plugin"
        params={[
          pluginParam({ id: 'text', label: 'Text', type: 'string', defaultValue: null }),
          pluginParam({ id: 'count', label: 'Count', type: 'number', defaultValue: null }),
          pluginParam({ id: 'enabled', label: 'Enabled', type: 'boolean', defaultValue: null }),
          pluginParam({
            id: 'choice',
            label: 'Choice',
            type: 'select',
            defaultValue: null,
            options: [{ value: 'one', label: 'One' }],
          }),
        ]}
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />,
    );

    expect(screen.getByPlaceholderText('Text')).toHaveValue('');
    expect(screen.getByRole('spinbutton')).toHaveValue(null);
    expect(screen.getByRole('checkbox', { name: 'Enabled' })).not.toBeChecked();
    expect(screen.getByRole('combobox')).toHaveValue('');
    fireEvent.click(screen.getByRole('button', { name: 'Run' }));

    expect(onSubmit).toHaveBeenCalledExactlyOnceWith({
      text: '',
      count: '',
      enabled: 'false',
      choice: '',
    });
    expect(
      Object.values(onSubmit.mock.calls[0][0]).every((value) => typeof value === 'string'),
    ).toBe(true);
  });

  it('preserves explicit empty, false, zero, true and select defaults', () => {
    const onSubmit = vi.fn();
    render(
      <PluginRunParamsDialog
        pluginName="Synthetic plugin"
        params={[
          pluginParam({ id: 'text', label: 'Text', defaultValue: 'false' }),
          pluginParam({ id: 'empty', label: 'Empty text', defaultValue: '' }),
          pluginParam({ id: 'count', label: 'Count', type: 'number', defaultValue: '0' }),
          pluginParam({ id: 'off', label: 'Off', type: 'boolean', defaultValue: 'false' }),
          pluginParam({ id: 'on', label: 'On', type: 'boolean', defaultValue: 'true' }),
          pluginParam({ id: 'emptyFlag', label: 'Empty flag', type: 'boolean', defaultValue: '' }),
          pluginParam({
            id: 'choice',
            label: 'Choice',
            type: 'select',
            defaultValue: 'false',
            options: [{ value: 'false', label: 'False option' }],
          }),
        ]}
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />,
    );

    expect(screen.getByPlaceholderText('Text')).toHaveValue('false');
    expect(screen.getByRole('spinbutton')).toHaveValue(0);
    expect(screen.getByRole('checkbox', { name: 'Off' })).not.toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'On' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'Empty flag' })).not.toBeChecked();
    expect(screen.getByRole('combobox')).toHaveValue('false');
    fireEvent.click(screen.getByRole('button', { name: 'Run' }));

    expect(onSubmit).toHaveBeenCalledExactlyOnceWith({
      text: 'false',
      empty: '',
      count: '0',
      off: 'false',
      on: 'true',
      emptyFlag: '',
      choice: 'false',
    });
  });

  it('keeps required validation and submits edited string values after correction', () => {
    const onSubmit = vi.fn();
    const onCancel = vi.fn();
    render(
      <PluginRunParamsDialog
        pluginName="Synthetic plugin"
        params={[
          pluginParam({ id: 'text', label: 'Required text', required: true }),
          pluginParam({ id: 'count', label: 'Count', type: 'number', required: true }),
          pluginParam({ id: 'enabled', label: 'Enabled', type: 'boolean', required: true }),
          pluginParam({
            id: 'choice',
            label: 'Choice',
            type: 'select',
            required: true,
            options: [{ value: 'one', label: 'One' }],
          }),
        ]}
        onSubmit={onSubmit}
        onCancel={onCancel}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Run' }));
    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getAllByText('Required')).toHaveLength(3);

    fireEvent.change(screen.getByPlaceholderText('Required text'), { target: { value: '   ' } });
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '12' } });
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'one' } });
    fireEvent.click(screen.getByRole('button', { name: 'Run' }));
    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getAllByText('Required')).toHaveLength(1);

    fireEvent.change(screen.getByPlaceholderText('Required text'), { target: { value: 'Edited' } });
    fireEvent.click(screen.getByRole('checkbox', { name: 'Enabled' }));
    expect(screen.queryByText('Required')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Run' }));
    expect(onSubmit).toHaveBeenCalledExactlyOnceWith({
      text: 'Edited',
      count: '12',
      enabled: 'true',
      choice: 'one',
    });
    expect(onCancel).not.toHaveBeenCalled();
  });
});
