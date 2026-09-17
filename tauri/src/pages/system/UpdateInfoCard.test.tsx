import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { UpdateInfoCard } from './UpdateInfoCard';

const baseProps = {
  info: { appName: 'SoloSoul', version: '2.13.0', os: 'macos', arch: 'aarch64' },
  versionInfo: {
    currentVersion: '2.13.0',
    latestVersion: '2.13.1',
    state: 'available' as const,
  },
  loading: false,
  checking: false,
  downloading: true,
  downloadProgress: { event: 'Started' as const, data: { contentLength: 100 } },
  downloadedBytes: 25,
  totalBytes: 100,
  downloadError: null,
  progressPercent: 25,
  runCheck: vi.fn(),
  handleUpdate: vi.fn(),
};

describe('UpdateInfoCard download controls', () => {
  it('shows real download progress and waits for cancellation before showing update options', () => {
    const cancelDownload = vi.fn();
    const { rerender } = render(<UpdateInfoCard {...baseProps} cancelDownload={cancelDownload} />);
    expect(screen.getByText(/25%/)).toBeInTheDocument();
    expect(screen.queryByText('common:update_installing')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'common:cancel_download' }));
    expect(cancelDownload).toHaveBeenCalledOnce();

    rerender(<UpdateInfoCard {...baseProps} cancelling cancelDownload={cancelDownload} />);
    expect(screen.getByText('common:update_cancelling')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'common:cancel_download' })).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Update Now' })).not.toBeInTheDocument();

    rerender(<UpdateInfoCard {...baseProps} downloading={false} cancelDownload={cancelDownload} />);
    expect(screen.getByRole('button', { name: 'Update Now' })).toBeInTheDocument();
  });

  it('removes cancellation during installation', () => {
    render(<UpdateInfoCard {...baseProps} installing cancelDownload={vi.fn()} />);
    expect(screen.getByText('common:update_installing')).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'common:cancel_download' }),
    ).not.toBeInTheDocument();
  });
});
