import { cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useApplyThemeFromSettings } from './useApplyThemeFromSettings';
import { applyTheme } from '@/lib/theme';

const { settings } = vi.hoisted(() => ({
  settings: {
    theme: 'system' as 'system' | 'light' | 'dark',
    accentColor: 'custom',
    customAccentHex: '#777777',
    backgroundType: 'solid',
    backgroundValue: '',
    defaultLightTheme: 'warm-stone',
    defaultDarkTheme: 'warm-stone-dark',
  },
}));

vi.mock('@/stores/settingsStore', () => ({
  useSettingsStore: { getState: () => ({ settings }) },
}));
vi.mock('@/lib/theme', () => ({
  applyTheme: vi.fn(async () => {}),
  getSystemTheme: vi.fn(async () => 'dark'),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('登录/锁定页重新挂载时恢复强调色', () => {
  it.each(['light', 'dark', 'system'] as const)('%s 模式携带保存的自定义色', async (theme) => {
    settings.theme = theme;
    renderHook(() => useApplyThemeFromSettings());
    await waitFor(() =>
      expect(applyTheme).toHaveBeenCalledWith(
        expect.objectContaining({
          accentColor: 'custom',
          customAccentHex: '#777777',
          resolvedSystemTheme: theme === 'system' ? 'dark' : undefined,
        }),
      ),
    );
  });
});
