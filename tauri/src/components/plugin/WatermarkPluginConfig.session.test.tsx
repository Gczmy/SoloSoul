import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAuthStore } from '@/stores/authStore';
import { WatermarkPluginConfig } from './WatermarkPluginConfig';

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), onParamsChange: vi.fn() }));

vi.mock('@/lib/typedIpc', () => ({ invokeTypedCommand: mocks.invoke }));
vi.mock('@tauri-apps/api/path', () => ({ downloadDir: async () => 'test-output' }));
vi.mock('@/hooks/useAttachmentPageSort', () => ({
  useAttachmentPageSort: (pages: unknown[]) => pages,
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function tree(fileName: string, objectId: string) {
  return JSON.stringify({
    pages: [
      {
        pageName: 'identity',
        objects: [
          {
            objectId,
            objectName: 'Passport',
            attachments: [{ id: 'att-1', objectId, fileName, mimeType: 'application/pdf' }],
          },
        ],
      },
    ],
  });
}

beforeEach(() => {
  vi.resetAllMocks();
  useAuthStore.setState({
    isAuthenticated: true,
    currentAccount: { id: 'account-a', name: 'Alice' },
  });
});

describe('watermark attachment session ownership', () => {
  it('ignores account A attachment list that arrives after switching to B', async () => {
    const oldList = deferred<string>();
    const newList = deferred<string>();
    mocks.invoke.mockReturnValueOnce(oldList.promise).mockReturnValueOnce(newList.promise);
    render(<WatermarkPluginConfig onParamsChange={mocks.onParamsChange} />);
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(1));

    act(() => {
      useAuthStore.setState({ currentAccount: { id: 'account-b', name: 'Bob' } });
    });
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(2));
    await act(async () => newList.resolve(tree('B document.pdf', 'object-b')));
    await act(async () => oldList.resolve(tree('A document.pdf', 'object-a')));

    expect(screen.getByLabelText('B document.pdf')).toBeInTheDocument();
    expect(screen.queryByLabelText('A document.pdf')).not.toBeInTheDocument();
  });

  it('clears selected attachments when the same account unlocks again', async () => {
    mocks.invoke
      .mockResolvedValueOnce(tree('Old document.pdf', 'object-old'))
      .mockResolvedValueOnce(tree('New document.pdf', 'object-new'));
    render(<WatermarkPluginConfig onParamsChange={mocks.onParamsChange} />);
    fireEvent.click(await screen.findByLabelText('Old document.pdf'));
    await waitFor(() =>
      expect(JSON.parse(mocks.onParamsChange.mock.lastCall?.[0].selectedAttachments)).toHaveLength(
        1,
      ),
    );

    act(() => {
      useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
      useAuthStore.setState({
        isAuthenticated: true,
        currentAccount: { id: 'account-a', name: 'Alice' },
      });
    });

    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(2));
    expect(screen.queryByLabelText('Old document.pdf')).not.toBeInTheDocument();
    expect(await screen.findByLabelText('New document.pdf')).toBeInTheDocument();
    expect(JSON.parse(mocks.onParamsChange.mock.lastCall?.[0].selectedAttachments)).toEqual([]);
  });
});
