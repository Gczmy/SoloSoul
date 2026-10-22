import { useLayoutEffect } from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useTemplateStore } from '@/stores/templateStore';
import type { FieldSuggestion } from '@/components/editor/FieldSuggestions';
import { useObjectEditorPage } from './useObjectEditorPage';

const { route, navigate, stableT } = vi.hoisted(() => ({
  route: { objectId: 'A' as string | undefined },
  navigate: vi.fn(),
  stableT: (key: string) => key,
}));
vi.mock('react-router-dom', () => ({
  useParams: () => route,
  useNavigate: () => navigate,
  useSearchParams: () => [new URLSearchParams()],
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: stableT }),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function object(id: string) {
  return {
    id,
    accountId: 'account',
    name: `Object ${id}`,
    typeId: 'identity',
    templateId: 'template',
    properties: { fullName: `Name ${id}` },
    sensitivityLevel: 'public',
    createdAt: '2026-10-22',
    updatedAt: '2026-10-22',
  };
}

function suggestions(id: string): FieldSuggestion[] {
  return [
    {
      objectId: `source-${id}`,
      objectName: `Source ${id}`,
      fieldKey: 'legacy-name',
      fieldName: 'Full Name',
      sensitivityLevel: 'sensitive',
      value: `Suggestion ${id}`,
    },
  ];
}

describe('editor field suggestion handoff', () => {
  const reads = new Map<string, ReturnType<typeof deferred<ReturnType<typeof object>>>>();
  const recommendations = new Map<string, ReturnType<typeof deferred<FieldSuggestion[]>>>();

  beforeEach(() => {
    vi.clearAllMocks();
    reads.clear();
    recommendations.clear();
    route.objectId = 'A';
    useAuthStore.setState({
      currentAccount: { id: 'account', name: 'Public fixture' },
      isAuthenticated: true,
    });
    useObjectStore.setState({ currentObjectCache: {}, objects: [] });
    useTemplateStore.setState({
      templates: [
        {
          id: 'template',
          accountId: 'account',
          name: 'Identity',
          category: 'identity',
          createdAt: '',
          updatedAt: '',
          properties: [{ id: 'fullName', name: '全名', type: 'text' }],
        },
      ],
      loadTemplates: vi.fn().mockResolvedValue(undefined),
    });
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      const input = args as { objectId?: string; excludeObjectId?: string | null };
      if (command === 'object_get') {
        const pending = deferred<ReturnType<typeof object>>();
        reads.set(input.objectId!, pending);
        return pending.promise;
      }
      if (command === 'object_field_suggestions') {
        const pending = deferred<FieldSuggestion[]>();
        recommendations.set(input.excludeObjectId ?? 'new', pending);
        return pending.promise;
      }
      return undefined;
    });
  });

  const loadA = async () => {
    const hook = renderHook(useObjectEditorPage);
    await act(async () => {
      reads.get('A')!.resolve(object('A'));
    });
    await act(async () => {
      recommendations.get('A')!.resolve(suggestions('A'));
    });
    await waitFor(() => expect(hook.result.current.isObjectReady).toBe(true));
    return hook;
  };

  it.each(['object-first', 'suggestions-first'])(
    '%s: publishes the new form and recommendations together',
    async (order) => {
      const frames: { id: string | undefined; sources: string[] }[] = [];
      const hook = renderHook(() => {
        const editor = useObjectEditorPage();
        useLayoutEffect(() => {
          frames.push({
            id: editor.currentObject?.id,
            sources: Object.values(editor.fieldSuggestions)
              .flat()
              .map((item) => item.objectId),
          });
        });
        return editor;
      });
      // Both reads must start before either result arrives.
      expect(recommendations.has('A')).toBe(true);
      await act(async () => {
        reads.get('A')!.resolve(object('A'));
        recommendations.get('A')!.resolve(suggestions('A'));
      });
      route.objectId = 'B';
      hook.rerender();
      expect(recommendations.has('B')).toBe(true);
      await act(async () => {
        if (order === 'object-first') reads.get('B')!.resolve(object('B'));
        else recommendations.get('B')!.resolve(suggestions('B'));
      });
      expect(hook.result.current.currentObject?.id).toBe('A');
      expect(hook.result.current.fieldSuggestions.fullName).toEqual(suggestions('A'));
      expect(hook.result.current.isObjectReady).toBe(false);
      await act(async () => {
        if (order === 'object-first') recommendations.get('B')!.resolve(suggestions('B'));
        else reads.get('B')!.resolve(object('B'));
      });
      expect(hook.result.current.values.fullName).toBe('Name B');
      expect(hook.result.current.fieldSuggestions.fullName).toEqual(suggestions('B'));
      expect(hook.result.current.isObjectReady).toBe(true);
      expect(
        frames
          .filter((frame) => frame.id)
          .every((frame) => frame.sources.join() === `source-${frame.id}`),
      ).toBe(true);
      const calls = vi.mocked(invoke).mock.calls.length;
      hook.rerender();
      expect(vi.mocked(invoke).mock.calls.length).toBe(calls);
    },
  );

  it('ignores recommendations from a skipped object even when they finish last', async () => {
    const hook = await loadA();
    route.objectId = 'B';
    hook.rerender();
    const oldRecommendations = recommendations.get('B')!;
    await act(async () => {
      reads.get('B')!.resolve(object('B'));
    });
    route.objectId = 'C';
    hook.rerender();
    await act(async () => {
      reads.get('C')!.resolve(object('C'));
      recommendations.get('C')!.resolve(suggestions('C'));
    });
    await act(async () => {
      oldRecommendations.resolve(suggestions('B'));
    });
    expect(hook.result.current.currentObject?.id).toBe('C');
    expect(hook.result.current.fieldSuggestions.fullName).toEqual(suggestions('C'));
  });

  it('allows editing without recommendations when their request fails', async () => {
    const hook = await loadA();
    route.objectId = 'B';
    hook.rerender();
    await act(async () => {
      reads.get('B')!.resolve(object('B'));
      recommendations.get('B')!.reject(new Error('Fixture read failure'));
    });
    expect(hook.result.current.currentObject?.id).toBe('B');
    expect(hook.result.current.isObjectReady).toBe(true);
    expect(hook.result.current.objectLoadFailed).toBe(false);
    expect(hook.result.current.fieldSuggestions).toEqual({});
  });

  it('does not publish pending values after the vault locks', async () => {
    const hook = renderHook(useObjectEditorPage);
    await act(async () => {
      useAuthStore.setState({ isAuthenticated: false });
    });
    await act(async () => {
      reads.get('A')!.resolve(object('A'));
      recommendations.get('A')!.resolve(suggestions('A'));
    });
    expect(hook.result.current.currentObject).toBeNull();
    expect(hook.result.current.fieldSuggestions).toEqual({});
  });

  it.each([undefined, null])(
    'degrades a malformed recommendation response (%s) without failing the editor',
    async (response) => {
      const hook = await loadA();
      route.objectId = 'B';
      hook.rerender();
      await act(async () => {
        reads.get('B')!.resolve(object('B'));
        recommendations.get('B')!.resolve(response as unknown as FieldSuggestion[]);
      });
      expect(hook.result.current.isObjectReady).toBe(true);
      expect(hook.result.current.objectLoadFailed).toBe(false);
      expect(hook.result.current.currentObject?.id).toBe('B');
      expect(hook.result.current.fieldSuggestions).toEqual({});
    },
  );

  it('continues loading recommendations for a new object without excluding a source', async () => {
    route.objectId = undefined;
    const hook = renderHook(useObjectEditorPage);
    expect(reads.size).toBe(0);
    await act(async () => {
      recommendations.get('new')!.resolve(suggestions('new'));
    });
    expect(hook.result.current.isObjectReady).toBe(true);
    expect(hook.result.current.fieldSuggestions.fullName).toEqual(suggestions('new'));
    expect(invoke).toHaveBeenCalledWith('object_field_suggestions', {
      accountId: 'account',
      excludeObjectId: null,
    });
  });
});
