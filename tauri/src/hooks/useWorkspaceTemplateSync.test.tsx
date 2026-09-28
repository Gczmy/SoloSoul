import { act, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { TemplateSyncResult } from '@/lib/templateSync';
import { useWorkspaceTemplateSync } from './useWorkspaceTemplateSync';

function preview(templateHash: string, hasChanges: boolean): TemplateSyncResult {
  return {
    templateHash,
    hasChanges,
    fieldsAdded: [],
    fieldsDeprecated: [],
    fieldsUpdated: [],
    fieldsIncompatible: [],
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function setup(
  previewSyncTemplate: (accountId: string, objectId: string) => Promise<TemplateSyncResult>,
) {
  const applySyncTemplate = vi.fn().mockResolvedValue(undefined);
  const loadObjects = vi.fn().mockResolvedValue(undefined);
  const options = {
    accountId: 'account',
    pageId: undefined,
    sectionFilter: '',
    detailObj: null,
    setDetailObj: vi.fn(),
    userTemplates: [],
    loadObjects,
    previewSyncTemplate,
    applySyncTemplate,
    ignoreTemplateSync: vi.fn().mockResolvedValue(undefined),
  };
  const view = renderHook(() => useWorkspaceTemplateSync(options));
  return { ...view, applySyncTemplate, loadObjects };
}

describe('RF-921 template sync preview ownership', () => {
  it('keeps the newer object dialog when an earlier preview finishes late', async () => {
    const oldPreview = deferred<TemplateSyncResult>();
    const newPreview = deferred<TemplateSyncResult>();
    const previews = vi.fn((_accountId: string, objectId: string) =>
      objectId === 'object-a' ? oldPreview.promise : newPreview.promise,
    );
    const { result } = setup(previews);
    let first!: Promise<void>;
    act(() => {
      first = result.current.handleStartSync('object-a', 'A');
    });
    let second!: Promise<void>;
    act(() => {
      second = result.current.handleStartSync('object-b', 'B');
    });
    await act(async () => {
      newPreview.resolve(preview('hash-b', true));
      await second;
    });
    expect(result.current.syncDialog).toMatchObject({
      objectId: 'object-b',
      result: preview('hash-b', true),
      loading: false,
    });
    await act(async () => {
      oldPreview.resolve(preview('hash-a', true));
      await first;
    });
    expect(result.current.syncDialog).toMatchObject({
      objectId: 'object-b',
      result: preview('hash-b', true),
      loading: false,
    });
  });

  it('does not close a newer dialog when an older preview fails', async () => {
    const oldPreview = deferred<TemplateSyncResult>();
    const newPreview = deferred<TemplateSyncResult>();
    const previews = vi.fn((_accountId: string, objectId: string) =>
      objectId === 'object-a' ? oldPreview.promise : newPreview.promise,
    );
    const { result } = setup(previews);
    let first!: Promise<void>;
    act(() => {
      first = result.current.handleStartSync('object-a', 'A');
    });
    let second!: Promise<void>;
    act(() => {
      second = result.current.handleStartSync('object-b', 'B');
    });
    await act(async () => {
      newPreview.resolve(preview('hash-b', true));
      await second;
    });
    await act(async () => {
      oldPreview.reject(new Error('old preview failed'));
      await first;
    });
    expect(result.current.syncDialog).toMatchObject({
      objectId: 'object-b',
      result: preview('hash-b', true),
      loading: false,
    });
  });

  it('does not auto-apply a no-change preview after the user closes its dialog', async () => {
    const oldPreview = deferred<TemplateSyncResult>();
    const { result, applySyncTemplate, loadObjects } = setup(() => oldPreview.promise);
    let request!: Promise<void>;
    act(() => {
      request = result.current.handleStartSync('object-a', 'A');
    });
    act(() => result.current.handleCancelSync());
    await act(async () => {
      oldPreview.resolve(preview('hash-a', false));
      await request;
    });
    expect(result.current.syncDialog).toBeNull();
    expect(applySyncTemplate).not.toHaveBeenCalled();
    expect(loadObjects).not.toHaveBeenCalled();
  });

  it('closes the old dialog and ignores its preview when the account changes', async () => {
    const oldPreview = deferred<TemplateSyncResult>();
    const previewSyncTemplate = vi.fn().mockReturnValue(oldPreview.promise);
    const applySyncTemplate = vi.fn().mockResolvedValue(undefined);
    const options = {
      pageId: undefined,
      sectionFilter: '',
      detailObj: null,
      setDetailObj: vi.fn(),
      userTemplates: [],
      loadObjects: vi.fn().mockResolvedValue(undefined),
      previewSyncTemplate,
      applySyncTemplate,
      ignoreTemplateSync: vi.fn().mockResolvedValue(undefined),
    };
    const { result, rerender } = renderHook(
      ({ accountId }) => useWorkspaceTemplateSync({ ...options, accountId }),
      { initialProps: { accountId: 'account-a' } },
    );
    let request!: Promise<void>;
    act(() => {
      request = result.current.handleStartSync('object-a', 'A');
    });
    expect(result.current.syncDialog?.objectId).toBe('object-a');
    rerender({ accountId: 'account-b' });
    expect(result.current.syncDialog).toBeNull();
    await act(async () => {
      oldPreview.resolve(preview('hash-a', false));
      await request;
    });
    expect(result.current.syncDialog).toBeNull();
    expect(applySyncTemplate).not.toHaveBeenCalled();
    expect(options.loadObjects).not.toHaveBeenCalled();
  });
});
