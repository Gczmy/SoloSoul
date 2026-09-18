import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { DeviceNameEditor } from './DeviceNameEditor';
import type { SyncPeer } from '@/stores/syncStore';

const peer: SyncPeer = {
  id: 'peer',
  name: 'original',
  customName: 'Work Mac',
  addr: '',
  fingerprint: '12345678',
  trusted: true,
  lastSeen: '',
};

describe('DeviceNameEditor', () => {
  it('saves a trimmed name and keeps the form open while saving', async () => {
    let finish!: () => void;
    const save = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    );
    render(<DeviceNameEditor peer={peer} onSave={save} />);
    fireEvent.click(screen.getByRole('button', { name: 'settings:sync_device_name_edit' }));
    fireEvent.change(screen.getByRole('textbox'), { target: { value: '  My laptop  ' } });
    fireEvent.click(screen.getByRole('button', { name: 'common:save' }));
    expect(save).toHaveBeenCalledWith('peer', 'My laptop');
    expect(screen.getByRole('textbox')).toBeDisabled();
    finish();
    await waitFor(() => expect(screen.queryByRole('textbox')).not.toBeInTheDocument());
  });

  it('preserves the draft after an error and allows retry or cancellation', async () => {
    const save = vi
      .fn()
      .mockRejectedValueOnce(new Error('Save failed'))
      .mockResolvedValue(undefined);
    render(<DeviceNameEditor peer={peer} onSave={save} />);
    fireEvent.click(screen.getByRole('button', { name: 'settings:sync_device_name_edit' }));
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'Phone' } });
    fireEvent.click(screen.getByRole('button', { name: 'common:save' }));
    await waitFor(() => expect(screen.getByRole('textbox')).not.toBeDisabled());
    expect(screen.getByRole('textbox')).toHaveValue('Phone');
    expect(screen.getByText(/Save failed/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'common:cancel' }));
    expect(screen.getByText('Work Mac')).toBeInTheDocument();
    expect(save).toHaveBeenCalledTimes(1);
  });

  it('allows an empty name to restore the automatic name', async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    render(<DeviceNameEditor peer={peer} onSave={save} />);
    fireEvent.click(screen.getByRole('button', { name: 'settings:sync_device_name_edit' }));
    fireEvent.change(screen.getByRole('textbox'), { target: { value: '' } });
    fireEvent.click(screen.getByRole('button', { name: 'common:save' }));
    await waitFor(() => expect(save).toHaveBeenCalledWith('peer', ''));
  });
});
