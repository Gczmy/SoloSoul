import { platformCapabilityStore } from '@/lib/platformCapabilities';
import { capabilityFixture } from '@/lib/__fixtures__/platformCapabilityFixture';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { OcrPage } from './OcrPage';

const mockNavigate = vi.fn();
const mockShowToast = vi.fn();
const mockCreateObject = vi.fn();
const mockOpen = vi.fn();
const device = vi.hoisted(() => ({ platform: 'macos', state: {} as { filePath?: string } }));
vi.mock('@/lib/platform', () => ({
  isIOSSync: () => device.platform === 'ios',
  isAndroidSync: () => device.platform === 'android',
  isMobilePlatformSync: () => ['ios', 'android'].includes(device.platform),
  isMacOSSync: () => false,
  isWindowsSync: () => false,
}));

vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({
    children,
    title,
    onBack,
  }: {
    children: React.ReactNode;
    title: string;
    onBack?: () => void;
  }) => (
    <div data-testid="app-shell" data-title={title}>
      {onBack && (
        <button data-testid="back-btn" onClick={onBack}>
          Back
        </button>
      )}
      {children}
    </div>
  ),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
    useLocation: () => ({ state: device.state }),
  };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

vi.mock('@/stores/authStore', () => ({
  useAuthStore: (selector: (s: { currentAccount: { id: string } | null }) => unknown) =>
    selector({ currentAccount: { id: 'test-account' } }),
}));

vi.mock('@/stores/objectStore', () => ({
  // P047 后组件用 store 级选择器订阅（useObjectStore((s) => s.createObject)），
  // mock 需透传 selector 才能返回 createObject 函数本身。
  useObjectStore: (selector?: (s: { createObject: typeof mockCreateObject }) => unknown) => {
    const state = { createObject: mockCreateObject };
    return selector ? selector(state) : state;
  },
}));

vi.mock('@/hooks/useToastError', () => ({
  useToastError: () => ({
    onError: (e: unknown, ctx: string) => {
      mockShowToast({ type: 'error', message: `${ctx}: ${e}` });
    },
    onSuccess: (msg: string) => {
      mockShowToast({ type: 'success', message: msg });
    },
  }),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: () => mockOpen(),
}));

const mockInvoke = vi.mocked(invoke);

// 模块级 prefetch store 单例跨测试共享——重置避免 TTL 缓存跳过 loader（0 calls）
import { prefetchRegistry } from '@/lib/prefetch/registry';

describe('OcrPage', () => {
  beforeEach(() => {
    device.platform = 'macos';
    platformCapabilityStore.setState({ capabilities: capabilityFixture('macos'), loaded: true });
    device.state = {};
    prefetchRegistry.ocrModel.reset();
    vi.clearAllMocks();
    mockInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'ocr_list_available_tiers')
        return [
          { tier: 'tiny', name: 'Tiny', description: 'Fast' },
          { tier: 'small', name: 'Small', description: 'Default' },
          { tier: 'medium', name: 'Medium', description: 'Accurate' },
        ];
      if (cmd === 'ocr_get_active_tier') return 'small';
      if (cmd === 'ocr_get_model_status')
        return {
          tier: ((args as Record<string, unknown>).tier as string) ?? 'small',
          installed: true,
          bundled: true,
        };
      return undefined;
    });
  });

  it('renders scanner title and select image button', async () => {
    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    expect(screen.getByTestId('app-shell')).toHaveAttribute('data-title', 'ocr:title');
    expect(await screen.findByText('ocr:select_image_or_pdf')).toBeInTheDocument();
  });

  it('loads model tiers and status on mount', async () => {
    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('ocr_list_available_tiers');
      expect(mockInvoke).toHaveBeenCalledWith('ocr_get_active_tier');
      expect(mockInvoke).toHaveBeenCalledWith('ocr_get_model_status', { tier: 'small' });
    });
  });

  it('scans selected image and displays result', async () => {
    mockOpen.mockResolvedValue('/test/image.png');
    mockInvoke.mockImplementation(async (cmd: string, _args?: unknown) => {
      if (cmd === 'ocr_scan_image')
        return {
          text: 'Hello World',
          confidence: 0.95,
          boxes: [{ text: 'Hello World', confidence: 0.95, points: [] }],
        };
      if (cmd === 'ocr_list_available_tiers')
        return [
          { tier: 'tiny', name: 'Tiny', description: 'Fast' },
          { tier: 'small', name: 'Small', description: 'Default' },
          { tier: 'medium', name: 'Medium', description: 'Accurate' },
        ];
      if (cmd === 'ocr_get_active_tier') return 'small';
      if (cmd === 'ocr_get_model_status') return { tier: 'small', installed: true, bundled: true };
      return undefined;
    });

    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('ocr:select_image_or_pdf')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('ocr:select_image_or_pdf'));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'ocr_scan_image',
        expect.objectContaining({
          filePath: '/test/image.png',
          taskId: expect.stringMatching(/^[0-9a-f-]{36}$/i),
        }),
      );
    });

    const results = await screen.findAllByText('Hello World');
    expect(results.length).toBeGreaterThanOrEqual(1);
  });

  it('imports scan result as object', async () => {
    mockOpen.mockResolvedValue('/test/image.png');
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'ocr_scan_image')
        return {
          text: 'Hello World',
          confidence: 0.95,
          boxes: [{ text: 'Hello World', confidence: 0.95, points: [] }],
        };
      if (cmd === 'ocr_list_available_tiers')
        return [
          { tier: 'tiny', name: 'Tiny', description: 'Fast' },
          { tier: 'small', name: 'Small', description: 'Default' },
          { tier: 'medium', name: 'Medium', description: 'Accurate' },
        ];
      if (cmd === 'ocr_get_active_tier') return 'small';
      if (cmd === 'ocr_get_model_status') return { tier: 'small', installed: true, bundled: true };
      return undefined;
    });
    mockCreateObject.mockResolvedValue({});

    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    fireEvent.click(await screen.findByText('ocr:select_image_or_pdf'));
    await screen.findAllByText('Hello World');

    fireEvent.click(screen.getByText('ocr:import_as_object'));

    // 新流程：先弹出名称输入框，输入自定义名称后确认
    const dialog = await screen.findByRole('dialog');
    expect(dialog).toBeInTheDocument();

    const input = within(dialog).getByRole('textbox');
    fireEvent.change(input, { target: { value: 'My OCR Object' } });

    fireEvent.click(within(dialog).getByTestId('prompt-dialog-confirm'));

    await waitFor(() => {
      expect(mockCreateObject).toHaveBeenCalledWith({
        accountId: 'test-account',
        name: 'My OCR Object',
        typeId: 'document',
        properties: {
          ocrText: 'Hello World',
          __fields: {
            ocrText: {
              name: expect.any(String),
              type: 'multiline',
              sensitivityLevel: 'internal',
            },
          },
        },
      });
    });
  });

  it('shows not-installed toast when active model is not installed', async () => {
    mockInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'ocr_list_available_tiers')
        return [
          { tier: 'tiny', name: 'Tiny', description: 'Fast' },
          { tier: 'small', name: 'Small', description: 'Default' },
          { tier: 'medium', name: 'Medium', description: 'Accurate' },
        ];
      if (cmd === 'ocr_get_active_tier') return 'tiny';
      if (cmd === 'ocr_get_model_status')
        return {
          tier: ((args as Record<string, unknown>).tier as string) ?? 'tiny',
          installed: false,
          bundled: true,
        };
      if (cmd === 'ocr_set_active_tier') return undefined;
      return undefined;
    });
    mockOpen.mockResolvedValue('/test/image.png');

    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    fireEvent.click(await screen.findByText('ocr:select_image_or_pdf'));

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          message: expect.stringContaining('ocr:scan_model_not_installed'),
        }),
      );
    });
    expect(mockInvoke).not.toHaveBeenCalledWith('ocr_scan_image', expect.anything());
  });

  it('shows error toast when scan fails', async () => {
    mockOpen.mockResolvedValue('/test/image.png');
    mockInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'ocr_scan_image') throw new Error('model not found');
      if (cmd === 'ocr_list_available_tiers')
        return [
          { tier: 'tiny', name: 'Tiny', description: 'Fast' },
          { tier: 'small', name: 'Small', description: 'Default' },
          { tier: 'medium', name: 'Medium', description: 'Accurate' },
        ];
      if (cmd === 'ocr_get_active_tier') return 'small';
      if (cmd === 'ocr_get_model_status')
        return {
          tier: ((args as Record<string, unknown>).tier as string) ?? 'small',
          installed: true,
          bundled: true,
        };
      return undefined;
    });
    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    fireEvent.click(await screen.findByText('ocr:select_image_or_pdf'));

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          message: expect.stringContaining('ocr:scan_failed'),
        }),
      );
    });
  });
  it('RF-203 iOS 禁用选图、拍照和模式切换并说明原因', async () => {
    device.platform = 'ios';
    platformCapabilityStore.setState({ capabilities: capabilityFixture('ios'), loaded: true });
    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button', { name: 'ocr:select_image' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'ocr:take_photo' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'ocr:scan_mode_mrz' })).toBeDisabled();
    expect(screen.getByText('ocr:ios_ocr_unsupported')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'ocr:select_image' }));
    fireEvent.click(screen.getByRole('button', { name: 'ocr:take_photo' }));
    expect(mockOpen).not.toHaveBeenCalled();
    expect(
      mockInvoke.mock.calls.some(([cmd]) =>
        ['ocr_scan_image', 'ocr_scan_mrz', 'mobile_ocr_take_photo'].includes(cmd),
      ),
    ).toBe(false);
  });
  it('RF-203 iOS 传入附件路径也不自动扫描或进入 loading', async () => {
    device.platform = 'ios';
    platformCapabilityStore.setState({ capabilities: capabilityFixture('ios'), loaded: true });
    device.state = { filePath: '/test/attachment.png' };
    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText('ocr:ios_ocr_unsupported')).toBeVisible());
    expect(
      mockInvoke.mock.calls.some(([cmd]) => ['ocr_scan_image', 'ocr_scan_mrz'].includes(cmd)),
    ).toBe(false);
    expect(screen.queryByText('ocr:scanning')).not.toBeInTheDocument();
  });
  it('RF-203 Android 保留选图与拍照入口', () => {
    device.platform = 'android';
    platformCapabilityStore.setState({ capabilities: capabilityFixture('android'), loaded: true });
    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button', { name: 'ocr:select_image' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'ocr:take_photo' })).toBeEnabled();
    expect(screen.queryByText('ocr:ios_ocr_unsupported')).not.toBeInTheDocument();
  });
});
