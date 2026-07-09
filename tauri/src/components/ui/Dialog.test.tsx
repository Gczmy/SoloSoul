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
    expect(screen.queryByText('Content')).not.toBeInTheDocument();
  });

  it('renders and opens when isOpen is true', () => {
    render(
      <Dialog isOpen={true} onClose={vi.fn()}>
        Content
      </Dialog>,
    );
    expect(screen.getByText('Content')).toBeInTheDocument();
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
    const backdrop = document.querySelector('[class*="backdrop"]');
    expect(backdrop).toBeInTheDocument();
    fireEvent.click(backdrop!);
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

  it('calls onClose when pressing Escape', () => {
    const onClose = vi.fn();
    render(
      <Dialog isOpen={true} onClose={onClose}>
        Content
      </Dialog>,
    );
    fireEvent.keyDown(document, { key: 'Escape' });
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
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).not.toHaveBeenCalled();
  });
});
