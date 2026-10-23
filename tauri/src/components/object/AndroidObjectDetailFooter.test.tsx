import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { CircleHelp } from 'lucide-react';
import { AndroidObjectDetailFooter } from './AndroidObjectDetailFooter';
import type { GuidePage } from '@/components/guide/PageGuide';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';

const mocks = vi.hoisted(() => ({ navigate: vi.fn(), onError: vi.fn(), onSuccess: vi.fn() }));
vi.mock('@/hooks/useToastError', () => ({ useToastError: () => mocks }));

vi.mock('react-router-dom', () => ({ useNavigate: () => mocks.navigate }));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock('@/components/android/AndroidSheet', () => ({
  AndroidSheet: ({
    title,
    onClose,
    children,
  }: {
    title: string;
    onClose: () => void;
    children: React.ReactNode;
  }) => (
    <div role="dialog" aria-label={title}>
      <button type="button" onClick={onClose}>
        Close sheet
      </button>
      {children}
    </div>
  ),
}));

const guidePages: GuidePage[] = [
  {
    icon: CircleHelp,
    title: 'Overview',
    steps: [{ icon: CircleHelp, title: 'Find fields', description: 'Inspect the object' }],
    helpLinks: [{ title: 'Help article', description: 'More detail', href: '/help/objects' }],
  },
  {
    icon: CircleHelp,
    title: 'Privacy',
    steps: [],
    helpLinks: [],
  },
];

function renderFooter(options?: {
  attachmentCount?: number;
  guidePages?: GuidePage[];
  onEdit?: () => void;
  editLocked?: boolean;
}) {
  const onHistory = vi.fn();
  const onAttachments = vi.fn();
  const onDelete = vi.fn();
  const view = render(
    <AndroidObjectDetailFooter
      objectId="object-a"
      objectName="Passport"
      attachmentCount={options?.attachmentCount}
      guidePages={options?.guidePages ?? []}
      onHistory={onHistory}
      onAttachments={onAttachments}
      onEdit={options?.onEdit}
      onDelete={onDelete}
      editLocked={options?.editLocked}
    />,
  );
  return { ...view, onHistory, onAttachments, onDelete };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(invoke).mockReset();
  useAuthStore.setState({
    currentAccount: { id: 'account', name: 'Public' },
    isAuthenticated: true,
  });
  useObjectStore.setState({ currentObjectCache: {}, objects: [] });
});

function record(locked: boolean) {
  return {
    id: 'object-a',
    accountId: 'account',
    name: 'Passport',
    typeId: 'identity',
    properties: { __editLocked: locked },
    sensitivityLevel: 'public',
    createdAt: '2026-10-23',
    updatedAt: '2026-10-23',
  };
}

describe('RF-1022 Android object detail footer', () => {
  it('locks and unlocks from More actions without editing or closing the object detail', async () => {
    vi.mocked(invoke).mockImplementation(async (_command, args) =>
      record((args as { locked: boolean }).locked),
    );
    const onEdit = vi.fn();
    const { onHistory, onDelete } = renderFooter({ onEdit, editLocked: true });
    const trigger = screen.getByRole('button', { name: 'material.object_actions' });
    fireEvent.click(trigger);
    const menu = screen.getByRole('dialog', { name: 'more_actions' });
    expect(within(menu).getByRole('button', { name: 'delete' })).toBeDisabled();
    fireEvent.click(within(menu).getByRole('button', { name: 'object_unlock' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(invoke).toHaveBeenLastCalledWith('object_set_locked', {
      objectId: 'object-a',
      locked: false,
    });
    fireEvent.click(trigger);
    expect(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }),
    ).toBeEnabled();
    fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'object_lock' }),
    );
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(invoke).toHaveBeenLastCalledWith('object_set_locked', {
      objectId: 'object-a',
      locked: true,
    });
    fireEvent.click(trigger);
    expect(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }),
    ).toBeDisabled();
    expect(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'object_unlock' }),
    ).toBeEnabled();
    expect(onEdit).not.toHaveBeenCalled();
    expect(onHistory).not.toHaveBeenCalled();
    expect(onDelete).not.toHaveBeenCalled();
  });

  it('blocks repeat submissions and conflicting actions, retaining the menu for retry after failure', async () => {
    let reject!: (reason: Error) => void;
    vi.mocked(invoke).mockReturnValueOnce(
      new Promise((_resolve, fail) => {
        reject = fail;
      }),
    );
    renderFooter({ onEdit: vi.fn() });
    fireEvent.click(screen.getByRole('button', { name: 'material.object_actions' }));
    const menu = screen.getByRole('dialog');
    const lock = within(menu).getByRole('button', { name: 'object_lock' });
    fireEvent.click(lock);
    fireEvent.click(lock);
    expect(invoke).toHaveBeenCalledOnce();
    expect(lock).toBeDisabled();
    expect(within(menu).getByRole('button', { name: 'delete' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'edit' })).toBeDisabled();
    await act(async () => reject(new Error('Public failure')));
    expect(mocks.onError).toHaveBeenCalledOnce();
    expect(lock).toBeEnabled();
    expect(mocks.onSuccess).not.toHaveBeenCalled();
    vi.mocked(invoke).mockResolvedValueOnce(record(true));
    fireEvent.click(lock);
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(mocks.onSuccess).toHaveBeenCalledWith('object_locked_success');
  });

  it('opens attachments and edit directly, and caps the visible attachment count', () => {
    const onEdit = vi.fn();
    const { onAttachments } = renderFooter({ attachmentCount: 123, onEdit });

    fireEvent.click(screen.getByRole('button', { name: 'material.attachments_with_count' }));
    fireEvent.click(screen.getByRole('button', { name: 'edit' }));
    expect(onAttachments).toHaveBeenCalledOnce();
    expect(onEdit).toHaveBeenCalledOnce();
    expect(screen.getByText('99+')).toBeInTheDocument();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('closes the menu before invoking history or delete', () => {
    const { onHistory, onDelete } = renderFooter();
    const actions = screen.getByRole('button', { name: 'material.object_actions' });
    expect(screen.queryByRole('button', { name: 'edit' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'attachments' })).toBeInTheDocument();

    fireEvent.click(actions);
    expect(actions).toHaveAttribute('aria-expanded', 'true');
    const menu = screen.getByRole('dialog', { name: 'more_actions' });
    expect(within(menu).queryByRole('button', { name: 'guide' })).not.toBeInTheDocument();
    fireEvent.click(within(menu).getByRole('button', { name: 'history' }));
    expect(onHistory).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(actions).toHaveAttribute('aria-expanded', 'false');

    fireEvent.click(actions);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }));
    expect(onDelete).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('focuses the guide and closes it before navigating to a help link', () => {
    renderFooter({ attachmentCount: 3, guidePages });
    const actions = screen.getByRole('button', { name: 'material.object_actions' });
    expect(screen.getByText('3')).toBeInTheDocument();
    fireEvent.click(actions);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'guide' }));

    const guide = screen.getByRole('dialog', { name: 'Overview' });
    expect(within(guide).getByText('Find fields')).toBeInTheDocument();
    expect(within(guide).getByText('Privacy')).toBeInTheDocument();
    expect(document.activeElement).toHaveClass('android-object-detail-guide');
    fireEvent.click(within(guide).getByRole('button', { name: /Help article/ }));
    expect(mocks.navigate).toHaveBeenCalledWith('/help/objects');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(actions).toHaveAttribute('aria-expanded', 'false');
  });

  it('restores the collapsed state when the sheet closes without an action', () => {
    renderFooter({ guidePages });
    const actions = screen.getByRole('button', { name: 'material.object_actions' });
    fireEvent.click(actions);
    fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Close sheet' }),
    );
    expect(actions).toHaveAttribute('aria-expanded', 'false');
    expect(mocks.navigate).not.toHaveBeenCalled();
  });
});
