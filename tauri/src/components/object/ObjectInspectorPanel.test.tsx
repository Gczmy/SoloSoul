import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { useUiStore } from '@/stores/uiStore';
import { setRequestSession } from '@/lib/sessionRequests';
import { ObjectInspectorPanel } from './ObjectInspectorPanel';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@/hooks/useWorkspacePasswordGuard', () => ({
  useWorkspacePasswordGuard: () => ({
    showPwDialog: false,
    bioAvailable: { available: false },
    passwordHint: null,
    passwordVerify: vi.fn(),
  }),
}));
vi.mock('@/components/forms/PasswordVerificationDialog', () => ({
  PasswordVerificationDialog: () => null,
}));
vi.mock('@/components/attachment/AttachmentPreviewOverlay', () => ({
  AttachmentPreviewOverlay: ({
    item,
    onClose,
  }: {
    item: AttachmentItem | null;
    onClose: () => void;
  }) =>
    item ? (
      <div role="dialog" aria-label="preview">
        <span>{item.id}</span>
        <button onClick={onClose}>Close preview</button>
      </div>
    ) : null,
}));
const ipc = vi.mocked(invoke);
const context = { objectId: 'one', objectName: 'First object', ready: true };
const file = (
  id: string,
  createdAt: string,
  objectId = 'one',
  deletedAt?: string,
): AttachmentItem => ({
  id,
  createdAt,
  objectId,
  deletedAt,
  fileName: `${id}.pdf`,
  mimeType: 'application/pdf',
  sizeBytes: 1024,
});
beforeEach(() => {
  vi.clearAllMocks();
  useUiStore.setState({
    desktopObjectInspectorTab: 'attachments',
    desktopObjectInspectorOpen: true,
  });
});
afterEach(() => {
  cleanup();
  setRequestSession(null);
  useUiStore.setState({ desktopObjectInspectorOpen: false, desktopObjectInspectorTab: 'history' });
});

describe('desktop object inspector', () => {
  it('reports an unavailable object instead of showing stale contents or spinning indefinitely', () => {
    render(
      <ObjectInspectorPanel
        context={{ ...context, ready: false, failed: true }}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByRole('alert')).toHaveTextContent('object_load_failed');
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.getByRole('complementary', { name: 'object_inspector' })).toBeVisible();
    expect(screen.getByRole('tablist')).toBeVisible();
    expect(ipc).not.toHaveBeenCalled();
  });
  it('loads only active attachments for the current object, newest first, and reuses preview', async () => {
    ipc.mockResolvedValue([
      file('old', '2026-01-01'),
      file('foreign', '2026-06-01', 'other'),
      file('new', '2026-02-01'),
      file('deleted', '2026-07-01', 'one', '2026-07-02'),
    ]);
    render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
    await screen.findByText('new.pdf');
    expect(
      [...document.querySelectorAll('[data-inspector-attachment]')].map((row) =>
        row.getAttribute('data-inspector-attachment'),
      ),
    ).toEqual(['new', 'old']);
    expect(ipc.mock.calls).toEqual([['attachment_list', { objectId: 'one', showDeleted: false }]]);
    fireEvent.click(screen.getByText('new.pdf'));
    expect(screen.getByRole('dialog')).toHaveTextContent('new');
    fireEvent.click(screen.getByRole('tab', { name: 'history' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByRole('tab', { name: 'history' })).toHaveAttribute('aria-selected', 'true');
  });

  it('clears old metadata and rejects a late attachment response on object change', async () => {
    let finish!: (value: AttachmentItem[]) => void;
    ipc.mockImplementation(async (_, args) =>
      (args as { objectId: string }).objectId === 'one'
        ? new Promise<AttachmentItem[]>((resolve) => {
            finish = resolve;
          })
        : [file('current', '2026-01-01', 'two')],
    );
    const { rerender } = render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
    await waitFor(() => expect(finish).toBeDefined());
    rerender(
      <ObjectInspectorPanel
        context={{ ...context, objectId: 'two', objectName: 'Second object' }}
        onClose={vi.fn()}
      />,
    );
    await screen.findByText('current.pdf');
    await act(async () => finish([file('stale', '2026-02-01')]));
    expect(screen.queryByText('stale.pdf')).toBeNull();
    expect(screen.queryByText('First object')).toBeNull();
  });

  it('unmounts preview and old contents immediately while the next editor is loading', async () => {
    ipc.mockResolvedValue([file('one-file', '2026-01-01')]);
    const { rerender } = render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
    fireEvent.click(await screen.findByText('one-file.pdf'));
    const panel = screen.getByRole('complementary');
    const tabs = screen.getByRole('tablist');
    rerender(<ObjectInspectorPanel context={{ ...context, ready: false }} onClose={vi.fn()} />);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByText('one-file.pdf')).toBeNull();
    expect(screen.queryByText('First object')).toBeNull();
    expect(screen.getByRole('status')).toBeInTheDocument();
    expect(screen.getByRole('complementary')).toBe(panel);
    expect(screen.getByRole('tablist')).toBe(tabs);
    expect(screen.getByRole('tabpanel')).toHaveAttribute('aria-busy', 'true');
    expect(screen.getByRole('tab', { name: 'attachments' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });

  it('invalidates a pending response across lock/unlock even for the same account', async () => {
    setRequestSession('account');
    let finish!: (value: AttachmentItem[]) => void;
    ipc
      .mockImplementationOnce(
        () =>
          new Promise<AttachmentItem[]>((resolve) => {
            finish = resolve;
          }),
      )
      .mockResolvedValue([file('fresh', '2026-01-01')]);
    render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
    await waitFor(() => expect(finish).toBeDefined());
    act(() => {
      setRequestSession(null);
      setRequestSession('account');
    });
    // UI preference resets on session change; choose attachments in the new session.
    fireEvent.click(screen.getByRole('tab', { name: 'attachments' }));
    await screen.findByText('fresh.pdf');
    await act(async () => finish([file('stale', '2026-02-01')]));
    expect(screen.queryByText('stale.pdf')).toBeNull();
  });

  it('retries attachment failures and supports tab keyboard navigation', async () => {
    ipc.mockRejectedValueOnce(new Error('offline')).mockResolvedValue([]);
    render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
    await screen.findByRole('alert');
    fireEvent.click(screen.getByRole('button', { name: 'retry' }));
    await screen.findByText('no_attachments');
    fireEvent.keyDown(screen.getByRole('tab', { name: 'attachments' }), { key: 'Home' });
    expect(screen.getByRole('tab', { name: 'history' })).toHaveFocus();
    expect(screen.getByRole('tabpanel')).toHaveAttribute(
      'aria-labelledby',
      'object-inspector-tab-history',
    );
  });
});
