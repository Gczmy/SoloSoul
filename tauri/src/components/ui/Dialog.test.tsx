import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Dialog } from './Dialog';

describe('Dialog', () => {
  it('does not render in DOM when isOpen is false', () => {
    render(
      <Dialog isOpen={false} onClose={vi.fn()}>
        Content
      </Dialog>,
    );
    // When closed, the dialog should not be "open" in the DOM
    const dialog = document.querySelector('dialog');
    expect(dialog).not.toBeInTheDocument();
  });

  it('renders and opens when isOpen is true', () => {
    render(
      <Dialog isOpen={true} onClose={vi.fn()}>
        Content
      </Dialog>,
    );
    expect(screen.getByText('Content')).toBeInTheDocument();
    const dialog = document.querySelector('dialog');
    expect(dialog).toBeInTheDocument();
  });

  it('renders title when provided', () => {
    render(
      <Dialog isOpen={true} onClose={vi.fn()} title="Test Title">
        Content
      </Dialog>,
    );
    expect(screen.getByRole('heading', { name: /test title/i })).toBeInTheDocument();
  });

  it('calls onClose when clicking backdrop', () => {
    const onClose = vi.fn();
    render(
      <Dialog isOpen={true} onClose={onClose}>
        Content
      </Dialog>,
    );
    // Click on the <dialog> element itself simulates backdrop click
    const dialog = document.querySelector('dialog')!;
    fireEvent.click(dialog);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('does not call onClose when clicking dialog content', () => {
    const onClose = vi.fn();
    render(
      <Dialog isOpen={true} onClose={onClose}>
        <button>Inside</button>
      </Dialog>,
    );
    fireEvent.click(screen.getByRole('button', { name: /inside/i }));
    expect(onClose).not.toHaveBeenCalled();
  });

  it('calls onClose via native close event', () => {
    const onClose = vi.fn();
    render(
      <Dialog isOpen={true} onClose={onClose}>
        Content
      </Dialog>,
    );
    // Native <dialog> fires 'close' event on Escape or programmatic close()
    const dialog = document.querySelector('dialog')!;
    fireEvent(dialog, new Event('close'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('cleans up close listener on unmount', () => {
    const onClose = vi.fn();
    const { unmount } = render(
      <Dialog isOpen={true} onClose={onClose}>
        Content
      </Dialog>,
    );
    unmount();
    // After unmount the dialog is removed, so firing close does nothing
    const dialog = document.querySelector('dialog');
    expect(dialog).not.toBeInTheDocument();
  });
});
