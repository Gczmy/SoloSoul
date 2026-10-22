import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { HistoryViewer } from './HistoryViewer';
import { setRequestSession } from '@/lib/sessionRequests';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const ipc = vi.mocked(invoke);
const entry = (id: string, timestamp: number, diffSummary = 'diff_updated') => ({
  id,
  timestamp,
  diffSummary,
  triggeredBy: 'user_edit',
});
const props = {
  objectId: 'object',
  objectName: 'Object',
  presentation: 'timeline' as const,
  onClose: vi.fn(),
  passwordVerify: vi.fn(async () => ({ ok: true, method: 'password' as const })),
  getFieldSensitivity: () => 'public' as const,
  getFieldName: (key: string) => key,
  isFieldDeprecated: () => false,
};
beforeEach(() => vi.clearAllMocks());
afterEach(() => {
  cleanup();
  setRequestSession(null);
});
const row = (id: string) =>
  document.querySelector<HTMLButtonElement>(`[data-history-snapshot="${id}"]`)!;

describe('embedded history timeline', () => {
  it('uses prepared metadata without a second list read, and clears decrypted data during handoff', async () => {
    ipc.mockResolvedValue({
      properties: { secret: 'SECRET_VALUE' },
      propertyLabels: { secret: 'sensitive' },
    });
    const prepared = [entry('prepared', 2)];
    const { rerender } = render(<HistoryViewer {...props} initialSnapshots={prepared} />);
    expect(row('prepared')).toBeInTheDocument();
    expect(ipc).not.toHaveBeenCalled();
    fireEvent.click(row('prepared'));
    fireEvent.click(await screen.findByText('••••••••'));
    await screen.findByText('SECRET_VALUE');
    rerender(<HistoryViewer {...props} initialSnapshots={prepared} active={false} />);
    expect(screen.queryByText('SECRET_VALUE')).toBeNull();
    expect(row('prepared')).toHaveAttribute('aria-expanded', 'false');
    rerender(<HistoryViewer {...props} initialSnapshots={prepared} active />);
    fireEvent.click(row('prepared'));
    await screen.findByText('••••••••');
    expect(screen.queryByText('SECRET_VALUE')).toBeNull();
    expect(ipc.mock.calls.map(([cmd]) => cmd)).toEqual(['snapshot_get_data', 'snapshot_get_data']);
  });

  it('rejects pending critical verification and auditing after a handoff', async () => {
    let finish!: (value: { ok: boolean; method: 'password' }) => void;
    const verify = vi.fn(
      () =>
        new Promise<{ ok: boolean; method: 'password' }>((done) => {
          finish = done;
        }),
    );
    ipc.mockResolvedValue({
      properties: { secret: 'CRITICAL_SECRET' },
      propertyLabels: { secret: 'critical' },
    });
    const prepared = [entry('prepared', 2)];
    const { rerender } = render(
      <HistoryViewer {...props} initialSnapshots={prepared} passwordVerify={verify} />,
    );
    fireEvent.click(row('prepared'));
    fireEvent.click(await screen.findByText('••••••••'));
    await waitFor(() => expect(verify).toHaveBeenCalled());
    rerender(
      <HistoryViewer
        {...props}
        initialSnapshots={prepared}
        passwordVerify={verify}
        active={false}
      />,
    );
    await act(async () => finish({ ok: true, method: 'password' }));
    expect(screen.queryByText('CRITICAL_SECRET')).toBeNull();
    expect(ipc.mock.calls.some(([cmd]) => cmd === 'log_write')).toBe(false);
    expect(row('prepared')).toHaveAttribute('aria-expanded', 'false');
  });

  it('sorts metadata newest first without decrypting snapshots or echoing unknown summaries', async () => {
    ipc.mockResolvedValue([
      entry('old', 1),
      entry('new', 3, 'PRIVATE_LEGACY_SUMMARY'),
      entry('middle', 2),
    ]);
    const count = vi.fn();
    render(<HistoryViewer {...props} onCountChange={count} />);
    await waitFor(() => expect(row('new')).not.toBeNull());
    expect(
      [...document.querySelectorAll('[data-history-snapshot]')].map((el) =>
        el.getAttribute('data-history-snapshot'),
      ),
    ).toEqual(['new', 'middle', 'old']);
    expect(document.body.innerHTML).not.toContain('PRIVATE_LEGACY_SUMMARY');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(ipc.mock.calls.map(([cmd]) => cmd)).toEqual(['snapshot_list']);
    await waitFor(() => expect(count).toHaveBeenCalledWith(3));
  });

  it('only loads expanded snapshots and resets revealed values after collapsing', async () => {
    ipc.mockImplementation(async (cmd) =>
      cmd === 'snapshot_list'
        ? [entry('one', 2), entry('two', 1)]
        : {
            properties: { secret: 'SECRET_VALUE' },
            propertyLabels: { secret: 'sensitive' },
          },
    );
    render(<HistoryViewer {...props} />);
    await waitFor(() => expect(row('one')).not.toBeNull());
    fireEvent.click(row('one'));
    fireEvent.click(await screen.findByText('••••••••'));
    await screen.findByText('SECRET_VALUE');
    fireEvent.click(row('one'));
    expect(screen.queryByText('SECRET_VALUE')).toBeNull();
    fireEvent.click(row('one'));
    await screen.findByText('••••••••');
    expect(screen.queryByText('SECRET_VALUE')).toBeNull();
    fireEvent.click(row('two'));
    expect(row('one')).toHaveAttribute('aria-expanded', 'false');
    await waitFor(() =>
      expect(ipc.mock.calls.filter(([cmd]) => cmd === 'snapshot_get_data')).toHaveLength(2),
    );
  });

  it('rejects critical verification and auditing after collapsing its record', async () => {
    let resolve!: (value: { ok: boolean; method: 'password' }) => void;
    const verify = vi.fn(
      () =>
        new Promise<{ ok: boolean; method: 'password' }>((done) => {
          resolve = done;
        }),
    );
    ipc.mockImplementation(async (cmd) =>
      cmd === 'snapshot_list'
        ? [entry('one', 1)]
        : {
            properties: { secret: 'CRITICAL_SECRET' },
            propertyLabels: { secret: 'critical' },
          },
    );
    render(<HistoryViewer {...props} passwordVerify={verify} />);
    await waitFor(() => expect(row('one')).not.toBeNull());
    fireEvent.click(row('one'));
    fireEvent.click(await screen.findByText('••••••••'));
    await waitFor(() => expect(verify).toHaveBeenCalled());
    fireEvent.click(row('one'));
    await act(async () => resolve({ ok: true, method: 'password' }));
    expect(document.body.innerHTML).not.toContain('CRITICAL_SECRET');
    expect(ipc.mock.calls.some(([cmd]) => cmd === 'log_write')).toBe(false);
  });

  it('discards a late list from the previous object', async () => {
    let resolve!: (value: unknown) => void;
    ipc.mockImplementation(async (_, args) =>
      (args as { objectId: string }).objectId === 'object'
        ? new Promise((done) => {
            resolve = done;
          })
        : [entry('current', 1)],
    );
    const { rerender } = render(<HistoryViewer {...props} />);
    rerender(<HistoryViewer {...props} objectId="other" />);
    await waitFor(() => expect(row('current')).not.toBeNull());
    await act(async () => resolve([entry('stale', 2)]));
    expect(row('stale')).toBeNull();
  });

  it('shows a retryable failure instead of an empty history', async () => {
    ipc.mockRejectedValueOnce(new Error('offline')).mockResolvedValueOnce([entry('retry', 1)]);
    render(<HistoryViewer {...props} />);
    await screen.findByRole('alert');
    expect(screen.queryByText('common:no_history')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'common:retry' }));
    await waitFor(() => expect(row('retry')).not.toBeNull());
  });
});
