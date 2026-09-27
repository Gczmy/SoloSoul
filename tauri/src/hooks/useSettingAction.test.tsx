import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SettingWriteResult } from '@/stores/settingsStore';
import { useSettingAction } from './useSettingAction';

const { updateSetting, showToast } = vi.hoisted(() => ({
  updateSetting: vi.fn(),
  showToast: vi.fn(),
}));
vi.mock('@/stores/settingsStore', () => ({
  useSettingsStore: (select: (state: { updateSetting: typeof updateSetting }) => unknown) =>
    select({ updateSetting }),
}));
vi.mock('@/stores/uiStore', () => ({
  useUiStore: (select: (state: { showToast: typeof showToast }) => unknown) =>
    select({ showToast }),
}));

afterEach(cleanup);
beforeEach(() => vi.clearAllMocks());

describe('useSettingAction', () => {
  it('当前失败只提示一次，并保留失败结果供调用方中止成功流程', async () => {
    const failed: SettingWriteResult = { status: 'failed', isCurrent: () => true };
    updateSetting.mockResolvedValue(failed);
    const { result } = renderHook(() => useSettingAction());
    let saved: SettingWriteResult | undefined;
    await act(async () => {
      saved = await result.current('account', 'autoLockOnBackground', true);
    });
    expect(updateSetting).toHaveBeenCalledWith('account', 'autoLockOnBackground', true);
    expect(saved).toBe(failed);
    expect(showToast).toHaveBeenCalledExactlyOnceWith({
      type: 'error',
      message: 'common:save_failed',
    });
  });

  it.each(['saved', 'failed'] as const)('%s 在消费时失效则返回 stale 且不提示', async (status) => {
    let current = true;
    updateSetting.mockImplementation(async () => {
      const result = { status, isCurrent: () => current };
      current = false;
      return result;
    });
    const { result } = renderHook(() => useSettingAction());
    await expect(result.current('account', 'theme', 'dark')).resolves.toEqual({ status: 'stale' });
    expect(showToast).not.toHaveBeenCalled();
  });

  it('store 的 stale 静默透传', async () => {
    updateSetting.mockResolvedValue({ status: 'stale' });
    const { result } = renderHook(() => useSettingAction());
    await expect(result.current('account', 'theme', 'dark')).resolves.toEqual({ status: 'stale' });
    expect(showToast).not.toHaveBeenCalled();
  });

  it('保存成功保留活的守卫，后续 await 后仍能识别过期结果', async () => {
    let current = true;
    const saved: SettingWriteResult = { status: 'saved', isCurrent: () => current };
    updateSetting.mockResolvedValue(saved);
    const { result } = renderHook(() => useSettingAction());
    expect(await result.current('account', 'theme', 'dark')).toBe(saved);
    current = false;
    expect(saved.isCurrent()).toBe(false);
    expect(showToast).not.toHaveBeenCalled();
  });
});
