import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SyncShowQrDialog } from './SyncShowQrDialog';

const { stableT, mockInvoke } = vi.hoisted(() => ({
  stableT: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
  mockInvoke: vi.fn(),
}));

// 覆写 setup.ts 的 react-i18next mock：提供「稳定」t 引用。
// SyncShowQrDialog 的加载 effect 依赖 [isOpen, t]，若 t 每次渲染都是新引用，
// effect 会在每次渲染后重跑并重复发起 IPC，导致测试无法收敛。
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: stableT,
    i18n: { language: 'en', changeLanguage: vi.fn(() => Promise.resolve()) },
  }),
  I18nextProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: (...args: unknown[]) => mockInvoke(...args),
}));

const SYNC_PAYLOAD = JSON.stringify({ a: '192.168.1.5:42069', f: 'AA:BB:CC', n: 'MacBook' });
const RECOVERY_INFO = {
  displayAddr: '10.0.0.2:42069',
  bindAddr: '10.0.0.2:42069',
  pin: '1234',
  qrPayload: 'qr-payload',
};

const mockSyncInvokes = () => {
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === 'sync_generate_qr_payload') return Promise.resolve(SYNC_PAYLOAD);
    if (cmd === 'recovery_host_start') return Promise.resolve(RECOVERY_INFO);
    return Promise.resolve(undefined);
  });
};

describe('SyncShowQrDialog', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('isOpen=false 时不渲染内容', () => {
    render(<SyncShowQrDialog isOpen={false} onClose={vi.fn()} />);
    expect(screen.queryByLabelText('common:close')).not.toBeInTheDocument();
    expect(screen.queryByText('Sync QR')).not.toBeInTheDocument();
  });

  it('打开后加载同步二维码并显示设备信息（SyncQrContent 提取后链路完整）', async () => {
    mockSyncInvokes();
    render(<SyncShowQrDialog isOpen onClose={vi.fn()} />);

    expect(mockInvoke).toHaveBeenCalledWith('sync_generate_qr_payload');
    await waitFor(() => {
      expect(screen.getByText('MacBook')).toBeInTheDocument();
    });
    expect(screen.getByText('AA:BB:CC')).toBeInTheDocument();
    expect(screen.getByText('192.168.1.5:42069')).toBeInTheDocument();
  });

  it('切换到恢复二维码 Tab 启动恢复会话并显示 PIN（RecoveryQrContent 提取后链路完整）', async () => {
    mockSyncInvokes();
    render(<SyncShowQrDialog isOpen onClose={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByText('MacBook')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Recovery QR'));

    await waitFor(() => {
      expect(screen.getByText('1234')).toBeInTheDocument();
    });
    expect(mockInvoke).toHaveBeenCalledWith('recovery_host_start');
    // 手动输入指引折叠面板默认关闭，可展开
    expect(screen.getByText('No camera? Enter details manually')).toBeInTheDocument();
  });

  it('同步二维码加载失败时显示错误占位（QrStatusBlock 错误分支）', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'sync_generate_qr_payload') {
        return Promise.reject(new Error('QR backend unavailable'));
      }
      return Promise.resolve(undefined);
    });
    render(<SyncShowQrDialog isOpen onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('QR backend unavailable')).toBeInTheDocument();
    });
  });

  it('点击关闭按钮触发 onClose', async () => {
    mockSyncInvokes();
    const onClose = vi.fn();
    render(<SyncShowQrDialog isOpen onClose={onClose} />);
    fireEvent.click(screen.getByLabelText('common:close'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('ignores an old recovery host response after the dialog closes and reopens', async () => {
    let resolveOld!: (value: typeof RECOVERY_INFO) => void;
    let resolveNew!: (value: typeof RECOVERY_INFO) => void;
    const oldRequest = new Promise<typeof RECOVERY_INFO>((resolve) => {
      resolveOld = resolve;
    });
    const newRequest = new Promise<typeof RECOVERY_INFO>((resolve) => {
      resolveNew = resolve;
    });
    let starts = 0;
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'sync_generate_qr_payload') return Promise.resolve(SYNC_PAYLOAD);
      if (cmd === 'recovery_host_start') return ++starts === 1 ? oldRequest : newRequest;
      return Promise.resolve(undefined);
    });

    const { rerender } = render(<SyncShowQrDialog isOpen onClose={vi.fn()} />);
    await screen.findByText('MacBook');
    fireEvent.click(screen.getByText('Recovery QR'));
    await waitFor(() => expect(starts).toBe(1));

    rerender(<SyncShowQrDialog isOpen={false} onClose={vi.fn()} />);
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('recovery_host_cancel'));
    rerender(<SyncShowQrDialog isOpen onClose={vi.fn()} />);
    await screen.findByText('MacBook');
    fireEvent.click(screen.getByText('Recovery QR'));
    await waitFor(() => expect(starts).toBe(2));

    await act(async () => resolveNew({ ...RECOVERY_INFO, pin: '2222' }));
    expect(screen.getByText('2222')).toBeInTheDocument();
    await act(async () => resolveOld({ ...RECOVERY_INFO, pin: '1111' }));
    expect(screen.getByText('2222')).toBeInTheDocument();
    expect(screen.queryByText('1111')).not.toBeInTheDocument();
  });

  it('ignores an old sync QR payload after the dialog closes and reopens', async () => {
    let resolveOld!: (value: string) => void;
    let resolveNew!: (value: string) => void;
    const oldRequest = new Promise<string>((resolve) => {
      resolveOld = resolve;
    });
    const newRequest = new Promise<string>((resolve) => {
      resolveNew = resolve;
    });
    let requests = 0;
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'sync_generate_qr_payload') return ++requests === 1 ? oldRequest : newRequest;
      return Promise.resolve(undefined);
    });

    const { rerender } = render(<SyncShowQrDialog isOpen onClose={vi.fn()} />);
    await waitFor(() => expect(requests).toBe(1));
    rerender(<SyncShowQrDialog isOpen={false} onClose={vi.fn()} />);
    rerender(<SyncShowQrDialog isOpen onClose={vi.fn()} />);
    await waitFor(() => expect(requests).toBe(2));

    await act(async () => resolveNew(JSON.stringify({ a: '10.0.0.2', n: 'New device' })));
    expect(screen.getByText('New device')).toBeInTheDocument();
    await act(async () => resolveOld(JSON.stringify({ a: '10.0.0.1', n: 'Old device' })));
    expect(screen.getByText('New device')).toBeInTheDocument();
    expect(screen.queryByText('Old device')).not.toBeInTheDocument();
  });
});
