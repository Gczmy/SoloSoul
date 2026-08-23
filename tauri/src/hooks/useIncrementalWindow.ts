import { useCallback, useState } from 'react';

/**
 * P026：增量分页窗口共享 hook——收敛「visibleLimit + slice + 加载更多」样板
 * （此前散落 5+ 处：OperationLogPage / DebugLogPage / HistoryPage /
 * useTrashPage / useObjectWorkspaceData / PhotoAlbumGrid 等）。
 *
 * - `limit`：当前可见条数上限（调用方自行 `items.slice(0, limit)`）。
 * - `hasMore(total)`：是否还有未挂载条目。
 * - `showMore()`：步进加载下一批。
 * - `reset()`：数据集/筛选条件变化时重置回初始值。
 */
export function useIncrementalWindow(initial: number, step = initial) {
  const [limit, setLimit] = useState(initial);
  const showMore = useCallback(() => setLimit((n) => n + step), [step]);
  const reset = useCallback(() => setLimit(initial), [initial]);
  const hasMore = useCallback((total: number) => total > limit, [limit]);
  return { limit, hasMore, showMore, reset, setLimit };
}
