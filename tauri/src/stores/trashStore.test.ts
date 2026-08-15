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
});
