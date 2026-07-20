import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';

// Mock settingsStore 的模块级依赖（@tauri-apps/api/core 已在 test/setup.ts 全局 mock）
vi.mock('@/lib/theme', () => ({
  applyTheme: vi.fn(),
}));

vi.mock('@/lib/i18n', () => ({
  __esModule: true,
  default: { changeLanguage: vi.fn(() => Promise.resolve()) },
  detectSystemLanguage: vi.fn(() => 'en-US'),
}));

import { invoke } from '@tauri-apps/api/core';
import { useAutoLock } from './useAutoLock';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useVaultStore } from '@/stores/vaultStore';
import { useAutoLockPauseStore } from '@/stores/autoLockPauseStore';

const MIN = 60_000;

function setTimeoutMinutes(minutes: number) {
  useSettingsStore.setState({
    settings: { ...useSettingsStore.getState().settings, autoLockTimeoutMinutes: minutes },
  });
}

describe('useAutoLock', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(invoke).mockResolvedValue(undefined);
    useAuthStore.setState({ isAuthenticated: true });
    useVaultStore.setState({ vaultState: 'unlocked' });
    useAutoLockPauseStore.setState({ pauseCount: 0 });
    setTimeoutMinutes(5);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('无活动超过阈值后调用 lock', () => {
    renderHook(() => useAutoLock());

    vi.advanceTimersByTime(4 * MIN + 59_000);
    expect(invoke).not.toHaveBeenCalledWith('lock');

    vi.advanceTimersByTime(10_000);
    expect(invoke).toHaveBeenCalledWith('lock');
  });

  it('锁定只触发一次', () => {
    renderHook(() => useAutoLock());

    vi.advanceTimersByTime(20 * MIN);
    const lockCalls = vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === 'lock');
    expect(lockCalls).toHaveLength(1);
  });

  it('用户活动重置闲置计时', () => {
    renderHook(() => useAutoLock());

    vi.advanceTimersByTime(4 * MIN);
    window.dispatchEvent(new Event('mousemove'));
    vi.advanceTimersByTime(4 * MIN);
    expect(invoke).not.toHaveBeenCalledWith('lock');

    vi.advanceTimersByTime(2 * MIN);
    expect(invoke).toHaveBeenCalledWith('lock');
  });

  it('autoLockTimeoutMinutes 为 0（从不）时不锁定', () => {
    setTimeoutMinutes(0);
    renderHook(() => useAutoLock());

    vi.advanceTimersByTime(60 * MIN);
    expect(invoke).not.toHaveBeenCalledWith('lock');
  });

  it('未认证时不锁定', () => {
    useAuthStore.setState({ isAuthenticated: false });
    renderHook(() => useAutoLock());

    vi.advanceTimersByTime(60 * MIN);
    expect(invoke).not.toHaveBeenCalledWith('lock');
  });

  it('暂停期间（密码验证框打开）不锁定，恢复后重新计时', () => {
    renderHook(() => useAutoLock());

    useAutoLockPauseStore.getState().pause();
    vi.advanceTimersByTime(10 * MIN);
    expect(invoke).not.toHaveBeenCalledWith('lock');

    useAutoLockPauseStore.getState().resume();
    vi.advanceTimersByTime(4 * MIN);
    expect(invoke).not.toHaveBeenCalledWith('lock');

    vi.advanceTimersByTime(2 * MIN);
    expect(invoke).toHaveBeenCalledWith('lock');
  });

  it('回到前台时如果未被隐藏事件锁定则检查闲置（罕见情况）', () => {
    renderHook(() => useAutoLock());
    vi.mocked(invoke).mockClear();

    // 直接 dispatch visible（假设之前没有被 hidden 锁定）
    vi.setSystemTime(Date.now() + 6 * MIN);
    Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true });
    document.dispatchEvent(new Event('visibilitychange'));

    expect(invoke).toHaveBeenCalledWith('lock');
  });

  it('切到后台（visibility hidden）时立即锁定', () => {
    renderHook(() => useAutoLock());
    vi.mocked(invoke).mockClear();

    Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true });
    document.dispatchEvent(new Event('visibilitychange'));

    expect(invoke).toHaveBeenCalledWith('lock');
  });

  it('切到后台只锁定一次', () => {
    renderHook(() => useAutoLock());
    vi.mocked(invoke).mockClear();

    Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true });
    document.dispatchEvent(new Event('visibilitychange'));

    expect(invoke).toHaveBeenCalledWith('lock');
    vi.mocked(invoke).mockClear();

    // 再次 hidden 不应重复锁定
    document.dispatchEvent(new Event('visibilitychange'));
    expect(invoke).not.toHaveBeenCalled();
  });

  it('设置变更后按新阈值生效', () => {
    renderHook(() => useAutoLock());

    act(() => setTimeoutMinutes(1));

    vi.advanceTimersByTime(1 * MIN + 10_000);
    expect(invoke).toHaveBeenCalledWith('lock');
  });
});
