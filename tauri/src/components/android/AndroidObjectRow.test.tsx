import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { IdCard } from 'lucide-react';
import { initReactI18next } from 'react-i18next';
import i18n from '@/lib/i18n';
import enCommon from '@/locales/en-US/common.json';
import enEditor from '@/locales/en-US/editor.json';
import { useObjectStore, type ObjectSummary } from '@/stores/objectStore';
import { useAuthStore } from '@/stores/authStore';
import { AndroidObjectRow } from './AndroidObjectRow';

vi.unmock('react-i18next');
const notices = vi.hoisted(() => ({ onError: vi.fn(), onSuccess: vi.fn() }));
vi.mock('@/hooks/useToastError', () => ({ useToastError: () => notices }));

const obj: ObjectSummary = {
  id: 'object-a',
  name: 'Passport',
  typeId: 'identity',
  sensitivityLevel: 'internal',
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-02T00:00:00Z',
};

const callbacks = {
  onClick: vi.fn(),
  onHistory: vi.fn(),
  onAttachments: vi.fn(),
  onEdit: vi.fn(),
  onDelete: vi.fn(),
  onSync: vi.fn(),
  onDismissSync: vi.fn(),
};

function renderRow(
  options: { needsSync?: boolean; snapshotCount?: number; attachmentCount?: number } = {},
) {
  vi.clearAllMocks();
  return render(
    <AndroidObjectRow
      obj={obj}
      Icon={IdCard}
      collectionLabel="Identity"
      templateName="Passport template"
      sensitivities={['internal', 'critical']}
      needsSync={options.needsSync ?? false}
      snapshotCount={options.snapshotCount}
      attachmentCount={options.attachmentCount}
      {...callbacks}
    />,
  );
}

beforeAll(async () => {
  await i18n.use(initReactI18next).init({
    lng: 'en-US',
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'editor'],
    resources: { 'en-US': { common: enCommon, editor: enEditor } },
    interpolation: { escapeValue: false },
  });
});

beforeEach(() => {
  vi.clearAllMocks();
  useAuthStore.setState({
    currentAccount: { id: 'account', name: 'Public' },
    isAuthenticated: true,
  });
  useObjectStore.setState({ currentObjectCache: {}, objects: [obj] });
});

function lockedRecord(locked: boolean) {
  return { ...obj, accountId: 'account', properties: { __editLocked: locked } };
}

describe('AndroidObjectRow actions', () => {
  it('locks and unlocks from the action sheet, updating the row and delete protection', async () => {
    vi.mocked(invoke).mockImplementation(async (_command, args) =>
      lockedRecord((args as { locked: boolean }).locked),
    );
    renderRow();
    const trigger = screen.getByRole('button', { name: 'Actions for Passport' });
    fireEvent.click(trigger);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Lock' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(invoke).toHaveBeenCalledWith('object_set_locked', { objectId: obj.id, locked: true });
    expect(screen.getByLabelText('Locked')).toBeVisible();
    fireEvent.click(trigger);
    expect(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Delete' }),
    ).toBeDisabled();
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Unlock' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(invoke).toHaveBeenLastCalledWith('object_set_locked', {
      objectId: obj.id,
      locked: false,
    });
    expect(screen.getByLabelText('Editable')).toBeVisible();
    fireEvent.click(trigger);
    expect(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Delete' }),
    ).toBeEnabled();
    expect(notices.onSuccess.mock.calls).toEqual([['Object locked'], ['Object unlocked']]);
    expect(callbacks.onEdit).not.toHaveBeenCalled();
  });

  it('blocks repeat lock requests and conflicting actions until the backend confirms the change', async () => {
    let finish!: (value: ReturnType<typeof lockedRecord>) => void;
    vi.mocked(invoke).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    renderRow();
    fireEvent.click(screen.getByRole('button', { name: 'Actions for Passport' }));
    const dialog = screen.getByRole('dialog');
    const lock = within(dialog).getByRole('button', { name: 'Lock' });
    fireEvent.click(lock);
    fireEvent.click(lock);
    expect(invoke).toHaveBeenCalledOnce();
    expect(lock).toBeDisabled();
    expect(within(dialog).getByRole('button', { name: 'Delete' })).toBeDisabled();
    expect(within(dialog).getByRole('button', { name: 'Edit' })).toBeDisabled();
    expect(screen.getByLabelText('Editable')).toBeVisible();
    await act(async () => finish(lockedRecord(true)));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.getByLabelText('Locked')).toBeVisible();
  });

  it('keeps the current status and sheet available for retry when locking fails', async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error('Public failure'));
    renderRow();
    fireEvent.click(screen.getByRole('button', { name: 'Actions for Passport' }));
    const lock = within(screen.getByRole('dialog')).getByRole('button', {
      name: 'Lock',
    });
    fireEvent.click(lock);
    await waitFor(() => expect(notices.onError).toHaveBeenCalledOnce());
    expect(lock).toBeEnabled();
    expect(screen.getByLabelText('Editable')).toBeVisible();
    expect(notices.onSuccess).not.toHaveBeenCalled();
  });

  it('does not show stale notices or publish a lock result after switching accounts', async () => {
    let finish!: (value: ReturnType<typeof lockedRecord>) => void;
    vi.mocked(invoke).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    const view = renderRow();
    fireEvent.click(screen.getByRole('button', { name: 'Actions for Passport' }));
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Lock' }));
    act(() => useAuthStore.setState({ currentAccount: { id: 'other', name: 'Other' } }));
    view.unmount();
    await act(async () => finish(lockedRecord(true)));
    expect(useObjectStore.getState().currentObjectCache[obj.id]).toBeUndefined();
    expect(notices.onSuccess).not.toHaveBeenCalled();
    expect(notices.onError).not.toHaveBeenCalled();
  });

  it('opens the object from its main row and restores action trigger focus after editing', () => {
    renderRow();
    fireEvent.click(screen.getByRole('button', { name: /Passport.*Identity/ }));
    expect(callbacks.onClick).toHaveBeenCalledOnce();

    const trigger = screen.getByRole('button', { name: 'Actions for Passport' });
    fireEvent.click(trigger);
    const dialog = screen.getByRole('dialog', { name: 'Passport' });
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Edit' }));
    expect(callbacks.onEdit).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(trigger).toHaveFocus();
  });

  it('keeps zero history count visible and routes history, attachments, and delete separately', () => {
    renderRow({ snapshotCount: 0, attachmentCount: 2 });
    const trigger = screen.getByRole('button', { name: 'Actions for Passport' });

    fireEvent.click(trigger);
    fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'History · 0' }),
    );
    expect(callbacks.onHistory).toHaveBeenCalledOnce();
    fireEvent.click(trigger);
    fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Attachments · 2' }),
    );
    expect(callbacks.onAttachments).toHaveBeenCalledOnce();
    fireEvent.click(trigger);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Delete' }));
    expect(callbacks.onDelete).toHaveBeenCalledOnce();
    expect(callbacks.onEdit).not.toHaveBeenCalled();
  });

  it('offers template update and skip only when synchronization is needed', () => {
    const view = renderRow();
    const trigger = screen.getByRole('button', { name: 'Actions for Passport' });
    fireEvent.click(trigger);
    expect(
      within(screen.getByRole('dialog')).queryByRole('button', { name: 'Skip template update' }),
    ).toBeNull();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();

    view.rerender(
      <AndroidObjectRow
        obj={obj}
        Icon={IdCard}
        collectionLabel="Identity"
        templateName="Passport template"
        sensitivities={['internal', 'critical']}
        needsSync
        {...callbacks}
      />,
    );
    fireEvent.click(trigger);
    const dialog = screen.getByRole('dialog');
    fireEvent.click(
      within(dialog).getByRole('button', { name: /template for this object has been updated/ }),
    );
    expect(callbacks.onSync).toHaveBeenCalledOnce();
    fireEvent.click(trigger);
    fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Skip template update' }),
    );
    expect(callbacks.onDismissSync).toHaveBeenCalledOnce();
  });
});
