import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: vi.fn(),
}));

import { invokeCommand } from '@/lib/ipcClient';
import { pickVaultDirectory, VaultDirPickLostError } from './vaultDirectory';

/** 模拟页面可见性切换（SAF 选择器打开 → 页面隐藏；选择器关闭 → 重新可见）。 */
function setVisibility(state: DocumentVisibilityState) {
  Object.defineProperty(document, 'visibilityState', { value: state, configurable: true });
  document.dispatchEvent(new Event('visibilitychange'));
}

describe('pickVaultDirectory（SAF 选择器 + 结果丢失兜底）', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(invokeCommand).mockReset();
    setVisibility('visible');
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('正常返回选择的 uri', async () => {
    vi.mocked(invokeCommand).mockResolvedValue({ uri: 'content://tree/abc' } as never);
    await expect(pickVaultDirectory()).resolves.toBe('content://tree/abc');
  });

  it('用户取消（返回空对象）时返回 null', async () => {
    vi.mocked(invokeCommand).mockResolvedValue({} as never);
    await expect(pickVaultDirectory()).resolves.toBeNull();
  });

  it('选择器关闭后结果正常投递：页面重新可见后 invoke 返回，不误触发兜底', async () => {
    let resolveInvoke!: (v: { uri?: string | null }) => void;
    vi.mocked(invokeCommand).mockReturnValue(
      new Promise((resolve) => {
        resolveInvoke = resolve;
      }) as never,
    );

    const promise = pickVaultDirectory();
    // 选择器打开 → 隐藏；选择器关闭 → 重新可见，结果随即返回
    setVisibility('hidden');
    setVisibility('visible');
    resolveInvoke({ uri: 'content://tree/late' });

    await expect(promise).resolves.toBe('content://tree/late');
    // 宽限期过后仍不应被兜底拒绝
    await vi.advanceTimersByTimeAsync(10000);
  });

  it('结果丢失（invoke 永不返回）：重新可见且超过宽限期后抛 VaultDirPickLostError', async () => {
    vi.mocked(invokeCommand).mockReturnValue(new Promise(() => {}) as never);

    const promise = pickVaultDirectory();
    setVisibility('hidden');
    setVisibility('visible');

    // 宽限期（8 秒）内不触发
    await vi.advanceTimersByTimeAsync(7000);
    let rejected = false;
    promise.catch(() => {
      rejected = true;
    });
    await Promise.resolve();
    expect(rejected).toBe(false);

    // 超过宽限期 → 判定结果丢失
    await vi.advanceTimersByTimeAsync(3000);
    await expect(promise).rejects.toBeInstanceOf(VaultDirPickLostError);
  });

  it('页面始终可见（选择器未打开）时不触发兜底，invoke 正常返回', async () => {
    let resolveInvoke!: (v: { uri?: string | null }) => void;
    vi.mocked(invokeCommand).mockReturnValue(
      new Promise((resolve) => {
        resolveInvoke = resolve;
      }) as never,
    );

    const promise = pickVaultDirectory();
    await vi.advanceTimersByTimeAsync(20000);
    let settled = false;
    promise.then(() => {
      settled = true;
    });
    await Promise.resolve();
    expect(settled).toBe(false);

    resolveInvoke({ uri: 'content://tree/normal' });
    await expect(promise).resolves.toBe('content://tree/normal');
  });

  it('P020：桌面端正常返回后 visibilitychange 监听器被移除（不累积）', async () => {
    const addSpy = vi.spyOn(document, 'addEventListener');
    const removeSpy = vi.spyOn(document, 'removeEventListener');
    // 桌面端：系统对话框不触发 visibility 变化，invoke 直接返回
    vi.mocked(invokeCommand).mockResolvedValue({ uri: 'content://tree/desktop' } as never);
    await pickVaultDirectory();

    const added = addSpy.mock.calls.filter(([type]) => type === 'visibilitychange');
    const removed = removeSpy.mock.calls.filter(([type]) => type === 'visibilitychange');
    expect(added.length).toBe(1);
    expect(removed.length).toBe(1);
    addSpy.mockRestore();
    removeSpy.mockRestore();
  });
});
