import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore, type ObjectData } from '@/stores/objectStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';
import { isAndroidSync } from '@/lib/platform';
import { ObjectEditorPage } from './ObjectEditorPage';

const { route, navigate, t } = vi.hoisted(() => ({
  route: { objectId: 'A' as string | undefined },
  navigate: vi.fn(),
  t: (key: string) => key,
}));
vi.mock('react-i18next', () => ({ useTranslation: () => ({ t, i18n: { language: 'en-US' } }) }));
vi.mock('react-router-dom', async (original) => ({
  ...(await original<typeof import('react-router-dom')>()),
  useLocation: () => ({ pathname: '/editor/A', search: '', hash: '', state: null, key: 'test' }),
  useParams: () => route,
  useNavigate: () => navigate,
  useSearchParams: () => [new URLSearchParams()],
}));
vi.mock('@/lib/platform', async (original) => ({
  ...(await original<typeof import('@/lib/platform')>()),
  isAndroidSync: vi.fn(() => false),
}));
vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({
    children,
    primaryActions,
  }: {
    children: React.ReactNode;
    primaryActions?: React.ReactNode;
  }) => (
    <main>
      {primaryActions}
      {children}
    </main>
  ),
}));

function fixture(id = 'A', locked = true): ObjectData {
  return {
    id,
    accountId: 'account',
    name: `Object ${id}`,
    typeId: 'identity',
    templateId: 'template',
    properties: { title: 'Public text', choice: 'A', date: '2026-10-23', __editLocked: locked },
    sensitivityLevel: 'public',
    createdAt: '2026-10-23',
    updatedAt: '2026-10-23',
  };
}
let record = fixture();
beforeEach(() => {
  vi.clearAllMocks();
  route.objectId = 'A';
  record = fixture();
  useAuthStore.setState({
    currentAccount: { id: 'account', name: 'Public fixture' },
    isAuthenticated: true,
  });
  useObjectStore.setState({ currentObjectCache: {}, objects: [], isLoading: false });
  useNativeWindowStore.setState({ isWindows: false, isMacOS: false });
  vi.mocked(isAndroidSync).mockReturnValue(false);
  useTemplateStore.setState({
    loadTemplates: vi.fn().mockResolvedValue(undefined),
    templates: [
      {
        id: 'template',
        accountId: 'account',
        name: 'Identity',
        category: 'identity',
        createdAt: '',
        updatedAt: '',
        properties: [
          { id: 'title', name: 'Title', type: 'text' },
          { id: 'choice', name: 'Choice', type: 'select', options: ['A', 'B'] },
          { id: 'date', name: 'Date', type: 'date' },
        ],
      },
    ],
  });
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    const input = args as {
      objectId: string;
      locked: boolean;
      input: { name: string; properties: Record<string, unknown> };
    };
    if (command === 'object_get')
      return input.objectId === 'A' ? record : fixture(input.objectId, false);
    if (command === 'object_field_suggestions')
      return [
        {
          objectId: 'source',
          objectName: 'Source',
          fieldKey: 'title',
          fieldName: 'Title',
          sensitivityLevel: 'public',
          value: 'Suggested',
        },
      ];
    if (command === 'biometric_available') return { available: false };
    if (command === 'account_list') return [];
    if (command === 'object_update') {
      record = {
        ...record,
        ...input.input,
        properties: { ...record.properties, ...input.input.properties },
      };
      return record;
    }
    if (command === 'object_set_locked') {
      record = { ...record, properties: { ...record.properties, __editLocked: input.locked } };
      return record;
    }
    return undefined;
  });
});

describe('object edit protection', () => {
  it.each(
    ['windows', 'macos', 'android'].flatMap((platform) =>
      [true, false].map((templateAvailable) => ({ platform, templateAvailable })),
    ),
  )(
    '$platform, template available=$templateAvailable: locked fields remain readable without recommendations',
    async ({ platform, templateAvailable }) => {
      if (!templateAvailable) {
        useTemplateStore.setState({ templates: [] });
        record.properties.__fields = {
          title: { name: 'Title', type: 'text' },
          choice: { name: 'Choice', type: 'select', options: ['A', 'B'] },
          date: { name: 'Date', type: 'date' },
        };
      }
      useNativeWindowStore.setState({
        isWindows: platform === 'windows',
        isMacOS: platform === 'macos',
      });
      vi.mocked(isAndroidSync).mockReturnValue(platform === 'android');
      render(<ObjectEditorPage />);
      await waitFor(() =>
        expect(screen.getByRole('textbox', { name: /^Title/ })).toHaveValue('Public text'),
      );
      expect(screen.getByRole('textbox', { name: /^Title/ })).toHaveAttribute('readonly');
      expect(screen.getByRole('textbox', { name: /^Title/ })).not.toBeDisabled();
      expect(screen.queryByRole('button', { name: 'common:cancel' })).toBeNull();
      expect(screen.queryByRole('button', { name: 'common:save' })).toBeNull();
      expect(screen.getByRole('button', { name: 'common:object_unlock' })).toBeEnabled();
      expect(screen.queryByTestId('field-suggestions')).toBeNull();
      if (platform !== 'android')
        expect(screen.getByRole('combobox', { name: /^Choice/ })).toBeDisabled();
      fireEvent.click(screen.getByRole('button', { name: 'common:object_unlock' }));
      await waitFor(() => expect(screen.getByTestId('field-suggestion-fill')).toBeEnabled());
      fireEvent.click(screen.getByRole('button', { name: 'common:object_lock' }));
      await waitFor(() =>
        expect(screen.getByRole('button', { name: 'common:object_unlock' })).toBeEnabled(),
      );
      expect(screen.queryByTestId('field-suggestions')).toBeNull();
    },
  );

  it('unlocks, saves dirty contents before locking, and stays on the current editor', async () => {
    render(<ObjectEditorPage />);
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'common:object_unlock' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'common:object_unlock' }));
    await waitFor(() =>
      expect(screen.getByRole('textbox', { name: /^Title/ })).not.toHaveAttribute('readonly'),
    );
    expect(screen.getByRole('button', { name: 'common:cancel' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'common:save' })).toBeEnabled();
    fireEvent.change(screen.getByRole('textbox', { name: /^Title/ }), {
      target: { value: 'Edited public text' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'common:object_lock' }));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'common:object_unlock' })).toBeEnabled(),
    );
    expect(record.properties.title).toBe('Edited public text');
    expect(screen.getByRole('textbox', { name: /^Title/ })).toHaveAttribute('readonly');
    expect(
      vi
        .mocked(invoke)
        .mock.calls.filter(([cmd]) => cmd === 'object_update' || cmd === 'object_set_locked')
        .map(([cmd]) => cmd),
    ).toEqual(['object_set_locked', 'object_update', 'object_set_locked']);
    expect(navigate).not.toHaveBeenCalled();
  });

  it('shows the accepted name when locking saves an empty-name draft', async () => {
    record = fixture('A', false);
    render(<ObjectEditorPage />);
    await waitFor(() =>
      expect(screen.getByLabelText('common:object_name')).toHaveValue('Object A'),
    );
    fireEvent.change(screen.getByLabelText('common:object_name'), { target: { value: '' } });
    fireEvent.click(screen.getByRole('button', { name: 'common:object_lock' }));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'common:object_unlock' })).toBeEnabled(),
    );
    expect(screen.getByLabelText('common:object_name')).toHaveValue(
      'common:object_name_placeholder',
    );
  });

  it('preserves a dirty draft and does not lock after saving fails', async () => {
    record = fixture('A', false);
    const original = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation(async (cmd, args) => {
      if (cmd === 'object_update') throw 'OBJECT_EDIT_LOCKED';
      return original(cmd, args);
    });
    render(<ObjectEditorPage />);
    await waitFor(() =>
      expect(screen.getByRole('textbox', { name: /^Title/ })).toHaveValue('Public text'),
    );
    fireEvent.change(screen.getByRole('textbox', { name: /^Title/ }), {
      target: { value: 'Unsaved draft' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'common:object_lock' }));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'common:object_lock' })).toBeEnabled(),
    );
    expect(screen.getByRole('textbox', { name: /^Title/ })).toHaveValue('Unsaved draft');
    expect(invoke).not.toHaveBeenCalledWith('object_set_locked', expect.anything());
    expect(navigate).not.toHaveBeenCalled();
  });

  it('does not publish an old lock response after switching objects', async () => {
    let finish!: (value: ObjectData) => void;
    const original = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation(async (cmd, args) =>
      cmd === 'object_set_locked'
        ? new Promise<ObjectData>((resolve) => {
            finish = resolve;
          })
        : original(cmd, args),
    );
    const view = render(<ObjectEditorPage />);
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'common:object_unlock' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'common:object_unlock' }));
    await waitFor(() => expect(finish).toBeDefined());
    route.objectId = 'B';
    view.rerender(<ObjectEditorPage />);
    await waitFor(() =>
      expect(screen.getByLabelText('common:object_name')).toHaveValue('Object B'),
    );
    await act(async () => finish(fixture('A', false)));
    expect(screen.getByLabelText('common:object_name')).toHaveValue('Object B');
    expect(screen.getByRole('button', { name: 'common:object_lock' })).toBeEnabled();
  });

  it('discards a lock response after the account session ends', async () => {
    let finish!: (value: ObjectData) => void;
    vi.mocked(invoke).mockImplementation(
      async () =>
        new Promise<ObjectData>((resolve) => {
          finish = resolve;
        }),
    );
    const pending = useObjectStore.getState().setObjectLocked('A', true);
    await waitFor(() => expect(finish).toBeDefined());
    useAuthStore.setState({ isAuthenticated: false });
    finish(fixture());
    await expect(pending).rejects.toBeDefined();
    expect(useObjectStore.getState().currentObjectCache).toEqual({});
  });
});
