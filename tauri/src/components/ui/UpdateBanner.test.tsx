import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { UpdateBanner } from './UpdateBanner';

describe('UpdateBanner', () => {
  it('renders version info and action buttons', () => {
    render(<UpdateBanner version="2.1.0" onUpdate={vi.fn()} onSkip={vi.fn()} onClose={vi.fn()} />);

    expect(screen.getByText(/update_available/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /update_now/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /skip_version/i })).toBeInTheDocument();
  });

  it('calls onUpdate when clicking update button', () => {
    const onUpdate = vi.fn();
    render(<UpdateBanner version="2.1.0" onUpdate={onUpdate} onSkip={vi.fn()} onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /update_now/i }));
    expect(onUpdate).toHaveBeenCalledTimes(1);
  });

  it('calls onSkip when clicking skip button', () => {
    const onSkip = vi.fn();
    render(<UpdateBanner version="2.1.0" onUpdate={vi.fn()} onSkip={onSkip} onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /skip_version/i }));
    expect(onSkip).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when clicking close button', () => {
    const onClose = vi.fn();
    render(<UpdateBanner version="2.1.0" onUpdate={vi.fn()} onSkip={vi.fn()} onClose={onClose} />);

    fireEvent.click(screen.getByLabelText(/close/i));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
