import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { setRequestSession } from '@/lib/sessionRequests';
import { PagePaneObjectPreview } from './PagePanePreview';
import type { ObjectData } from '@/lib/generated/ipcContracts';
import { useRef } from 'react';
import { PagePanePreviewFrame } from './PagePanePreviewFrame';

const object = (id: string): ObjectData => ({
  id,
  accountId: 'preview-account',
  name: `Object ${id}`,
  typeId: 'identity',
  sensitivityLevel: 'internal',
  createdAt: '',
  updatedAt: '',
  properties: { public: 'Public preview', secret: 'PRIVATE-PREVIEW-SECRET' },
  propertyLabels: { public: 'public', secret: 'critical' },
  templateId: null,
  templateType: null,
  deletedAt: null,
  contractTypeId: null,
  templateHash: null,
  ignoredTemplateHash: null,
});
const preview = (objectId: string, onNavigate = vi.fn()) => (
  <PagePaneObjectPreview
    objectId={objectId}
    collectionLabel="Identity"
    index={0}
    total={2}
    onNavigate={onNavigate}
  />
);

function FramedPreview({ objectId }: { objectId: string }) {
  const anchor = useRef<HTMLButtonElement>(null);
  const card = useRef<HTMLDivElement>(null);
  return (
    <>
      <button ref={anchor}>Preview anchor</button>
      <PagePanePreviewFrame
        anchor={anchor}
        cardRef={card}
        id="test-preview"
        label="Object details"
        onClose={vi.fn()}
        onDismiss={vi.fn()}
        onMouseEnter={vi.fn()}
        onMouseLeave={vi.fn()}
      >
        {preview(objectId)}
      </PagePanePreviewFrame>
    </>
  );
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  useAuthStore.setState({
    currentAccount: { id: 'preview-account' } as never,
    isAuthenticated: true,
  });
  setRequestSession('preview-account');
});
afterEach(() => setRequestSession(null));

it('uses the ruler mask, exposes public fields only, and opens the editor', async () => {
  vi.mocked(invoke).mockResolvedValue(object('a'));
  const navigate = vi.fn();
  render(preview('a', navigate));
  expect(await screen.findByText('Public preview')).toBeInTheDocument();
  expect(screen.getByText('••••••••')).toBeInTheDocument();
  expect(document.body.innerHTML).not.toContain('PRIVATE-PREVIEW-SECRET');
  fireEvent.click(screen.getByRole('button', { name: 'page_pane_edit_object' }));
  expect(navigate).toHaveBeenCalledOnce();
});

it.each(['success', 'failure'])(
  'shows only the completed %s card, never its loading frame',
  async (outcome) => {
    let finish!: () => void;
    vi.mocked(invoke).mockImplementation(
      () =>
        new Promise((resolve, reject) => {
          finish = () =>
            outcome === 'success'
              ? resolve(object('a'))
              : reject(new Error('Public fixture failure'));
        }),
    );
    render(<FramedPreview objectId="a" />);
    await waitFor(() => expect(finish).toBeDefined());
    const frame = screen.getByRole('region', { hidden: true });
    expect(frame).not.toBeVisible();
    expect(screen.getByText('page_pane_loading')).not.toBeVisible();
    await act(async () => finish());
    await waitFor(() => expect(frame).toBeVisible());
    expect(screen.queryByText('page_pane_loading')).not.toBeInTheDocument();
    if (outcome === 'success') {
      expect(screen.getByText('Public preview')).toBeVisible();
      expect(frame).not.toHaveTextContent('PRIVATE-PREVIEW-SECRET');
    } else expect(screen.getByText('page_pane_preview_unavailable')).toBeVisible();
  },
);

it.each(['account', 'deleted', 'id'])(
  'rejects a %s-mismatched object response',
  async (mismatch) => {
    const wire = object('a');
    if (mismatch === 'account') wire.accountId = 'other-account';
    if (mismatch === 'deleted') wire.deletedAt = '2026-10-20';
    if (mismatch === 'id') wire.id = 'other-object';
    vi.mocked(invoke).mockResolvedValue(wire);
    render(preview('a'));
    expect(await screen.findByText('page_pane_preview_unavailable')).toBeInTheDocument();
    expect(document.body.innerHTML).not.toContain('Public preview');
  },
);

it('does not replace a new hover card with a late previous response', async () => {
  let finishOld!: (value: ObjectData) => void;
  vi.mocked(invoke).mockImplementation(async (_command, args) => {
    if ((args as { objectId: string }).objectId === 'a')
      return await new Promise<ObjectData>((resolve) => {
        finishOld = resolve;
      });
    return object('b');
  });
  const { rerender } = render(<FramedPreview objectId="a" />);
  await waitFor(() => expect(finishOld).toBeDefined());
  rerender(<FramedPreview objectId="b" />);
  expect(await screen.findByRole('heading', { name: 'Object b' })).toBeInTheDocument();
  await act(async () => finishOld(object('a')));
  expect(screen.getByRole('heading', { name: 'Object b' })).toBeInTheDocument();
  expect(screen.queryByRole('heading', { name: 'Object a' })).not.toBeInTheDocument();
});

it('clears already loaded details on account lock', async () => {
  vi.mocked(invoke).mockResolvedValue(object('a'));
  render(preview('a'));
  expect(await screen.findByText('Public preview')).toBeInTheDocument();
  act(() => {
    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
    setRequestSession(null);
  });
  expect(document.body.innerHTML).not.toContain('Public preview');
  expect(screen.queryByRole('heading')).not.toBeInTheDocument();
});
