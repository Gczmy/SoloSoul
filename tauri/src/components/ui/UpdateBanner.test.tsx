import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { UpdateBanner } from './UpdateBanner';

const baseProps = {
  version: '2.1.0',
  onUpdate: vi.fn(),
  onInstall: vi.fn(),
  onSkip: vi.fn(),
  onClose: vi.fn(),
  downloadedBytes: 0,
  totalBytes: 0,
};

describe('UpdateBanner', () => {
  it('renders version info and action buttons when available', () => {
    render(<UpdateBanner {...baseProps} state="available" />);

    expect(screen.getByText(/update_available/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /update_now/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /skip_version/i })).toBeInTheDocument();
  });

  it('calls onUpdate when clicking update button', () => {
    const onUpdate = vi.fn();
    render(<UpdateBanner {...baseProps} state="available" onUpdate={onUpdate} />);

    fireEvent.click(screen.getByRole('button', { name: /update_now/i }));
    expect(onUpdate).toHaveBeenCalledTimes(1);
  });

  it('calls onSkip when clicking skip button', () => {
    const onSkip = vi.fn();
    render(<UpdateBanner {...baseProps} state="available" onSkip={onSkip} />);

    fireEvent.click(screen.getByRole('button', { name: /skip_version/i }));
    expect(onSkip).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when clicking close button', () => {
    const onClose = vi.fn();
    render(<UpdateBanner {...baseProps} state="available" onClose={onClose} />);

    fireEvent.click(screen.getByLabelText(/close/i));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('renders progress info when downloading', () => {
    render(
      <UpdateBanner
        {...baseProps}
        state="downloading"
        downloadedBytes={50 * 1024 * 1024}
        totalBytes={100 * 1024 * 1024}
      />,
    );

    expect(screen.getByText(/update_downloading/i)).toBeInTheDocument();
    expect(screen.getByText(/50.0 MB \/ 100.0 MB/i)).toBeInTheDocument();
  });

  it('renders install button when downloaded', () => {
    render(<UpdateBanner {...baseProps} state="downloaded" />);

    expect(screen.getByText(/update_downloaded/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /install_update/i })).toBeInTheDocument();
  });

  it('calls onInstall when clicking install button', () => {
    const onInstall = vi.fn();
    render(<UpdateBanner {...baseProps} state="downloaded" onInstall={onInstall} />);

    fireEvent.click(screen.getByRole('button', { name: /install_update/i }));
    expect(onInstall).toHaveBeenCalledTimes(1);
  });
});
