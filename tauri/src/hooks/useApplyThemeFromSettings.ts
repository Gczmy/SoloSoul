import { useEffect } from 'react';
import { applyTheme, getSystemTheme } from '@/lib/theme';
import { useSettingsStore } from '@/stores/settingsStore';

/**
 * 根据当前 settings store 中的主题配置应用主题。
 * 在组件 mount 时执行一次，并同步 Android 状态栏/桌面标题栏风格。
 * 用于 AppRoutes、LoginPage、BootstrapPage 等需要在无账户或锁定状态下
 * 也能正确显示主题的位置。
 */
export function useApplyThemeFromSettings() {
  useEffect(() => {
    const run = async () => {
      const s = useSettingsStore.getState().settings;
      const resolvedSystemTheme = s.theme === 'system' ? await getSystemTheme() : undefined;
      await applyTheme({
        preset:
          s.theme === 'dark'
            ? 'warm-stone-dark'
            : s.theme === 'light'
              ? 'warm-stone-light'
              : 'system',
        accentColor: s.accentColor,
        backgroundType: s.backgroundType,
        backgroundValue: s.backgroundValue,
        defaultLightTheme: s.defaultLightTheme,
        defaultDarkTheme: s.defaultDarkTheme,
        resolvedSystemTheme:
          typeof resolvedSystemTheme === 'string' ? resolvedSystemTheme : undefined,
      });
    };
    void run();
  }, []);
}
