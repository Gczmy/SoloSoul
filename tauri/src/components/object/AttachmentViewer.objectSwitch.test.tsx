import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { AttachmentViewer } from './AttachmentViewer';

vi.mock('@/hooks/useDragToAttach', () => ({
  useDragToAttach: () => ({
    ref: { current: null },
    dragState: {
      isDraggingOver: false,
      isUploading: false,
      currentIndex: 0,
      totalFiles: 0,
      currentFileName: '',
      pendingFiles: 0,
    },
  }),
}));

const mockInvoke = vi.mocked(invoke);

function attachment(objectId: string): AttachmentItem {
  return {
    id: 'shared-id',
    objectId,
    fileName: objectId + '.txt',
    mimeType: 'text/plain',
    sizeBytes: 24,
    createdAt: '2026-09-28T00:00:00Z',
    vaultPath: '/synthetic-vault/' + objectId + '.txt',
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((yes) => {
    resolve = yes;
  });
  return { promise, resolve };
}

describe('RF-917 attachment viewer object ownership', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  afterEach(() => {
    cleanup();
  });

  it('clears the previous object selection and confirmation when the viewer switches objects', async () => {
    mockInvoke.mockImplementation((command, args) => {
      if (command === 'set_status_bar_style') return Promise.resolve(undefined);
      if (command !== 'attachment_list') throw new Error('Unexpected IPC: ' + command);
      const request = args as { objectId: string; showDeleted: boolean };
      return Promise.resolve(request.showDeleted ? [] : [attachment(request.objectId)]);
    });
    const onClose = vi.fn();
    const view = render(<AttachmentViewer objectId="object-a" onClose={onClose} />);
    fireEvent.click(await screen.findByRole('checkbox', { name: 'object-a.txt' }));
    expect(screen.getByRole('checkbox', { name: 'object-a.txt' })).toBeChecked();
    fireEvent.click(screen.getByRole('button', { name: 'common:delete' }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    view.rerender(<AttachmentViewer objectId="object-b" onClose={onClose} />);
    const nextRow = await screen.findByRole('checkbox', { name: 'object-b.txt' });
    expect(screen.queryByRole('checkbox', { name: 'object-a.txt' })).not.toBeInTheDocument();
    expect(nextRow).not.toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'select_all' })).not.toBePartiallyChecked();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('ignores the previous object list after its requests finish late', async () => {
    const oldActive = deferred<AttachmentItem[]>();
    const oldTrash = deferred<AttachmentItem[]>();
    mockInvoke.mockImplementation((command, args) => {
      if (command === 'set_status_bar_style') return Promise.resolve(undefined);
      if (command !== 'attachment_list') throw new Error('Unexpected IPC: ' + command);
      const request = args as { objectId: string; showDeleted: boolean };
      if (request.objectId === 'object-a') {
        return request.showDeleted ? oldTrash.promise : oldActive.promise;
      }
      return Promise.resolve(request.showDeleted ? [] : [attachment('object-b')]);
    });
    const onClose = vi.fn();
    const view = render(<AttachmentViewer objectId="object-a" onClose={onClose} />);
    expect(mockInvoke).toHaveBeenCalledWith('attachment_list', {
      objectId: 'object-a',
      showDeleted: false,
    });
    view.rerender(<AttachmentViewer objectId="object-b" onClose={onClose} />);
    await screen.findByRole('checkbox', { name: 'object-b.txt' });

    await act(async () => {
      oldActive.resolve([attachment('object-a')]);
      oldTrash.resolve([]);
      await Promise.all([oldActive.promise, oldTrash.promise]);
    });
    await waitFor(() => {
      expect(screen.getByRole('checkbox', { name: 'object-b.txt' })).toBeInTheDocument();
      expect(screen.queryByRole('checkbox', { name: 'object-a.txt' })).not.toBeInTheDocument();
    });
    expect(mockInvoke).toHaveBeenCalledWith('attachment_list', {
      objectId: 'object-b',
      showDeleted: false,
    });
  });
});
