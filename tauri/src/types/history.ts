/**
 * 对象历史快照条目（P037 收敛单一来源）。
 * 此前在 trash/types.ts、HistoryViewer.tsx、HistoryPage.tsx 各定义一份。
 */
export interface SnapshotEntry {
  id: string;
  timestamp: number;
  triggeredBy: string;
  diffSummary: string;
}
