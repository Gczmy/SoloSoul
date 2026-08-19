import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useTrashStore } from './trashStore';
import { invoke } from '@tauri-apps/api/core';

// 与既有测试（AttachmentRow 等）同款：mock invokeCommand 底层 @tauri-apps/api/core
const mockInvoke = vi.mocked(invoke);

describe('trashStore time filter', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue([]);
    // 每个用例重置 store 状态（含 timeFilter），避免用例间污染
    useTrashStore.setState({
      items: [],
      timeFilter: 'all',
      typeFilter: 'all',
      searchQuery: '',
      isLoading: false,
      error: null,
      selectedIds: new Set(),
    });
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-08-15T12:00:00Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // 回归（修复：回收站时间筛选失效）：TIME_SINCE 存的是相对偏移量，后端
  // since 语义是绝对毫秒时间戳（SQL: deleted_at >= since）。此前把偏移量
  // 原样直传（如 1d=86400000），被当成 1970 年的时间戳比较，任何真实
  // deleted_at（≈1.78e12）都恒 ≥ 它 → 过滤等于不过滤。
  it('loadItems converts time filter offset to an absolute timestamp before invoking', async () => {
    useTrashStore.setState({ timeFilter: '3d' });
    await useTrashStore.getState().loadItems('acc-1');

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    const [cmd, args] = mockInvoke.mock.calls[0] as [string, { accountId: string; since: number }];
    expect(cmd).toBe('object_trash_list');
    expect(args.accountId).toBe('acc-1');
    // 3d = 3*24*3600*1000 ms → since = now - 3天（绝对时间戳，2026-08-12T12:00:00Z）
    const now = Date.now();
    expect(args.since).toBe(now - 3 * 24 * 3600 * 1000);
    // 数值规模必须是真实时间戳量级（远大于 86400000 之类的偏移量）
    expect(args.since).toBeGreaterThan(1_000_000_000_000);
  });

  it('does not pass since when filter is all', async () => {
    useTrashStore.setState({ timeFilter: 'all' });
    await useTrashStore.getState().loadItems('acc-1');

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    const [cmd, args] = mockInvoke.mock.calls[0] as [string, { accountId: string; since?: number }];
    expect(cmd).toBe('object_trash_list');
    expect(args.since).toBeUndefined();
  });

  // P014: 批量恢复单次 IPC——按 consumedTrashIds（含级联消费）过滤本地列表
  it('restoreBatch invokes trash_restore_batch once and filters consumed ids', async () => {
    useTrashStore.setState({
      items: [
        { id: 't1', itemType: 'object', originalId: 'o1', name: 'A', deletedAt: 1 },
        { id: 't2', itemType: 'object', originalId: 'o2', name: 'B', deletedAt: 1 },
        { id: 't3', itemType: 'page', originalId: 'p1', name: 'P', deletedAt: 1 },
      ] as never,
    });
    // t3（页面）级联消费了 t1；t2 正常恢复
    mockInvoke.mockResolvedValue([
      {
        restoredId: 'p1',
        name: 'P',
        cascadedCount: 1,
        consumedTrashIds: ['t3', 't1'],
      },
      { restoredId: 'o2', name: 'B', cascadedCount: 0, consumedTrashIds: ['t2'] },
    ]);

    const outcomes = await useTrashStore.getState().restoreBatch(['t1', 't2', 't3']);

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    const [cmd, args] = mockInvoke.mock.calls[0] as [string, { trashIds: string[]; lang: string }];
    expect(cmd).toBe('trash_restore_batch');
    expect(args.trashIds).toEqual(['t1', 't2', 't3']);
    expect(outcomes).toHaveLength(2);
    // 被级联消费的 t1/t3 与恢复的 t2 全部移出列表
    expect(useTrashStore.getState().items.map((i) => i.id)).toEqual([]);
  });
});
