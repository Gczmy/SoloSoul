import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useTrashStore, type RestoreOutcome } from './trashStore';
import { setRequestSession } from '@/lib/sessionRequests';
import type { TrashItemSummary } from '@/lib/generated/ipcContracts';
import i18next from '@/lib/i18n';
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

// RF-909：保留真实 Store、会话票据和 wire 映射，仅替身最外层 Tauri IPC。
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function trashItem(id: string, overrides: Partial<TrashItemSummary> = {}): TrashItemSummary {
  return {
    id,
    itemType: 'object',
    originalId: `original-${id}`,
    name: `Object ${id}`,
    iconId: null,
    deletedAt: Date.parse('2026-08-14T12:00:00Z'),
    expiresAt: null,
    originalParentId: null,
    originalSectionType: null,
    contractTypeId: null,
    ...overrides,
  };
}

async function loadTrash(items: TrashItemSummary[], accountId = 'acc-a') {
  mockInvoke.mockResolvedValueOnce(items);
  await useTrashStore.getState().loadItems(accountId);
  mockInvoke.mockClear();
}

function visibleTrashState() {
  const { items, isLoading, error, selectedIds, timeFilter, typeFilter, searchQuery } =
    useTrashStore.getState();
  return { items, isLoading, error, selectedIds, timeFilter, typeFilter, searchQuery };
}

const mutationCases = [
  {
    name: 'template restore',
    itemType: 'template',
    command: 'template_restore',
    run: () => useTrashStore.getState().restoreItem('same-id'),
    result: undefined,
  },
  {
    name: 'object restore',
    itemType: 'object',
    command: 'trash_restore',
    run: () => useTrashStore.getState().restoreItem('same-id'),
    result: {
      restoredId: 'old-object',
      name: 'Old account object',
      consumedTrashIds: ['same-id', 'sibling'],
    },
  },
  {
    name: 'batch restore',
    itemType: 'object',
    command: 'trash_restore_batch',
    run: () => useTrashStore.getState().restoreBatch(['same-id', 'sibling']),
    result: [
      {
        restoredId: 'old-object',
        name: 'Old account object',
        consumedTrashIds: ['same-id', 'sibling'],
      },
    ],
  },
  {
    name: 'permanent delete',
    itemType: 'object',
    command: 'trash_permanent_delete_batch',
    run: () => useTrashStore.getState().permanentDelete(['same-id', 'sibling']),
    result: undefined,
  },
] as const;

describe('RF-909 trash recovery and session boundaries', () => {
  beforeEach(() => {
    setRequestSession(null);
    setRequestSession('acc-a');
    mockInvoke.mockReset().mockResolvedValue([]);
  });

  afterEach(() => {
    setRequestSession(null);
    vi.restoreAllMocks();
  });

  it.each([
    ['1d', '2026-08-14T12:00:00Z'],
    ['7d', '2026-08-08T12:00:00Z'],
    ['30d', '2026-07-16T12:00:00Z'],
    ['half_year', '2026-02-16T12:00:00Z'],
  ] as const)(
    'requests %s using the deletion cutoff rather than the duration',
    async (filter, cutoff) => {
      vi.spyOn(Date, 'now').mockReturnValue(Date.parse('2026-08-15T12:00:00Z'));
      useTrashStore.getState().setTimeFilter(filter);
      await useTrashStore.getState().loadItems('acc-a');
      expect(mockInvoke).toHaveBeenCalledExactlyOnceWith('object_trash_list', {
        accountId: 'acc-a',
        since: Date.parse(cutoff),
      });
    },
  );

  it('loads typed rows, preserves supplied metadata and clears a previous selection', async () => {
    useTrashStore.getState().selectAll(['stale-row']);
    const rows = [
      trashItem('object'),
      trashItem('page', {
        itemType: 'page',
        iconId: 'folder',
        expiresAt: 1788000000000,
        originalParentId: 'parent',
        originalSectionType: 'identity',
        contractTypeId: 'contract',
      }),
    ];
    mockInvoke.mockResolvedValueOnce(rows);
    await useTrashStore.getState().loadItems('acc-a');

    expect(mockInvoke).toHaveBeenCalledExactlyOnceWith('object_trash_list', { accountId: 'acc-a' });
    expect(useTrashStore.getState().items).toEqual([
      {
        ...rows[0],
        iconId: undefined,
        expiresAt: undefined,
        originalParentId: undefined,
        originalSectionType: undefined,
        contractTypeId: undefined,
      },
      rows[1],
    ]);
    expect(useTrashStore.getState()).toMatchObject({
      isLoading: false,
      error: null,
      selectedIds: new Set(),
    });
    expect(rows[0].iconId).toBeNull();
  });

  it('retains rows and selection on load failure, then replaces them on a successful retry', async () => {
    await loadTrash([trashItem('visible'), trashItem('unselected')]);
    useTrashStore.getState().selectAll(['visible', 'unselected']);
    useTrashStore.getState().toggleSelection('unselected');
    const previousRows = useTrashStore.getState().items;
    const response = deferred<TrashItemSummary[]>();
    mockInvoke.mockReturnValueOnce(response.promise);
    const failed = useTrashStore.getState().loadItems('acc-a');
    expect(useTrashStore.getState()).toMatchObject({ isLoading: true, error: null });
    response.reject(new Error('list unavailable'));
    await failed;

    expect(useTrashStore.getState()).toMatchObject({
      items: previousRows,
      selectedIds: new Set(['visible']),
      isLoading: false,
      error: 'Error: list unavailable',
    });
    const retry = deferred<TrashItemSummary[]>();
    mockInvoke.mockReturnValueOnce(retry.promise);
    const pendingRetry = useTrashStore.getState().loadItems('acc-a');
    expect(useTrashStore.getState()).toMatchObject({
      items: previousRows,
      isLoading: true,
      error: null,
    });
    retry.resolve([trashItem('replacement')]);
    await pendingRetry;
    expect(useTrashStore.getState().items.map((item) => item.id)).toEqual(['replacement']);
    expect(useTrashStore.getState()).toMatchObject({
      selectedIds: new Set(),
      isLoading: false,
      error: null,
    });
  });

  it.each(['resolve', 'reject'] as const)(
    'keeps the newest filtered list after an older request %s',
    async (finish) => {
      const first = deferred<TrashItemSummary[]>();
      const second = deferred<TrashItemSummary[]>();
      mockInvoke.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
      vi.spyOn(Date, 'now').mockReturnValue(Date.parse('2026-08-15T12:00:00Z'));
      useTrashStore.getState().setTimeFilter('30d');
      const oldLoad = useTrashStore.getState().loadItems('acc-a');
      useTrashStore.getState().setTimeFilter('1d');
      const newLoad = useTrashStore.getState().loadItems('acc-a');
      expect(mockInvoke).toHaveBeenNthCalledWith(1, 'object_trash_list', {
        accountId: 'acc-a',
        since: Date.parse('2026-07-16T12:00:00Z'),
      });
      expect(mockInvoke).toHaveBeenNthCalledWith(2, 'object_trash_list', {
        accountId: 'acc-a',
        since: Date.parse('2026-08-14T12:00:00Z'),
      });
      second.resolve([trashItem('new')]);
      await newLoad;
      useTrashStore.getState().toggleSelection('new');
      const current = visibleTrashState();
      if (finish === 'resolve') first.resolve([trashItem('old')]);
      else first.reject(new Error('old list failed'));
      await oldLoad;
      expect(visibleTrashState()).toEqual(current);
      expect(current.items.map((item) => item.id)).toEqual(['new']);
    },
  );

  it('rejects an old-account load without invalidating the current account pending list', async () => {
    setRequestSession('acc-b');
    const response = deferred<TrashItemSummary[]>();
    mockInvoke.mockReturnValueOnce(response.promise);
    const current = useTrashStore.getState().loadItems('acc-b');
    await useTrashStore.getState().loadItems('acc-a');
    expect(mockInvoke).toHaveBeenCalledExactlyOnceWith('object_trash_list', { accountId: 'acc-b' });
    expect(useTrashStore.getState()).toMatchObject({ isLoading: true, error: null });
    response.resolve([trashItem('account-b')]);
    await current;
    expect(useTrashStore.getState().items.map((item) => item.id)).toEqual(['account-b']);
    expect(useTrashStore.getState().isLoading).toBe(false);
  });

  it('dispatches a template restore without invoking the object endpoint', async () => {
    await loadTrash([
      trashItem('template', { itemType: 'template', name: 'Saved template' }),
      trashItem('keep'),
    ]);
    mockInvoke.mockResolvedValueOnce(undefined);
    await expect(useTrashStore.getState().restoreItem('template')).resolves.toEqual({
      restoredId: 'original-template',
      name: 'Saved template',
      cascadedCount: 0,
    });
    expect(mockInvoke).toHaveBeenCalledExactlyOnceWith('template_restore', { trashId: 'template' });
    expect(useTrashStore.getState().items.map((item) => item.id)).toEqual(['keep']);
  });

  it('removes the server-reported cascade while retaining unrelated trash', async () => {
    await loadTrash([
      trashItem('page', { itemType: 'page' }),
      trashItem('child'),
      trashItem('keep'),
    ]);
    const outcome: RestoreOutcome = {
      restoredId: 'restored-page',
      name: 'Page',
      cascadedCount: 1,
      consumedTrashIds: ['page', 'child'],
    };
    mockInvoke.mockResolvedValueOnce(outcome);
    await expect(useTrashStore.getState().restoreItem('page')).resolves.toEqual(outcome);
    expect(mockInvoke).toHaveBeenCalledExactlyOnceWith('trash_restore', {
      trashId: 'page',
      lang: i18next.language,
    });
    expect(useTrashStore.getState().items.map((item) => item.id)).toEqual(['keep']);
  });

  it('consumes only the restored row when a legacy single result omits cascade metadata', async () => {
    await loadTrash([trashItem('restored'), trashItem('keep')]);
    mockInvoke.mockResolvedValueOnce({ restoredId: 'original-restored', name: 'Restored' });
    await useTrashStore.getState().restoreItem('restored');
    expect(mockInvoke).toHaveBeenCalledExactlyOnceWith('trash_restore', {
      trashId: 'restored',
      lang: i18next.language,
    });
    expect(useTrashStore.getState().items.map((item) => item.id)).toEqual(['keep']);
  });

  it.each([
    ['backend string', 'Trash item not found'],
    ['backend Error', new Error('Trash item not found')],
  ] as const)(
    'accepts a cascade-consumed row reported as %s without losing its display identity',
    async (_name, error) => {
      await loadTrash([trashItem('consumed', { name: 'Child' }), trashItem('keep')]);
      mockInvoke.mockRejectedValueOnce(error);
      await expect(useTrashStore.getState().restoreItem('consumed')).resolves.toEqual({
        restoredId: 'original-consumed',
        name: 'Child',
        cascadedCount: 0,
      });
      expect(useTrashStore.getState().items.map((item) => item.id)).toEqual(['keep']);
    },
  );

  it('makes a repeated restore idempotent after its local row has already disappeared', async () => {
    await loadTrash([trashItem('keep')]);
    mockInvoke.mockRejectedValueOnce('Trash item not found');
    await expect(useTrashStore.getState().restoreItem('already-consumed')).resolves.toEqual({
      restoredId: 'already-consumed',
      name: 'already-consumed',
      cascadedCount: 0,
    });
    expect(mockInvoke).toHaveBeenCalledExactlyOnceWith('trash_restore', {
      trashId: 'already-consumed',
      lang: i18next.language,
    });
    expect(useTrashStore.getState().items.map((item) => item.id)).toEqual(['keep']);
  });

  it.each(mutationCases)(
    'retains the visible rows and selection when $name fails',
    async (operation) => {
      await loadTrash([
        trashItem('same-id', { itemType: operation.itemType }),
        trashItem('sibling'),
      ]);
      useTrashStore.getState().selectAll(['same-id', 'sibling']);
      const before = visibleTrashState();
      const error = new Error('operation rejected');
      mockInvoke.mockRejectedValueOnce(error);
      await expect(operation.run()).rejects.toBe(error);
      expect(mockInvoke).toHaveBeenCalledTimes(1);
      expect(mockInvoke.mock.calls[0][0]).toBe(operation.command);
      expect(visibleTrashState()).toEqual(before);
    },
  );

  it('restores a mixed batch with one IPC and consumes only acknowledged cascade rows', async () => {
    await loadTrash([
      trashItem('page', { itemType: 'page' }),
      trashItem('child'),
      trashItem('template', { itemType: 'template' }),
      trashItem('keep'),
    ]);
    const outcomes: RestoreOutcome[] = [
      {
        restoredId: 'original-page',
        name: 'Page',
        cascadedCount: 1,
        consumedTrashIds: ['page', 'child'],
      },
      {
        restoredId: 'original-template',
        name: 'Template',
        cascadedCount: 0,
        consumedTrashIds: ['template'],
      },
    ];
    mockInvoke.mockResolvedValueOnce(outcomes);
    await expect(useTrashStore.getState().restoreBatch(['page', 'template'])).resolves.toEqual(
      outcomes,
    );
    expect(mockInvoke).toHaveBeenCalledExactlyOnceWith('trash_restore_batch', {
      trashIds: ['page', 'template'],
      lang: i18next.language,
    });
    expect(useTrashStore.getState().items.map((item) => item.id)).toEqual(['keep']);
  });

  it('does not assume unacknowledged rows were restored when an idempotent batch returns no outcomes', async () => {
    await loadTrash([trashItem('keep')]);
    mockInvoke.mockResolvedValueOnce([]);
    await expect(useTrashStore.getState().restoreBatch(['already-consumed'])).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledExactlyOnceWith('trash_restore_batch', {
      trashIds: ['already-consumed'],
      lang: i18next.language,
    });
    expect(useTrashStore.getState().items.map((item) => item.id)).toEqual(['keep']);
  });

  it('permanently deletes the selected subset through a single batch request', async () => {
    await loadTrash([trashItem('delete-a'), trashItem('delete-b'), trashItem('keep')]);
    useTrashStore.getState().toggleSelection('delete-a');
    useTrashStore.getState().toggleSelection('delete-b');
    const ids = [...useTrashStore.getState().selectedIds];
    mockInvoke.mockResolvedValueOnce(undefined);
    await useTrashStore.getState().permanentDelete(ids);
    expect(mockInvoke).toHaveBeenCalledExactlyOnceWith('trash_permanent_delete_batch', {
      trashIds: ['delete-a', 'delete-b'],
    });
    expect(useTrashStore.getState().items.map((item) => item.id)).toEqual(['keep']);
  });

  for (const transition of ['lock', 'switch account'] as const) {
    it.each(['resolve', 'reject'] as const)(
      `${transition}: late list %s cannot restore old data or errors`,
      async (finish) => {
        await loadTrash([trashItem('old')]);
        useTrashStore.getState().setTimeFilter('7d');
        useTrashStore.getState().setTypeFilter('template');
        useTrashStore.getState().setSearchQuery('old secret');
        useTrashStore.getState().selectAll(['old']);
        const response = deferred<TrashItemSummary[]>();
        mockInvoke.mockReturnValueOnce(response.promise);
        const loading = useTrashStore.getState().loadItems('acc-a');
        expect(useTrashStore.getState().isLoading).toBe(true);
        setRequestSession(transition === 'lock' ? null : 'acc-b');
        expect(visibleTrashState()).toEqual({
          items: [],
          isLoading: false,
          error: null,
          selectedIds: new Set(),
          timeFilter: 'all',
          typeFilter: 'all',
          searchQuery: '',
        });
        if (transition === 'switch account') {
          await loadTrash([trashItem('new')], 'acc-b');
          useTrashStore.getState().toggleSelection('new');
        }
        const current = visibleTrashState();
        if (finish === 'resolve') response.resolve([trashItem('old secret')]);
        else response.reject(new Error('old secret failure'));
        await loading;
        expect(visibleTrashState()).toEqual(current);
      },
    );

    for (const operation of mutationCases) {
      it.each(['resolve', 'reject'] as const)(
        `${transition}: late ${operation.name} %s cannot remove another session's rows`,
        async (finish) => {
          await loadTrash([
            trashItem('same-id', { itemType: operation.itemType }),
            trashItem('sibling'),
          ]);
          useTrashStore.getState().selectAll(['same-id', 'sibling']);
          const response = deferred<unknown>();
          mockInvoke.mockReturnValueOnce(response.promise);
          // 立即观察拒绝，避免受控 Promise 在断言前成为未处理的 rejection。
          const outcome = operation.run().then(
            (value) => ({ status: 'fulfilled' as const, value }),
            (error: unknown) => ({ status: 'rejected' as const, error }),
          );
          expect(mockInvoke).toHaveBeenCalledTimes(1);
          expect(mockInvoke.mock.calls[0][0]).toBe(operation.command);
          setRequestSession(transition === 'lock' ? null : 'acc-b');
          expect(useTrashStore.getState().items).toEqual([]);
          expect(useTrashStore.getState().selectedIds).toEqual(new Set());
          if (transition === 'switch account') {
            // 使用相同 ID，证明旧完成回调不能误删新账户刚加载的行。
            await loadTrash(
              [trashItem('same-id', { name: 'New account' }), trashItem('sibling')],
              'acc-b',
            );
            useTrashStore.getState().selectAll(['same-id']);
          }
          const current = visibleTrashState();
          if (finish === 'resolve') response.resolve(operation.result);
          else response.reject(new Error('Trash item not found'));
          const result = await outcome;
          expect(result.status).toBe('rejected');
          if (result.status === 'rejected') {
            expect(result.error).toBeInstanceOf(Error);
            expect((result.error as Error).message).toBe('Request belongs to an expired session');
          }
          expect(visibleTrashState()).toEqual(current);
        },
      );
    }
  }
});
