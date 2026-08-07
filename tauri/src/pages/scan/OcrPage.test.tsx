import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { OcrPage } from './OcrPage';

const mockNavigate = vi.fn();
const mockShowToast = vi.fn();
const mockCreateObject = vi.fn();
const mockOpen = vi.fn();

vi.mock('@/components/layout/AppShell', () => ({
  AppShell: ({
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
    useLocation: () => ({ state: {} }),
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
  useObjectStore: () => ({
    createObject: mockCreateObject,
  }),
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

describe('OcrPage', () => {
  beforeEach(() => {
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
      expect(mockInvoke).toHaveBeenCalledWith('ocr_scan_image', { filePath: '/test/image.png' });
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
        collectionType: 'document',
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
});
