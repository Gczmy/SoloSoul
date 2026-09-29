import { act, render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useDragToAttach } from './useDragToAttach';

type DragPayload = {
  type: 'drop';
  paths: string[];
  position: { x: number; y: number };
};

const mocks = vi.hoisted(() => ({
  listener: null as null | ((event: { payload: DragPayload }) => void),
  upload: vi.fn(),
  filter: vi.fn(),
}));

vi.mock('@tauri-apps/api/webviewWindow', () => ({
  getCurrentWebviewWindow: () => ({
    onDragDropEvent: async (listener: (event: { payload: DragPayload }) => void) => {
      mocks.listener = listener;
      return () => {
        mocks.listener = null;
      };
    },
  }),
}));

vi.mock('@/lib/attachmentUpload', () => ({
  uploadAttachmentsSequentially: mocks.upload,
  filterOutDirectories: mocks.filter,
}));

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function DropTarget({ objectId }: { objectId: string }) {
  const { ref, dragState } = useDragToAttach(objectId);
  return (
    <div ref={ref} data-testid="drop-target" data-pending={dragState.pendingFiles}>
      {dragState.isUploading ? 'uploading' : 'idle'}
    </div>
  );
}

async function drop(path: string) {
  await act(async () => {
    mocks.listener?.({ payload: { type: 'drop', paths: [path], position: { x: 10, y: 10 } } });
  });
}

describe('useDragToAttach upload ownership', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listener = null;
    mocks.filter.mockImplementation(async (paths: string[]) => ({ files: paths, dirs: [] }));
    Object.defineProperty(document, 'elementFromPoint', {
      configurable: true,
      value: vi.fn(() => document.querySelector('[data-testid="drop-target"]')),
    });
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
      left: 0,
      top: 0,
      right: 100,
      bottom: 100,
      width: 100,
      height: 100,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(document, 'elementFromPoint');
  });

  it('uploads each queued drop to the object that received it before the panel switched', async () => {
    const first = deferred();
    mocks.upload.mockReturnValueOnce(first.promise).mockResolvedValue(undefined);
    const view = render(<DropTarget objectId="object-a" />);
    await waitFor(() => expect(mocks.listener).not.toBeNull());

    await drop('first.pdf');
    await waitFor(() =>
      expect(mocks.upload).toHaveBeenCalledWith(['first.pdf'], 'object-a', expect.any(Function)),
    );

    view.rerender(<DropTarget objectId="object-b" />);
    await drop('second.pdf');
    expect(view.getByTestId('drop-target')).toHaveAttribute('data-pending', '1');

    await act(async () => {
      first.resolve();
      await first.promise;
    });
    await waitFor(() => expect(mocks.upload).toHaveBeenCalledTimes(2));
    expect(mocks.upload).toHaveBeenNthCalledWith(
      2,
      ['second.pdf'],
      'object-b',
      expect.any(Function),
    );
    await waitFor(() => expect(view.getByTestId('drop-target')).toHaveTextContent('idle'));
  });

  it('accepts the same file on a different object while still deduplicating one object', async () => {
    const first = deferred();
    mocks.upload.mockReturnValueOnce(first.promise).mockResolvedValue(undefined);
    const view = render(<DropTarget objectId="object-a" />);
    await waitFor(() => expect(mocks.listener).not.toBeNull());

    await drop('shared.pdf');
    await waitFor(() => expect(mocks.upload).toHaveBeenCalledTimes(1));
    await drop('shared.pdf');
    expect(view.getByTestId('drop-target')).toHaveAttribute('data-pending', '0');

    view.rerender(<DropTarget objectId="object-b" />);
    await drop('shared.pdf');
    expect(view.getByTestId('drop-target')).toHaveAttribute('data-pending', '1');

    await act(async () => {
      first.resolve();
      await first.promise;
    });
    await waitFor(() => expect(mocks.upload).toHaveBeenCalledTimes(2));
    expect(mocks.upload).toHaveBeenNthCalledWith(
      2,
      ['shared.pdf'],
      'object-b',
      expect.any(Function),
    );
  });
});
