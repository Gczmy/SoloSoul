import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useExportExecution, type ExportScopeSnapshot } from './useExportExecution';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  onError: vi.fn(),
  onSuccess: vi.fn(),
}));

vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('@/hooks/useToastError', () => ({
  useToastError: () => ({ onError: mocks.onError, onSuccess: mocks.onSuccess }),
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const scope: ExportScopeSnapshot = {
  selectedPageIds: new Set(),
  selectedObjectIds: new Set(['object-1']),
  selectedTags: new Set(),
  includeAttachments: false,
  selectedAttachmentIds: new Set(),
  includePreferences: false,
  includeBehavioral: false,
};

beforeEach(() => {
  vi.clearAllMocks();
});

describe('RF-1012 ordinary export execution', () => {
  it('starts only one export when the action fires twice before React rerenders', async () => {
    let resolveExport!: (path: string) => void;
    const pendingExport = new Promise<string>((resolve) => {
      resolveExport = resolve;
    });
    mocks.invoke.mockImplementation((command: string) => {
      if (command === 'export_execute') return pendingExport;
      throw new Error('Unexpected IPC: ' + command);
    });
    const { result } = renderHook(() =>
      useExportExecution({
        accountId: 'account',
        cloudTargets: [],
        scope,
        totalSelected: 1,
      }),
    );
    act(() => {
      result.current.setExportPassword('export-password');
      result.current.setExportPasswordConfirm('export-password');
      result.current.setSavePath('C:/fixture.solosoul');
    });
    let first!: Promise<void>;
    let second!: Promise<void>;
    act(() => {
      first = result.current.handleExport();
      second = result.current.handleExport();
    });
    await waitFor(() =>
      expect(
        mocks.invoke.mock.calls.filter(([command]) => command === 'export_execute'),
      ).toHaveLength(1),
    );
    await act(async () => {
      resolveExport('C:/fixture.solosoul');
      await Promise.all([first, second]);
    });
    expect(mocks.onSuccess).toHaveBeenCalledOnce();
  });
});
