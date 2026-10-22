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
  ipc.mockReset();
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
    expect(screen.getByRole('tabpanel')).toHaveAttribute('aria-busy', 'false');
    expect(ipc).not.toHaveBeenCalled();
  });
  it('loads only active attachments for the current object, newest first, and reuses preview', async () => {
    ipc.mockImplementation(async (command) =>
      command === 'snapshot_list'
        ? []
        : [
            file('old', '2026-01-01'),
            file('foreign', '2026-06-01', 'other'),
            file('new', '2026-02-01'),
            file('deleted', '2026-07-01', 'one', '2026-07-02'),
          ],
    );
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
    await waitFor(() =>
      expect(screen.getByRole('tab', { name: 'history' })).toHaveAttribute('aria-selected', 'true'),
    );
  });

  it.each(['history', 'attachments'] as const)(
    'keeps the current list and tab attribution until %s is ready',
    async (target) => {
      const origin = target === 'history' ? 'attachments' : 'history';
      useUiStore.setState({ desktopObjectInspectorTab: origin });
      const snapshot = [
        { id: 'one-snapshot', timestamp: 1, triggeredBy: 'user_edit', diffSummary: 'diff_updated' },
      ];
      const attachments = [file('one-file', '2026-01-01')];
      let finish!: (items: unknown) => void;
      ipc
        .mockResolvedValueOnce(origin === 'history' ? snapshot : attachments)
        .mockImplementationOnce(
          () =>
            new Promise((resolve) => {
              finish = resolve;
            }),
        );
      render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
      await waitFor(() =>
        expect(screen.getByRole('tabpanel')).toHaveAttribute('aria-busy', 'false'),
      );
      const list = screen.getByRole('tabpanel');
      const content = document.querySelector('[data-inspector-content]');
      const header = document.querySelector('[data-object-inspector] header');
      list.scrollTop = 100;
      fireEvent.click(screen.getByRole('tab', { name: target }));
      expect(document.querySelector('[data-inspector-content]')).toBe(content);
      expect(content).toHaveAttribute('inert');
      expect(screen.getByRole('tab', { name: origin })).toHaveAttribute('aria-selected', 'true');
      expect(list).toHaveAttribute('aria-labelledby', `object-inspector-tab-${origin}`);
      expect(list).toHaveAttribute('aria-busy', 'true');
      expect(list.scrollTop).toBe(100);
      expect(
        document.querySelector(
          origin === 'history' ? '[data-history-snapshot]' : '[data-inspector-attachment]',
        ),
      ).not.toBeNull();
      await act(async () => finish(target === 'history' ? snapshot : attachments));
      expect(
        document.querySelector(
          target === 'history' ? '[data-history-snapshot]' : '[data-inspector-attachment]',
        ),
      ).not.toBeNull();
      expect(screen.getByRole('tab', { name: target })).toHaveAttribute('aria-selected', 'true');
      expect(list).toHaveAttribute('aria-labelledby', `object-inspector-tab-${target}`);
      expect(list).toHaveAttribute('aria-busy', 'false');
      expect(list.scrollTop).toBe(0);
      expect(screen.getByRole('tabpanel')).toBe(list);
      expect(document.querySelector('[data-object-inspector] header')).toBe(header);
      expect(document.querySelector('[data-inspector-content]')).not.toHaveAttribute('inert');
      expect(ipc.mock.calls).toHaveLength(2);
      expect(ipc.mock.calls.some(([command]) => command === 'snapshot_get_data')).toBe(false);
    },
  );

  it('rejects a skipped tab response after rapidly returning to attachments', async () => {
    let finish!: (items: unknown) => void;
    ipc
      .mockResolvedValueOnce([file('old-file', '2026-01-01')])
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finish = resolve;
          }),
      )
      .mockResolvedValueOnce([file('fresh-file', '2026-02-01')]);
    render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
    await screen.findByText('old-file.pdf');
    fireEvent.click(screen.getByRole('tab', { name: 'history' }));
    expect(screen.getByText('old-file.pdf')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('tab', { name: 'attachments' }));
    await screen.findByText('fresh-file.pdf');
    await act(async () =>
      finish([
        {
          id: 'late-snapshot',
          timestamp: 1,
          triggeredBy: 'user_edit',
          diffSummary: 'diff_updated',
        },
      ]),
    );
    expect(screen.getByText('fresh-file.pdf')).toBeInTheDocument();
    expect(document.querySelector('[data-history-snapshot]')).toBeNull();
    expect(screen.getByRole('tab', { name: 'attachments' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });

  it('replaces retained tab contents with an attributed error, then retries to a genuine empty state', async () => {
    ipc
      .mockResolvedValueOnce([file('old-file', '2026-01-01')])
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce([]);
    render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
    await screen.findByText('old-file.pdf');
    fireEvent.click(screen.getByRole('tab', { name: 'history' }));
    expect(screen.getByText('old-file.pdf')).toBeInTheDocument();
    expect(await screen.findByRole('alert')).toHaveTextContent('history_load_failed');
    expect(screen.queryByText('old-file.pdf')).toBeNull();
    expect(screen.getByRole('tab', { name: 'history' })).toHaveAttribute('aria-selected', 'true');
    fireEvent.click(screen.getByRole('button', { name: 'retry' }));
    await screen.findByText('common:no_history');
    expect(screen.getByRole('tabpanel')).toHaveAttribute('aria-busy', 'false');
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

  it('keeps correctly attributed inert metadata until the next editor and list are both ready, but closes preview immediately', async () => {
    let finish!: (value: AttachmentItem[]) => void;
    ipc.mockResolvedValueOnce([file('one-file', '2026-01-01')]).mockImplementationOnce(
      () =>
        new Promise<AttachmentItem[]>((resolve) => {
          finish = resolve;
        }),
    );
    const { rerender } = render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
    fireEvent.click(await screen.findByText('one-file.pdf'));
    const panel = screen.getByRole('complementary');
    const tabs = screen.getByRole('tablist');
    const next = { objectId: 'two', objectName: 'Second object', ready: false };
    rerender(<ObjectInspectorPanel context={next} onClose={vi.fn()} />);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByText('one-file.pdf')).toBeInTheDocument();
    expect(screen.getByText('First object')).toBeInTheDocument();
    expect(panel).toHaveAttribute('data-object-id', 'one');
    expect(panel).toHaveAttribute('data-inspector-target-id', 'two');
    expect(document.querySelector('[data-inspector-content]')).toHaveAttribute('inert');
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.getByRole('complementary')).toBe(panel);
    expect(screen.getByRole('tablist')).toBe(tabs);
    expect(screen.getByRole('tabpanel')).toHaveAttribute('aria-busy', 'true');
    expect(screen.getByRole('tab', { name: 'attachments' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    expect(ipc.mock.calls.filter(([cmd]) => cmd === 'attachment_list')).toHaveLength(2);
    await act(async () => finish([file('two-file', '2026-02-01', 'two')]));
    expect(screen.queryByText('two-file.pdf')).toBeNull();
    expect(screen.getByText('First object')).toBeInTheDocument();
    rerender(<ObjectInspectorPanel context={{ ...next, ready: true }} onClose={vi.fn()} />);
    expect(screen.getByText('two-file.pdf')).toBeInTheDocument();
    expect(screen.getByText('Second object')).toBeInTheDocument();
    expect(screen.queryByText('one-file.pdf')).toBeNull();
    expect(screen.getByRole('complementary')).toBe(panel);
    expect(screen.getByRole('tabpanel')).toHaveAttribute('aria-busy', 'false');
    expect(document.querySelector('[data-inspector-content]')).not.toHaveAttribute('inert');
    expect(ipc.mock.calls.filter(([cmd]) => cmd === 'attachment_list')).toHaveLength(2);
  });

  it.each(['history', 'attachments'] as const)(
    'waits for %s metadata when the new editor wins the race',
    async (tab) => {
      useUiStore.setState({ desktopObjectInspectorTab: tab });
      const metadata = (id: string) =>
        tab === 'history'
          ? [
              {
                id: `${id}-snapshot`,
                timestamp: 1,
                triggeredBy: 'user_edit',
                diffSummary: 'diff_updated',
              },
            ]
          : [file(`${id}-file`, '2026-01-01', id)];
      let finish!: (value: unknown) => void;
      ipc.mockResolvedValueOnce(metadata('one')).mockImplementationOnce(
        () =>
          new Promise((done) => {
            finish = done;
          }),
      );
      const { rerender } = render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
      await waitFor(() =>
        expect(screen.getByRole('tabpanel')).toHaveAttribute('aria-busy', 'false'),
      );
      const list = screen.getByRole('tabpanel');
      list.scrollTop = 120;
      rerender(
        <ObjectInspectorPanel
          context={{ objectId: 'two', objectName: 'Second object', ready: true }}
          onClose={vi.fn()}
        />,
      );
      expect(screen.getByText('First object')).toBeInTheDocument();
      expect(screen.queryByText('Second object')).toBeNull();
      expect(list.scrollTop).toBe(120);
      expect(screen.getByRole('tabpanel')).toBe(list);
      await act(async () => finish(metadata('two')));
      expect(screen.getByText('Second object')).toBeInTheDocument();
      expect(screen.queryByText('First object')).toBeNull();
      expect(list.scrollTop).toBe(0);
      expect(screen.getByRole('tabpanel')).toBe(list);
      expect(
        ipc.mock.calls.filter(
          ([cmd]) => cmd === (tab === 'history' ? 'snapshot_list' : 'attachment_list'),
        ),
      ).toHaveLength(2);
      expect(ipc.mock.calls.some(([cmd]) => cmd === 'snapshot_get_data')).toBe(false);
    },
  );

  it('clears retained metadata on failure and ignores a skipped object response', async () => {
    let skipped!: (value: AttachmentItem[]) => void;
    ipc
      .mockResolvedValueOnce([file('one-file', '2026-01-01')])
      .mockImplementationOnce(
        () =>
          new Promise<AttachmentItem[]>((done) => {
            skipped = done;
          }),
      )
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce([file('three-file', '2026-02-01', 'three')]);
    const { rerender } = render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
    await screen.findByText('one-file.pdf');
    rerender(
      <ObjectInspectorPanel
        context={{ objectId: 'two', objectName: '', ready: false }}
        onClose={vi.fn()}
      />,
    );
    rerender(
      <ObjectInspectorPanel
        context={{ objectId: 'three', objectName: 'Third object', ready: true }}
        onClose={vi.fn()}
      />,
    );
    await screen.findByRole('alert');
    expect(screen.queryByText('one-file.pdf')).toBeNull();
    expect(screen.queryByText('First object')).toBeNull();
    expect(screen.getByRole('tabpanel')).toHaveAttribute('aria-busy', 'false');
    fireEvent.click(screen.getByRole('button', { name: 'retry' }));
    await screen.findByText('three-file.pdf');
    await act(async () => skipped([file('two-file', '2026-01-01', 'two')]));
    expect(screen.queryByText('two-file.pdf')).toBeNull();
    expect(screen.getByText('Third object')).toBeInTheDocument();
  });

  it('drops displayed metadata synchronously on a lock/unlock cycle', async () => {
    setRequestSession('account');
    useUiStore.setState({ desktopObjectInspectorTab: 'attachments' });
    ipc
      .mockResolvedValueOnce([file('old-file', '2026-01-01')])
      .mockImplementation(() => new Promise(() => {}));
    render(<ObjectInspectorPanel context={context} onClose={vi.fn()} />);
    await screen.findByText('old-file.pdf');
    act(() => {
      setRequestSession(null);
      setRequestSession('account');
    });
    expect(screen.queryByText('old-file.pdf')).toBeNull();
    expect(screen.queryByText('First object')).toBeNull();
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
    await waitFor(() =>
      expect(screen.getByRole('tabpanel')).toHaveAttribute(
        'aria-labelledby',
        'object-inspector-tab-history',
      ),
    );
  });
});
