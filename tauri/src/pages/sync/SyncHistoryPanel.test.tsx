import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SyncHistoryPanel } from './SyncHistoryPanel';
import { formatTimestamp } from '@/lib/time';
import type { SyncResult } from '@/lib/ipc';

const baseResult: SyncResult = {
  summary: 'examined=3, applied=2, skipped=1, conflicts=0',
  examined: 3,
  applied: 2,
  skipped: 1,
  conflicts: [],
  per_table: [{ table: 'object', examined: 3, applied: 2, skipped: 1 }],
  outboundRecords: 0,
  at: Date.now() - 60_000,
  peerName: 'SoloSoul-ab12cd34',
  peerClientType: 'windows',
  peerNodeId: 'node-B',
};

function renderPanel(recentResults: SyncResult[], activityOpen = true) {
  return render(
    <SyncHistoryPanel
      activityOpen={activityOpen}
      recentResults={recentResults}
      onToggleActivity={vi.fn()}
    />,
  );
}

describe('SyncHistoryPanel', () => {
  it('renders nothing when there is no history', () => {
    const { container } = renderPanel([]);
    expect(container.firstChild).toBeNull();
  });

  it('shows direction badge, device name and timestamp for outbound entry', () => {
    renderPanel([baseResult]);
    // 方向徽章（本机发起）：全局 i18n mock 返回 defaultValue
    expect(screen.getByText('Outbound')).toBeInTheDocument();
    // 对端设备名（记录时固化的 peerName）
    expect(screen.getByText('SoloSoul-ab12cd34')).toBeInTheDocument();
    // 时间戳：相对时间渲染 + title 完整本地时间
    expect(
      screen.getByTitle(formatTimestamp(new Date(baseResult.at!).toISOString())),
    ).toBeInTheDocument();
    // 统计行保留（mock 的 t 返回 key；与 outbound 片段同属一个文本块，用正则）
    expect(screen.getByText(/settings:sync_result_stats/)).toBeInTheDocument();
  });

  it('shows inbound badge for peer-initiated entry', () => {
    renderPanel([{ ...baseResult, inbound: true }]);
    expect(screen.getByText('Inbound')).toBeInTheDocument();
    expect(screen.queryByText('Outbound')).not.toBeInTheDocument();
  });

  it('falls back to unknown device label when peerName is missing (legacy history)', () => {
    renderPanel([{ ...baseResult, peerName: undefined }]);
    expect(screen.getByText('Unknown device')).toBeInTheDocument();
  });

  it('renders failed entries with failure label and error summary, without stats', () => {
    renderPanel([
      {
        ...baseResult,
        failed: true,
        errorSummary: '__SYNC_ERR__:connect_failed:timeout',
        examined: 0,
        applied: 0,
        skipped: 0,
        per_table: [],
      },
    ]);
    expect(screen.getByText(/Sync failed/)).toBeInTheDocument();
    // 未映射的原始错误串原样展示（settings ns 测试环境未加载 → resolveBackendErrorMessage 透传）
    expect(screen.getByText(/connect_failed/)).toBeInTheDocument();
    expect(screen.queryByText(/settings:sync_result_stats/)).not.toBeInTheDocument();
  });

  it('hides detail body when activity is collapsed', () => {
    renderPanel([baseResult], false);
    expect(screen.queryByText('SoloSoul-ab12cd34')).not.toBeInTheDocument();
    expect(screen.getByText('Sync Activity')).toBeInTheDocument();
  });
});
