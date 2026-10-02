import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useApplyThemeFromSettings } from './useApplyThemeFromSettings';
import { applyTheme, getSystemTheme } from '@/lib/theme';

import { useSettingsStore } from '@/stores/settingsStore';
const initial = useSettingsStore.getState().settings;
const setTheme = (theme: 'system' | 'light' | 'dark') =>
  useSettingsStore.setState({
    settings: { ...initial, theme, accentColor: 'custom', customAccentHex: '#777777' },
  });

vi.mock('@/lib/theme', () => ({
  applyTheme: vi.fn(async () => {}),
  getSystemTheme: vi.fn(async () => 'dark'),
  listenForSystemTheme: vi.fn(async () => () => {}),
}));

afterEach(() => {
  cleanup();
  useSettingsStore.setState({ settings: initial });
  vi.clearAllMocks();
});

describe('登录/锁定页重新挂载时恢复强调色', () => {
  it.each(['light', 'dark', 'system'] as const)('%s 模式携带保存的自定义色', async (theme) => {
    setTheme(theme);
    renderHook(() => useApplyThemeFromSettings());
    await waitFor(() =>
      expect(applyTheme).toHaveBeenCalledWith(
        expect.objectContaining({
          accentColor: 'custom',
          customAccentHex: '#777777',
          resolvedSystemTheme: theme === 'system' ? 'dark' : undefined,
        }),
        expect.any(Function),
      ),
    );
  });
});

describe('RF-112 旧异步系统主题不得回填', () => {
  it('挂载后设置已切到 light，旧 system 解析不得应用', async () => {
    setTheme('system');
    let resolve!: (mode: 'light' | 'dark') => void;
    vi.mocked(getSystemTheme).mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    renderHook(() => useApplyThemeFromSettings());
    expect(getSystemTheme).toHaveBeenCalled();
    act(() => setTheme('light'));
    await act(async () => resolve('dark'));
    expect(vi.mocked(applyTheme).mock.calls.some(([config]) => config.preset === 'system')).toBe(
      false,
    );
  });

  it('卸载后到达的 system 解析不得应用', async () => {
    setTheme('system');
    let resolve!: (mode: 'light' | 'dark') => void;
    vi.mocked(getSystemTheme).mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const hook = renderHook(() => useApplyThemeFromSettings());
    hook.unmount();
    await act(async () => resolve('dark'));
    expect(applyTheme).not.toHaveBeenCalled();
  });
});
