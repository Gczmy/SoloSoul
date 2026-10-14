import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Dialog } from './Dialog';

const native = vi.hoisted(() => ({ enabled: true, release: vi.fn() }));
vi.mock('@/stores/nativeWindowStore', () => ({
  useNativeWindowStore: (select: (state: unknown) => unknown) =>
    select({ windowsFrame: native.enabled ? { fullWindowBackdrop: true } : null }),
}));
vi.mock('@/lib/nativeTitlebarControls', () => ({
  releaseNativeWindowBackdrop: native.release,
}));

function pendingRelease() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  native.release.mockReturnValue(promise);
  return { resolve, promise };
}
async function acknowledge(pending: ReturnType<typeof pendingRelease>) {
  await act(async () => {
    pending.resolve();
    await pending.promise;
  });
}
beforeEach(() => {
  native.enabled = true;
  native.release.mockReset();
});

describe('native Dialog close acknowledgement', () => {
  it('does not close a reopened dialog when the previous native request finishes late', async () => {
    const pending = pendingRelease();
    const onClose = vi.fn();
    const view = render(
      <Dialog isOpen onClose={onClose}>
        Public card
      </Dialog>,
    );
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(native.release).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();
    view.rerender(
      <Dialog isOpen={false} onClose={onClose}>
        Public card
      </Dialog>,
    );
    view.rerender(
      <Dialog isOpen onClose={onClose}>
        Public card
      </Dialog>,
    );
    await acknowledge(pending);
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('waits for one acknowledgement before closing and ignores repeated Escape', async () => {
    const pending = pendingRelease();
    const onClose = vi.fn();
    render(
      <Dialog isOpen onClose={onClose}>
        Public card
      </Dialog>,
    );
    fireEvent.keyDown(document, { key: 'Escape' });
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(native.release).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();
    await acknowledge(pending);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('does not call the close callback after unmounting', async () => {
    const pending = pendingRelease();
    const onClose = vi.fn();
    const view = render(
      <Dialog isOpen onClose={onClose}>
        Public card
      </Dialog>,
    );
    fireEvent.keyDown(document, { key: 'Escape' });
    view.unmount();
    await acknowledge(pending);
    expect(onClose).not.toHaveBeenCalled();
  });

  it('still completes the requested close if the same open dialog falls back to a DOM backdrop', async () => {
    const pending = pendingRelease();
    const onClose = vi.fn();
    const view = render(
      <Dialog isOpen onClose={onClose}>
        Public card
      </Dialog>,
    );
    fireEvent.keyDown(document, { key: 'Escape' });
    native.enabled = false;
    view.rerender(
      <Dialog isOpen onClose={onClose}>
        Public card
      </Dialog>,
    );
    await acknowledge(pending);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
