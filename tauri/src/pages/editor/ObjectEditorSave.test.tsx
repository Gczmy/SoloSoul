import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore, type ObjectData } from '@/stores/objectStore';
import { useTemplateStore } from '@/stores/templateStore';
import { isMobilePlatformSync } from '@/lib/platform';
import { objectEditorHasChanges, objectFormValues } from './objectEditorDraft';
import { useObjectEditorPage } from './useObjectEditorPage';

const { route, navigate, t } = vi.hoisted(() => ({
  route: { objectId: 'A' as string | undefined },
  navigate: vi.fn(),
  t: (key: string) => key,
}));
vi.mock('react-router-dom', () => ({
  useParams: () => route,
  useNavigate: () => navigate,
  useSearchParams: () => [new URLSearchParams('section=identity')],
}));
vi.mock('react-i18next', () => ({ useTranslation: () => ({ t }) }));
vi.mock('@/lib/platform', async (original) => ({
  ...(await original<typeof import('@/lib/platform')>()),
  isMobilePlatformSync: vi.fn(() => false),
}));

let record: ObjectData;
beforeEach(() => {
  vi.clearAllMocks();
  route.objectId = 'A';
  vi.mocked(isMobilePlatformSync).mockReturnValue(false);
  record = {
    id: 'A',
    accountId: 'account',
    name: 'Public A',
    typeId: 'identity',
    templateId: 'template',
    properties: { title: 'Original', number: 42, enabled: true, __editLocked: false },
    sensitivityLevel: 'public',
    createdAt: '',
    updatedAt: '',
  };
  useAuthStore.setState({
    currentAccount: { id: 'account', name: 'Public fixture' },
    isAuthenticated: true,
  });
  useObjectStore.setState({ currentObjectCache: {}, objects: [], isLoading: false });
  useTemplateStore.setState({
    loadTemplates: vi.fn().mockResolvedValue(undefined),
    templates: [
      {
        id: 'template',
        accountId: 'account',
        name: 'Public template',
        category: 'identity',
        createdAt: '',
        updatedAt: '',
        properties: [{ id: 'title', name: 'Title', type: 'text' }],
      },
    ],
  });
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    const input = args as {
      objectId: string;
      locked: boolean;
      input: { name: string; properties: Record<string, unknown> };
    };
    if (command === 'object_get') return { ...record, id: input.objectId };
    if (command === 'object_field_suggestions') return [];
    if (command === 'object_set_locked') {
      record = { ...record, properties: { ...record.properties, __editLocked: input.locked } };
      return record;
    }
    if (command === 'object_update' || command === 'object_create') {
      record = {
        ...record,
        ...input.input,
        properties: { ...record.properties, ...input.input.properties },
        updatedAt: 'saved',
      };
      return record;
    }
    return undefined;
  });
});
async function editor() {
  const hook = renderHook(useObjectEditorPage);
  await waitFor(() =>
    expect(
      hook.result.current.isObjectReady &&
        (route.objectId ? hook.result.current.currentObject : hook.result.current.selectedType),
    ).toBeTruthy(),
  );
  return hook;
}
const writes = () => vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === 'object_update');

describe('desktop editor save', () => {
  it('refreshes history for dirty save-before-lock, but not for lock-only changes', async () => {
    const hook = await editor();
    await act(async () => hook.result.current.handleToggleEditLock());
    expect(hook.result.current.historyRevision).toBe(0);
    await act(async () => hook.result.current.handleToggleEditLock());
    expect(hook.result.current.historyRevision).toBe(0);
    await act(async () => hook.result.current.handleFieldChange('title', 'Changed before locking'));
    await act(async () => hook.result.current.handleToggleEditLock());
    expect(hook.result.current.historyRevision).toBe(1);
    expect(hook.result.current.isEditLocked).toBe(true);
    expect(writes()).toHaveLength(1);
  });
  it('stays on the editor, accepts the persisted baseline, and does not write twice', async () => {
    const hook = await editor();
    await act(async () => hook.result.current.handleFieldChange('title', 'Changed'));
    await act(async () => hook.result.current.handleSave());
    expect(navigate).not.toHaveBeenCalled();
    expect(hook.result.current.currentObject?.updatedAt).toBe('saved');
    expect(hook.result.current.values.title).toBe('Changed');
    expect(hook.result.current.isSaving).toBe(false);
    expect(hook.result.current.historyRevision).toBe(1);
    await act(async () => hook.result.current.handleSave());
    expect(writes()).toHaveLength(1);
    expect(hook.result.current.historyRevision).toBe(1);
    expect(vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === 'object_get')).toHaveLength(1);
  });
  it('skips untouched and reverted drafts, including numeric and boolean input normalization', async () => {
    const hook = await editor();
    expect(hook.result.current.values).toMatchObject({ number: '42', enabled: 'true' });
    await act(async () => hook.result.current.handleSave());
    await act(async () => hook.result.current.handleFieldChange('title', 'Changed'));
    await act(async () => hook.result.current.handleFieldChange('title', 'Original'));
    await act(async () => hook.result.current.handleSave());
    expect(writes()).toHaveLength(0);
    expect(hook.result.current.historyRevision).toBe(0);
    expect(navigate).not.toHaveBeenCalled();
  });
  it('accepts the empty-name fallback as the next baseline', async () => {
    const hook = await editor();
    await act(async () => hook.result.current.setName(''));
    await act(async () => hook.result.current.handleSave());
    expect(hook.result.current.name).toBe('common:object_name_placeholder');
    await act(async () => hook.result.current.handleSave());
    expect(writes()).toHaveLength(1);
  });
  it('keeps a failed draft editable and retries without accepting a false baseline', async () => {
    const hook = await editor();
    await act(async () => hook.result.current.handleFieldChange('title', 'Changed'));
    vi.mocked(invoke).mockRejectedValueOnce(new Error('Public fixture failure'));
    await act(async () => hook.result.current.handleSave());
    expect(hook.result.current.values.title).toBe('Changed');
    expect(hook.result.current.currentObject?.properties.title).toBe('Original');
    expect(hook.result.current.historyRevision).toBe(0);
    expect(hook.result.current.isSaving).toBe(false);
    expect(navigate).not.toHaveBeenCalled();
    await act(async () => hook.result.current.handleSave());
    expect(hook.result.current.currentObject?.properties.title).toBe('Changed');
    expect(hook.result.current.historyRevision).toBe(1);
    expect(writes()).toHaveLength(2);
  });
  it.each(['object', 'account'])(
    'rejects late save completion after changing %s',
    async (target) => {
      const hook = await editor();
      await act(async () => hook.result.current.handleFieldChange('title', 'Changed'));
      let finish!: (value: ObjectData) => void;
      vi.mocked(invoke).mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finish = resolve;
          }),
      );
      let saving!: Promise<void>;
      act(() => {
        saving = hook.result.current.handleSave();
      });
      if (target === 'object') {
        route.objectId = 'B';
        hook.rerender();
        await waitFor(() => expect(hook.result.current.currentObject?.id).toBe('B'));
      } else await act(async () => useAuthStore.setState({ isAuthenticated: false }));
      await act(async () => {
        finish({ ...record, name: 'Late response' });
        await saving;
      });
      expect(hook.result.current.name).not.toBe('Late response');
      expect(navigate).not.toHaveBeenCalled();
    },
  );
  it('keeps mobile save navigation unchanged', async () => {
    vi.mocked(isMobilePlatformSync).mockReturnValue(true);
    const hook = await editor();
    await act(async () => hook.result.current.handleSave());
    expect(writes()).toHaveLength(1);
    expect(navigate).toHaveBeenCalledWith(-1);
  });
  it('keeps new-object save navigation unchanged', async () => {
    route.objectId = undefined;
    const hook = await editor();
    await act(async () => hook.result.current.handleSave());
    expect(invoke).toHaveBeenCalledWith('object_create', expect.anything());
    expect(navigate).toHaveBeenCalledWith(-1);
  });
  it('compares nested object keys by value while preserving dynamic group order', () => {
    record.properties = {
      __fields: { group: { type: 'dynamic_group' } },
      group: [
        { id: 'a', value: 'A' },
        { id: 'b', value: 'B' },
      ],
    };
    expect(
      objectEditorHasChanges(record, record.name, {
        group: [
          { value: 'A', id: 'a' },
          { value: 'B', id: 'b' },
        ],
      }),
    ).toBe(false);
    const draft = objectFormValues(record);
    expect(
      objectEditorHasChanges(record, record.name, {
        group: [...(draft.group as unknown[])].reverse(),
      }),
    ).toBe(true);
  });
});
