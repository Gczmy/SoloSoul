import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter } from 'react-router-dom';

// 真实对话框初始视图（tab='scan'）会挂载 RecoveryScanView → RecoveryQrScanner →
// html5-qrcode getCameras()，jsdom 无 MediaStreamTrack 会抛未处理异常。仅 mock 扫描
// 视图，其余全部保持真实模块（Card/标题/useRecoveryReceive），以覆盖懒加载映射本身。
vi.mock('@/components/recovery/RecoveryScanView', () => ({
  RecoveryScanView: () => <div data-testid="mock-scan-view" />,
}));

import { LazyRecoveryReceiveDialog } from './LazyRecoveryReceiveDialog';

describe('LazyRecoveryReceiveDialog', () => {
  it('resolves to the real RecoveryReceiveDialog (named-export mapping intact)', async () => {
    render(
      // 真实 useRecoveryReceive 调用 useNavigate()，需要 Router 上下文
      <MemoryRouter>
        <Suspense fallback={<div>lazy-loading</div>}>
          <LazyRecoveryReceiveDialog isOpen onClose={() => {}} />
        </Suspense>
      </MemoryRouter>,
    );

    // 若 RecoveryReceiveDialog 命名导出被重命名，lazy 工厂的 m.RecoveryReceiveDialog
    // 解析为 undefined → React 抛 "Element type is invalid" → 本测试失败（漂移保护）。
    expect(await screen.findByText(/recovery_receive_title/i)).toBeInTheDocument();
    expect(screen.getByTestId('mock-scan-view')).toBeInTheDocument();
  });
});
