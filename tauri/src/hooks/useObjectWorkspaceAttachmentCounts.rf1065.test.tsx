import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { IpcCommands } from '@/lib/generated/ipcContracts';
import { logger } from '@/lib/logger';
import { toObjectDataView } from '@/lib/objectViewModel';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { useAttachmentViewer } from '@/components/object/useAttachmentViewer';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useUiStore } from '@/stores/uiStore';
import {
  useObjectWorkspaceData,
  type UseObjectWorkspaceDataOptions,
} from './useObjectWorkspaceData';

vi.mock('@/hooks/useDragToAttach', () => ({
  useDragToAttach: () => ({ ref: { current: null }, dragState: {} }),
}));
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((ok, fail) => {
    resolve = ok;
    reject = fail;
  });
  return { promise, resolve, reject };
}
type Counts = Record<string, number>;
const counts: { ids: string[]; operation: ReturnType<typeof deferred<Counts>> }[] = [];
const deletion = { operation: deferred<void>() };
const originalLoadObjects = useObjectStore.getState().loadObjects;
function object(id: string) {
  const value: NonNullable<IpcCommands['object_get']['result']> = {
    id,
    accountId: 'account-a',
    name: `合成对象 ${id}`,
    typeId: 'identity',
    properties: {},
    sensitivityLevel: 'internal',
    templateId: null,
    templateType: null,
    propertyLabels: null,
    createdAt: '2026-09-30',
    updatedAt: '2026-09-30',
    deletedAt: null,
    contractTypeId: null,
    templateHash: null,
    ignoredTemplateHash: null,
  };
  return toObjectDataView(value);
}
const item: AttachmentItem = {
  id: 'attachment-a',
  objectId: 'A',
  fileName: 'A.txt',
  mimeType: 'text/plain',
  sizeBytes: 5,
  createdAt: '2026-09-30',
};
function options(sectionFilter = 'identity'): UseObjectWorkspaceDataOptions {
  return { sectionFilter, detailObjectId: null };
}
function clearToasts() {
  for (const toast of useUiStore.getState().toasts) useUiStore.getState().dismissToast(toast.id);
}
beforeEach(() => {
  vi.restoreAllMocks();
  counts.length = 0;
  deletion.operation = deferred<void>();
  vi.mocked(invoke)
    .mockReset()
    .mockImplementation(async (command, args) => {
      if (command === 'attachment_count_batch') {
        const operation = deferred<Counts>();
        counts.push({ ids: (args as { objectIds: string[] }).objectIds, operation });
        return operation.promise;
      }
      if (command === 'snapshot_count_batch') return {};
      if (command === 'attachment_list')
        return (args as { showDeleted: boolean }).showDeleted ? [] : [item];
      if (command === 'attachment_delete') return deletion.operation.promise;
      if (command === 'template_list' || command === 'object_list') return [];
      if (command === 'biometric_check_availability')
        return { available: false, configured: false };
      return null;
    });
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useObjectStore.getState().clearOnVaultLock();
  useSettingsStore.getState().clearOnVaultLock();
  useTemplateStore.getState().clearOnVaultLock();
  // 只替换异步列表获取，测试仍运行真实父 Hook、Store 与会话订阅。
  useObjectStore.setState({ loadObjects: vi.fn().mockResolvedValue(undefined) });
  useAuthStore.setState({
    currentAccount: { id: 'account-a', name: '合成账户 A' },
    isAuthenticated: true,
  });
  useObjectStore.setState({ objects: [object('A')] });
  clearToasts();
});
afterEach(async () => {
  cleanup();
  await act(async () => {
    for (const count of counts) count.operation.resolve({});
  });
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useObjectStore.setState({ loadObjects: originalLoadObjects });
  clearToasts();
  vi.restoreAllMocks();
});
async function resolveCount(index: number, value: Counts) {
  await act(async () => {
    counts[index].operation.resolve(value);
    await counts[index].operation.promise;
  });
}

describe('RF1065 工作区附件计数回调归属', () => {
  it('旧列表回调不能取消 B 请求，迟到 A 结果不能覆盖 B', async () => {
    const { result, rerender } = renderHook(useObjectWorkspaceData, { initialProps: options() });
    await waitFor(() => expect(counts).toHaveLength(1));
    const stale = result.current.refreshAttachmentCounts;
    act(() => useObjectStore.setState({ objects: [object('B')] }));
    rerender(options('financial'));
    await waitFor(() => expect(counts.at(-1)?.ids).toEqual(['B']));
    const current = counts.length - 1;
    act(() => stale());
    expect(counts).toHaveLength(current + 1);
    await resolveCount(current, { B: 7 });
    expect(result.current.attachmentCounts).toEqual({ B: 7 });
    await resolveCount(0, { A: 99 });
    expect(result.current.attachmentCounts).toEqual({ B: 7 });
  });
  it('只切路由且列表引用不变，仍拒绝旧回调', async () => {
    const { result, rerender } = renderHook(useObjectWorkspaceData, { initialProps: options() });
    await waitFor(() => expect(counts).toHaveLength(1));
    const stale = result.current.refreshAttachmentCounts;
    rerender({ ...options(), pageId: 'another-page' });
    act(() => result.current.refreshAttachmentCounts());
    const current = counts.length - 1;
    act(() => stale());
    expect(counts).toHaveLength(current + 1);
    await resolveCount(current, { A: 3 });
    expect(result.current.attachmentCounts).toEqual({ A: 3 });
  });
  it('路由 A→B→A 不恢复第一代回调', async () => {
    const { result, rerender } = renderHook(useObjectWorkspaceData, { initialProps: options() });
    await waitFor(() => expect(counts).toHaveLength(1));
    const stale = result.current.refreshAttachmentCounts;
    rerender(options('financial'));
    rerender(options());
    act(() => result.current.refreshAttachmentCounts());
    const current = counts.length - 1;
    act(() => stale());
    expect(counts).toHaveLength(current + 1);
    await resolveCount(current, { A: 5 });
    expect(result.current.attachmentCounts).toEqual({ A: 5 });
  });
  it('当前回调重复刷新仍保持最新请求优先', async () => {
    const { result } = renderHook(useObjectWorkspaceData, { initialProps: options() });
    await waitFor(() => expect(counts).toHaveLength(1));
    act(() => result.current.refreshAttachmentCounts());
    expect(counts).toHaveLength(2);
    await resolveCount(1, { A: 2 });
    await resolveCount(0, { A: 1 });
    expect(result.current.attachmentCounts).toEqual({ A: 2 });
  });
  it('空列表清空旧计数并拒绝旧回调', async () => {
    const { result } = renderHook(useObjectWorkspaceData, { initialProps: options() });
    await waitFor(() => expect(counts).toHaveLength(1));
    await resolveCount(0, { A: 4 });
    const stale = result.current.refreshAttachmentCounts;
    act(() => useObjectStore.setState({ objects: [] }));
    expect(result.current.attachmentCounts).toEqual({});
    act(() => stale());
    expect(counts).toHaveLength(1);
  });
  it('卸载后旧回调不发 IPC', async () => {
    const { result, unmount } = renderHook(useObjectWorkspaceData, { initialProps: options() });
    await waitFor(() => expect(counts).toHaveLength(1));
    const stale = result.current.refreshAttachmentCounts;
    unmount();
    act(() => stale());
    expect(counts).toHaveLength(1);
  });
  it('换账户后旧回调与迟到错误不影响新账户', async () => {
    const warning = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const { result } = renderHook(useObjectWorkspaceData, { initialProps: options() });
    await waitFor(() => expect(counts).toHaveLength(1));
    const stale = result.current.refreshAttachmentCounts;
    act(() => {
      useAuthStore.setState({
        currentAccount: { id: 'account-b', name: '合成账户 B' },
        isAuthenticated: true,
      });
      useObjectStore.setState({ objects: [object('B')] });
    });
    await waitFor(() => expect(counts.at(-1)?.ids).toEqual(['B']));
    const current = counts.length - 1;
    act(() => stale());
    expect(counts).toHaveLength(current + 1);
    await act(async () => {
      counts[0].operation.reject(new Error('迟到失败'));
      await counts[0].operation.promise.catch(() => {});
    });
    expect(warning).not.toHaveBeenCalled();
    await resolveCount(current, { B: 6 });
    expect(result.current.attachmentCounts).toEqual({ B: 6 });
  });
  it('同一 React 批次锁定并重新解锁、恢复相同列表引用也不恢复旧回调', async () => {
    const { result } = renderHook(useObjectWorkspaceData, { initialProps: options() });
    await waitFor(() => expect(counts).toHaveLength(1));
    const stale = result.current.refreshAttachmentCounts;
    const objects = useObjectStore.getState().objects;
    act(() => {
      useAuthStore.setState({ isAuthenticated: false });
      useAuthStore.setState({ isAuthenticated: true });
      useObjectStore.setState({ objects });
    });
    act(() => result.current.refreshAttachmentCounts());
    const current = counts.length - 1;
    act(() => stale());
    expect(counts).toHaveLength(current + 1);
    await resolveCount(current, { A: 8 });
    expect(result.current.attachmentCounts).toEqual({ A: 8 });
  });
  it('真实 Viewer 删除回调在父列表切换后不能取消 B 计数', async () => {
    const { result, rerender } = renderHook(
      ({ section }) => {
        const workspace = useObjectWorkspaceData(options(section));
        const viewer = useAttachmentViewer({
          objectId: 'A',
          onClose: () => {},
          onCountChange: workspace.refreshAttachmentCounts,
        });
        return { workspace, viewer };
      },
      { initialProps: { section: 'identity' } },
    );
    await waitFor(() => expect(result.current.viewer.items).toHaveLength(1));
    await waitFor(() => expect(counts).toHaveLength(1));
    let removal!: Promise<void>;
    act(() => {
      removal = result.current.viewer.handlePermanentDelete(item);
    });
    act(() => useObjectStore.setState({ objects: [object('B')] }));
    rerender({ section: 'financial' });
    await waitFor(() => expect(counts.at(-1)?.ids).toEqual(['B']));
    const current = counts.length - 1;
    await act(async () => {
      deletion.operation.reject('attachment_cleanup_pending');
      await removal;
    });
    expect(counts).toHaveLength(current + 1);
    await resolveCount(current, { B: 9 });
    expect(result.current.workspace.attachmentCounts).toEqual({ B: 9 });
  });
});
