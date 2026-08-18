import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: vi.fn(),
}));

import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useOcrModelManager } from './useOcrModelManager';
import { prefetchRegistry } from '@/lib/prefetch/registry';

const tMock = ((key: string, opts?: Record<string, unknown>) =>
  String(key) + (opts ? JSON.stringify(opts) : '')) as never;

const tiers = [
  { tier: 'tiny', name: 'Tiny', sizeBytes: 1, bundled: true },
  { tier: 'small', name: 'Small', sizeBytes: 2, bundled: true },
];

function makeOpts(overrides: Record<string, unknown> = {}) {
  return {
    t: tMock,
    onError: vi.fn(),
    onTierChangeSuccess: vi.fn(),
    onInstallSuccess: vi.fn(),
    onDownloadSuccess: vi.fn(),
    onDeleteSuccess: vi.fn(),
    ...overrides,
  };
}

describe('useOcrModelManager', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // 模块级 prefetch store 单例跨测试共享——重置避免缓存污染（TTL 命中会跳过 loader）
    prefetchRegistry.ocrModel.reset();
    vi.mocked(invoke)
      .mockResolvedValueOnce(tiers) // ocr_list_available_tiers
      .mockResolvedValueOnce('small') // ocr_get_active_tier
      .mockResolvedValueOnce({ installed: true, bundled: true }) // tiny status
      .mockResolvedValueOnce({ installed: true, bundled: true }); // small status
  });

  it('加载 tier 列表与状态', async () => {
    const opts = makeOpts();
    const { result } = renderHook(() => useOcrModelManager(opts as never));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.tiers).toEqual(tiers);
    expect(result.current.activeTier).toBe('small');
    expect(result.current.statusMap['tiny']).toEqual({ installed: true, bundled: true });
  });

  it('加载失败时调用 onError', async () => {
    const opts = makeOpts();
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockRejectedValue(new Error('boom'));
    const { result } = renderHook(() => useOcrModelManager(opts as never));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(opts.onError).toHaveBeenCalledWith(
      expect.any(Error),
      expect.stringContaining('load_status_failed'),
    );
  });

  it('切换档位成功后回调 onTierChangeSuccess', async () => {
    const opts = makeOpts();
    const { result } = renderHook(() => useOcrModelManager(opts as never));
    await waitFor(() => expect(result.current.loading).toBe(false));

    vi.mocked(invoke).mockClear();
    vi.mocked(invoke).mockResolvedValue(undefined);
    await act(async () => {
      await result.current.handleTierChange('tiny');
    });
    expect(invoke).toHaveBeenCalledWith('ocr_set_active_tier', { tier: 'tiny' });
    expect(result.current.activeTier).toBe('tiny');
    expect(opts.onTierChangeSuccess).toHaveBeenCalled();
    expect(opts.onError).not.toHaveBeenCalled();
  });

  it('安装 bundled 模型后刷新列表并回调 onInstallSuccess', async () => {
    const opts = makeOpts();
    const { result } = renderHook(() => useOcrModelManager(opts as never));
    await waitFor(() => expect(result.current.loading).toBe(false));

    vi.mocked(invoke).mockClear();
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // ocr_install_bundled_model
      .mockResolvedValueOnce(tiers) // reload: list
      .mockResolvedValueOnce('small') // reload: active
      .mockResolvedValueOnce({ installed: true, bundled: true })
      .mockResolvedValueOnce({ installed: true, bundled: true });

    await act(async () => {
      await result.current.handleInstallBundled('tiny');
    });
    expect(opts.onInstallSuccess).toHaveBeenCalled();
    expect(opts.onError).not.toHaveBeenCalled();
    expect(result.current.installingTier).toBeNull();
  });

  it('下载：无 confirmDownload 时直接下载并回调 onDownloadSuccess', async () => {
    const opts = makeOpts();
    const { result } = renderHook(() => useOcrModelManager(opts as never));
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => result.current.setDownloadUrl('https://example.com/model'));
    vi.mocked(invoke).mockClear();
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // ocr_download_model
      .mockResolvedValueOnce(tiers) // reload: list
      .mockResolvedValueOnce('small')
      .mockResolvedValueOnce({ installed: true, bundled: true })
      .mockResolvedValueOnce({ installed: true, bundled: true });

    await act(async () => {
      await result.current.handleDownload('tiny');
    });
    expect(invoke).toHaveBeenCalledWith('ocr_download_model', {
      tier: 'tiny',
      baseUrl: 'https://example.com/model',
    });
    expect(opts.onDownloadSuccess).toHaveBeenCalled();
  });

  it('下载：提供 confirmDownload 时先确认再下载，取消则不下发', async () => {
    const confirmDownload = vi.fn((_params: { onConfirm: () => void }) => {
      // 模拟用户取消：不调用 onConfirm
    });
    const opts = makeOpts({ confirmDownload });
    const { result } = renderHook(() => useOcrModelManager(opts as never));
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => result.current.setDownloadUrl('https://example.com/model'));
    const invokeBefore = vi.mocked(invoke).mock.calls.length;

    await act(async () => {
      await result.current.handleDownload('tiny');
    });
    expect(confirmDownload).toHaveBeenCalled();
    expect(vi.mocked(invoke).mock.calls.length).toBe(invokeBefore); // 未发起下载
  });

  it('下载：confirmDownload 确认后下发并回调 onDownloadSuccess', async () => {
    const confirmDownload = vi.fn(({ onConfirm }) => onConfirm());
    const opts = makeOpts({ confirmDownload });
    const { result } = renderHook(() => useOcrModelManager(opts as never));
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => result.current.setDownloadUrl('https://example.com/model'));
    vi.mocked(invoke).mockClear();
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // ocr_download_model
      .mockResolvedValueOnce(tiers)
      .mockResolvedValueOnce('small')
      .mockResolvedValueOnce({ installed: true, bundled: true })
      .mockResolvedValueOnce({ installed: true, bundled: true });

    await act(async () => {
      await result.current.handleDownload('tiny');
    });
    expect(vi.mocked(invoke).mock.calls.some((c) => c[0] === 'ocr_download_model')).toBe(true);
    expect(opts.onDownloadSuccess).toHaveBeenCalled();
  });

  it('下载：URL 为空时不发请求并报错', async () => {
    const opts = makeOpts();
    const { result } = renderHook(() => useOcrModelManager(opts as never));
    await waitFor(() => expect(result.current.loading).toBe(false));

    vi.mocked(invoke).mockClear();
    await act(async () => {
      await result.current.handleDownload('tiny');
    });
    expect(vi.mocked(invoke).mock.calls.some((c) => c[0] === 'ocr_download_model')).toBe(false);
    expect(opts.onError).toHaveBeenCalled();
  });

  it('删除模型后刷新并回调 onDeleteSuccess', async () => {
    const opts = makeOpts();
    const { result } = renderHook(() => useOcrModelManager(opts as never));
    await waitFor(() => expect(result.current.loading).toBe(false));

    vi.mocked(invoke).mockClear();
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // ocr_delete_model
      .mockResolvedValueOnce(tiers)
      .mockResolvedValueOnce('small')
      .mockResolvedValueOnce({ installed: false, bundled: true })
      .mockResolvedValueOnce({ installed: true, bundled: true });

    await act(async () => {
      await result.current.handleDelete('tiny');
    });
    expect(opts.onDeleteSuccess).toHaveBeenCalled();
    expect(opts.onError).not.toHaveBeenCalled();
    expect(result.current.deletingTier).toBeNull();
  });

  it('enabled=false 时不加载', async () => {
    const opts = makeOpts({ enabled: false });
    const { result } = renderHook(() => useOcrModelManager(opts as never));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(invoke).not.toHaveBeenCalled();
  });
});
