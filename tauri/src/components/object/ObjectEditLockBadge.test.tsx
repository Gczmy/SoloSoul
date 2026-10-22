import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore, type ObjectData } from '@/stores/objectStore';
import { ObjectEditLockBadge } from './ObjectEditLockBadge';
import { AttachmentActions } from '@/components/attachment/AttachmentActions';
import { objectEditLocked } from '@/lib/objectEditLock';
import { isAndroidSync } from '@/lib/platform';
import { AttachmentMetaEditDialog } from '@/components/attachment/AttachmentMetaEditDialog';
function statusRecord(accountId: string, locked: boolean): ObjectData {
  return {
    id: 'A',
    accountId,
    name: 'Public object',
    typeId: 'identity',
    sensitivityLevel: 'public',
    createdAt: '2026-10-23',
    updatedAt: '2026-10-23',
    properties: { __editLocked: locked },
  };
}
vi.mock('@/lib/platform', async (original) => ({
  ...(await original<typeof import('@/lib/platform')>()),
  isAndroidSync: vi.fn(() => false),
}));

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(isAndroidSync).mockReturnValue(false);
  useAuthStore.setState({
    currentAccount: { id: 'account', name: 'Public' },
    isAuthenticated: true,
  });
  useObjectStore.setState({ currentObjectCache: {}, objects: [] });
});

it('badges across stale summaries change together without fetching or hiding content', () => {
  render(
    <>
      <ObjectEditLockBadge objectId="A" properties={{}} />
      <ObjectEditLockBadge objectId="A" properties={{}} compact />
    </>,
  );
  expect(screen.getAllByLabelText('object_editable')).toHaveLength(2);
  act(() => useObjectStore.setState({ currentObjectCache: { A: statusRecord('account', true) } }));
  expect(screen.getAllByLabelText('object_locked')).toHaveLength(2);
  act(() => useObjectStore.setState({ currentObjectCache: { A: statusRecord('account', false) } }));
  expect(screen.getAllByLabelText('object_editable')).toHaveLength(2);
});

it('does not use cached status from a different account', () => {
  useObjectStore.setState({ currentObjectCache: { A: statusRecord('other', true) } });
  render(<ObjectEditLockBadge objectId="A" properties={{}} />);
  expect(screen.getByLabelText('object_editable')).toBeInTheDocument();
});

it('matches persisted fail-closed marker semantics', () => {
  expect(objectEditLocked({})).toBe(false);
  expect(objectEditLocked({ __editLocked: false })).toBe(false);
  for (const value of [true, null, 'false', 0])
    expect(objectEditLocked({ __editLocked: value })).toBe(true);
});

it('attachment metadata stays readable and keyboard tag removal is disabled while locked', () => {
  useObjectStore.setState({ currentObjectCache: { A: statusRecord('account', true) } });
  const onSaved = vi.fn();
  render(
    <AttachmentMetaEditDialog
      item={{
        objectId: 'A',
        id: 'attachment',
        fileName: 'Public attachment',
        description: 'Public description',
        tags: ['Public tag'],
      }}
      onClose={vi.fn()}
      onSaved={onSaved}
    />,
  );
  expect(screen.getByDisplayValue('Public attachment')).toHaveAttribute('readonly');
  expect(screen.getByDisplayValue('Public description')).toHaveAttribute('readonly');
  fireEvent.keyDown(screen.getByPlaceholderText('Type a tag and press Enter'), {
    key: 'Backspace',
  });
  expect(screen.getByText('Public tag')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'common:save' })).toBeDisabled();
  expect(onSaved).not.toHaveBeenCalled();
});

it.each([false, true])(
  'locked attachment actions keep viewing and downloading available (Android=%s)',
  (android) => {
    vi.mocked(isAndroidSync).mockReturnValue(android);
    const preview = vi.fn();
    const remove = vi.fn();
    render(
      <AttachmentActions
        fileName="public.txt"
        showTrash={false}
        readOnly
        onPreview={preview}
        onDownload={vi.fn()}
        onShare={vi.fn()}
        onEditMeta={vi.fn()}
        onSoftDelete={remove}
        onRestore={vi.fn()}
        onPermanentDelete={vi.fn()}
      />,
    );
    if (android) fireEvent.click(screen.getByRole('button', { name: 'attachment_actions_for' }));
    const deleteButton = screen.getByRole('button', { name: android ? 'delete' : 'common:delete' });
    expect(deleteButton).toBeDisabled();
    fireEvent.click(deleteButton);
    expect(remove).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: android ? 'preview' : 'common:preview' }));
    expect(preview).toHaveBeenCalledOnce();
  },
);
