import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Dialog } from './Dialog';

describe('Dialog', () => {
  it('does not render when isOpen is false', () => {
    render(
      <Dialog isOpen={false} onClose={vi.fn()}>
        Content
      </Dialog>,
    );
    expect(screen.queryByText('Content')).not.toBeInTheDocument();
  });

  it('renders when isOpen is true', () => {
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

  it('calls onClose when clicking overlay', () => {
    const onClose = vi.fn();
    const { container } = render(
      <Dialog isOpen={true} onClose={onClose}>
        Content
      </Dialog>,
    );
    const overlay = container.querySelector('div[class*="overlay"]');
    expect(overlay).toBeInTheDocument();
    fireEvent.click(overlay!);
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

  it('removes keydown listener on unmount', () => {
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
