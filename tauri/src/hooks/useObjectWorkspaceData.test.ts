import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { IpcCommands } from '@/lib/generated/ipcContracts';
import { logger } from '@/lib/logger';
import { toObjectDataView } from '@/lib/objectViewModel';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTemplateStore } from '@/stores/templateStore';
import {
  useObjectWorkspaceData,
  type UseObjectWorkspaceDataOptions,
} from './useObjectWorkspaceData';

type WireObject = NonNullable<IpcCommands['object_get']['result']>;
const details = new Map<string, Promise<WireObject | null>>();

function object(id: string): WireObject {
  return {
    id,
    accountId: 'account',
    name: `合成对象 ${id}`,
    typeId: 'identity',
    properties: { value: id },
    sensitivityLevel: 'internal',
    templateId: null,
    templateType: null,
    propertyLabels: null,
    createdAt: '2026-09-28T10:00:00Z',
    updatedAt: '2026-09-28T10:00:00Z',
    deletedAt: null,
    contractTypeId: null,
    templateHash: null,
    ignoredTemplateHash: null,
  };
}

function deferredDetail() {
  let resolve!: (value: WireObject | null) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<WireObject | null>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

async function release(...pending: ReturnType<typeof deferredDetail>[]) {
  await act(async () => {
    for (const item of pending) item.resolve(null);
    await Promise.all(pending.map((item) => item.promise.catch(() => null)));
  });
}

function options(detailObjectId: string | null): UseObjectWorkspaceDataOptions {
  return { sectionFilter: 'identity', detailObjectId };
}

beforeEach(() => {
  vi.restoreAllMocks();
  details.clear();
  vi.mocked(invoke)
    .mockReset()
    .mockImplementation(async (command, args) => {
      if (command === 'object_get') {
        const objectId = args && 'objectId' in args ? args.objectId : undefined;
        const response = typeof objectId === 'string' ? details.get(objectId) : undefined;
        if (!response) throw new Error('Unexpected synthetic object_get');
        return response;
      }
      if (command === 'template_list' || command === 'object_list') return [];
      if (command === 'vault_list_accounts') return [{ id: 'account', name: '合成账户' }];
      if (command === 'biometric_check_availability') {
        return { available: false, configured: false };
      }
      return null;
    });
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useObjectStore.getState().clearOnVaultLock();
  useSettingsStore.getState().clearOnVaultLock();
  useTemplateStore.getState().clearOnVaultLock();
  useAuthStore.getState().completeUnlock({ id: 'account', name: '合成账户' });
});

afterEach(() => {
  cleanup();
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  vi.restoreAllMocks();
});

describe('RF302 工作区深链详情请求', () => {
  it('同账户 A 切 B 后，A 的迟到结果不能覆盖先返回的 B', async () => {
    const a = deferredDetail();
    const b = deferredDetail();
    details.set('A', a.promise);
    details.set('B', b.promise);
    const warning = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const { result, rerender, unmount } = renderHook(useObjectWorkspaceData, {
      initialProps: options('A'),
    });
    try {
      await waitFor(() =>
        expect(invoke).toHaveBeenCalledWith('object_get', {
          accountId: 'account',
          objectId: 'A',
        }),
      );
      rerender(options('B'));
      await waitFor(() =>
        expect(invoke).toHaveBeenCalledWith('object_get', {
          accountId: 'account',
          objectId: 'B',
        }),
      );
      await act(async () => {
        b.resolve(object('B'));
        await b.promise;
      });
      await waitFor(() => expect(result.current.detailObj?.id).toBe('B'));
      await act(async () => {
        a.resolve(object('A'));
        await a.promise;
      });
      expect(result.current.detailObj?.id).toBe('B');
      expect(result.current.detailObj?.properties).toEqual({ value: 'B' });
      expect(warning).not.toHaveBeenCalled();
      expect(vi.mocked(invoke).mock.calls.filter(([command]) => command === 'object_get')).toEqual([
        ['object_get', { accountId: 'account', objectId: 'A' }],
        ['object_get', { accountId: 'account', objectId: 'B' }],
      ]);
    } finally {
      unmount();
      await release(a, b);
    }
  });

  it('移除参数清空已显示深链详情，但不清空手动卡片选择', async () => {
    details.set('A', Promise.resolve(object('A')));
    const manual = toObjectDataView(object('manual'));
    const { result, rerender, unmount } = renderHook(useObjectWorkspaceData, {
      initialProps: options('A'),
    });
    try {
      await waitFor(() => expect(result.current.detailObj?.id).toBe('A'));
      rerender(options(null));
      expect(result.current.detailObj).toBeNull();

      rerender(options('A'));
      await waitFor(() => expect(result.current.detailObj?.id).toBe('A'));
      act(() => result.current.setDetailObj(manual));
      rerender(options(null));
      expect(result.current.detailObj).toEqual(manual);
      rerender({ ...options(null), pageId: 'another-page' });
      expect(result.current.detailObj).toEqual(manual);
    } finally {
      unmount();
    }
  });

  it.each(['close', 'manual', 'remove-parameter'])(
    '请求尚未返回时 %s，迟到结果不重新打开或覆盖详情',
    async (action) => {
      const pending = deferredDetail();
      details.set('A', pending.promise);
      const manual = toObjectDataView(object('manual'));
      const { result, rerender, unmount } = renderHook(useObjectWorkspaceData, {
        initialProps: options(null),
      });
      try {
        // 已有卡片详情时收到另一个深链；用户仍可关闭或选择其他卡片。
        act(() => result.current.setDetailObj(toObjectDataView(object('previous-card'))));
        rerender(options('A'));
        await waitFor(() =>
          expect(invoke).toHaveBeenCalledWith('object_get', {
            accountId: 'account',
            objectId: 'A',
          }),
        );
        if (action === 'remove-parameter') {
          rerender(options(null));
          act(() => result.current.setDetailObj(manual));
        } else {
          act(() => result.current.setDetailObj(action === 'close' ? null : manual));
        }
        await act(async () => {
          pending.resolve(object('A'));
          await pending.promise;
        });
        expect(result.current.detailObj).toEqual(action === 'close' ? null : manual);
        // URL 未变的普通重渲染不能重新发起已由用户关闭的读取。
        rerender(options(action === 'remove-parameter' ? null : 'A'));
        expect(result.current.detailObj).toEqual(action === 'close' ? null : manual);
        expect(
          vi.mocked(invoke).mock.calls.filter(([command]) => command === 'object_get'),
        ).toHaveLength(1);
      } finally {
        unmount();
        await release(pending);
      }
    },
  );

  it('当前请求错误仍报告，卸载后的迟到错误不再触发工作区反馈', async () => {
    const active = deferredDetail();
    const stale = deferredDetail();
    details.set('A', active.promise);
    details.set('B', stale.promise);
    const warning = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const { rerender, unmount } = renderHook(useObjectWorkspaceData, {
      initialProps: options('A'),
    });
    try {
      await waitFor(() =>
        expect(invoke).toHaveBeenCalledWith('object_get', {
          accountId: 'account',
          objectId: 'A',
        }),
      );
      const activeError = new Error('synthetic active error');
      await act(async () => {
        active.reject(activeError);
        await active.promise.catch(() => null);
      });
      await waitFor(() =>
        expect(warning).toHaveBeenCalledWith(
          '[Workspace] Fetch object detail failed:',
          activeError,
        ),
      );
      warning.mockClear();
      rerender(options('B'));
      await waitFor(() =>
        expect(invoke).toHaveBeenCalledWith('object_get', {
          accountId: 'account',
          objectId: 'B',
        }),
      );
      unmount();
      await act(async () => {
        stale.reject(new Error('synthetic stale error'));
        await stale.promise.catch(() => null);
      });
      expect(warning).not.toHaveBeenCalled();
    } finally {
      unmount();
      await release(active, stale);
    }
  });

  it.each(['locked', 'reunlocked'])(
    '保留 accountId 的 %s 会话不能接纳之前的详情响应',
    async (transition) => {
      const pending = deferredDetail();
      details.set('A', pending.promise);
      const { result, unmount } = renderHook(useObjectWorkspaceData, {
        initialProps: options('A'),
      });
      try {
        await waitFor(() =>
          expect(invoke).toHaveBeenCalledWith('object_get', {
            accountId: 'account',
            objectId: 'A',
          }),
        );
        act(() => useAuthStore.setState({ isAuthenticated: false }));
        if (transition === 'reunlocked') {
          act(() => useAuthStore.getState().completeUnlock({ id: 'account', name: '合成账户' }));
        }
        await act(async () => {
          pending.resolve(object('A'));
          await pending.promise;
        });
        expect(result.current.detailObj).toBeNull();
        expect(useAuthStore.getState().currentAccount?.id).toBe('account');
        expect(
          vi.mocked(invoke).mock.calls.filter(([command]) => command === 'object_get'),
        ).toHaveLength(1);
      } finally {
        unmount();
        await release(pending);
      }
    },
  );
});
