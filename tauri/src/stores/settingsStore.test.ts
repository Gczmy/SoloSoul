import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useSettingsStore } from './settingsStore';

// Mocks must be declared before importing the module under test
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@/lib/theme', () => ({
  applyTheme: vi.fn(),
}));

vi.mock('@/lib/i18n', () => ({
  __esModule: true,
  default: { changeLanguage: vi.fn(() => Promise.resolve()) },
  detectSystemLanguage: vi.fn(() => 'en-US'),
}));

import { invoke } from '@tauri-apps/api/core';
import { applyTheme } from '@/lib/theme';
import i18next from '@/lib/i18n';

describe('settingsStore', () => {
  let localStorageData: Record<string, string> = {};

  beforeEach(() => {
    // Reset store to default state
    useSettingsStore.setState({
      settings: {
        theme: 'system',
        accentColor: 'ocean',
        customAccentHex: '',
        backgroundType: 'solid',
        backgroundValue: '',
        language: 'en-US',
        locale: 'en',
        autoLockTimeoutMinutes: 5,
        biometricEnabled: false,
        confirmDelete: true,
        customPages: [],
        defaultLightTheme: 'warm-stone',
        defaultDarkTheme: 'warm-stone-dark',
      },
      isLoading: false,
    });

    localStorageData = {};
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => localStorageData[key] ?? null,
      setItem: (key: string, value: string) => { localStorageData[key] = value; },
      removeItem: (key: string) => { delete localStorageData[key]; },
    });

    vi.clearAllMocks();
  });

  describe('loadUiPreferences', () => {
    it('should apply cached theme from localStorage instantly', async () => {
      localStorageData['solosoul_ui_prefs'] = JSON.stringify({
        theme: 'dark',
        accentColor: 'amber',
        defaultLightTheme: 'warm-stone',
        defaultDarkTheme: 'warm-stone-dark',
      });
      vi.mocked(invoke).mockRejectedValue(new Error('no file'));
      await useSettingsStore.getState().loadUiPreferences();
      expect(useSettingsStore.getState().settings.theme).toBe('dark');
      expect(useSettingsStore.getState().settings.accentColor).toBe('amber');
      expect(applyTheme).toHaveBeenCalled();
    });

    it('should fall back to IPC when localStorage is empty', async () => {
      vi.mocked(invoke).mockResolvedValue({
        theme: 'light',
        accentColor: 'forest',
        language: 'zh-CN',
        defaultLightTheme: 'warm-stone',
        defaultDarkTheme: 'warm-stone-dark',
      });
      await useSettingsStore.getState().loadUiPreferences();
      expect(useSettingsStore.getState().settings.theme).toBe('light');
      expect(useSettingsStore.getState().settings.accentColor).toBe('forest');
      expect(useSettingsStore.getState().settings.language).toBe('zh-CN');
      expect(localStorageData['solosoul_ui_prefs']).toContain('light');
    });

    it('should handle missing ui_preferences file gracefully', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('file not found'));
      await useSettingsStore.getState().loadUiPreferences();
      expect(useSettingsStore.getState().settings.theme).toBe('system');
    });
  });

  describe('loadSettings', () => {
    it('should load and validate all preference fields', async () => {
      vi.mocked(invoke).mockResolvedValue({
        theme: 'dark',
        accentColor: 'rose',
        customAccentHex: '#ff0000',
        backgroundType: 'gradient',
        backgroundValue: 'linear-gradient',
        language: 'zh-CN',
        locale: 'zh',
        autoLockTimeoutMinutes: 10,
        biometricEnabled: true,
        confirmDelete: false,
        customPages: [{ id: 'p1', name: 'Page1', iconId: 'document', createdAt: '2024-01-01', sortOrder: 0 }],
        defaultLightTheme: 'warm-stone',
        defaultDarkTheme: 'warm-stone-dark',
      });
      await useSettingsStore.getState().loadSettings('acc-1');
      const s = useSettingsStore.getState().settings;
      expect(s.theme).toBe('dark');
      expect(s.accentColor).toBe('rose');
      expect(s.customAccentHex).toBe('#ff0000');
      expect(s.backgroundType).toBe('gradient');
      expect(s.language).toBe('zh-CN');
      expect(s.locale).toBe('zh');
      expect(s.autoLockTimeoutMinutes).toBe(10);
      expect(s.biometricEnabled).toBe(true);
      expect(s.confirmDelete).toBe(false);
      expect(s.customPages).toHaveLength(1);
      expect(useSettingsStore.getState().isLoading).toBe(false);
    });

    it('should ignore invalid enum values and keep defaults', async () => {
      vi.mocked(invoke).mockResolvedValue({
        theme: 'invalid-theme',
        accentColor: 'invalid-color',
        backgroundType: 'invalid-bg',
        language: 'invalid-lang',
        autoLockTimeoutMinutes: 'not-a-number',
        biometricEnabled: 'not-a-bool',
      });
      await useSettingsStore.getState().loadSettings('acc-1');
      const s = useSettingsStore.getState().settings;
      expect(s.theme).toBe('system');
      expect(s.accentColor).toBe('ocean');
      expect(s.backgroundType).toBe('solid');
      expect(s.language).toBe('en-US');
      expect(s.autoLockTimeoutMinutes).toBe(5);
      expect(s.biometricEnabled).toBe(false);
    });

    it('should sync UI prefs after loading settings', async () => {
      vi.mocked(invoke).mockResolvedValue({ theme: 'light', accentColor: 'ocean' });
      await useSettingsStore.getState().loadSettings('acc-1');
      expect(invoke).toHaveBeenCalledWith('ui_update_preference', expect.any(Object));
    });
  });

  describe('updateSetting', () => {
    it('should update setting optimistically and persist', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);
      await useSettingsStore.getState().updateSetting('acc-1', 'theme', 'dark');
      expect(useSettingsStore.getState().settings.theme).toBe('dark');
      expect(invoke).toHaveBeenCalledWith('user_data_update_preference', {
        payload: { accountId: 'acc-1', preferences: { theme: 'dark' } },
      });
    });

    it('should rollback on persistence failure', async () => {
      useSettingsStore.setState({ settings: { ...useSettingsStore.getState().settings, theme: 'light' } });
      vi.mocked(invoke).mockRejectedValue(new Error('disk full'));
      await useSettingsStore.getState().updateSetting('acc-1', 'theme', 'dark');
      expect(useSettingsStore.getState().settings.theme).toBe('light');
    });

    it('should change i18n language when updating language', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);
      await useSettingsStore.getState().updateSetting('acc-1', 'language', 'zh-CN');
      expect(i18next.changeLanguage).toHaveBeenCalledWith('zh-CN');
    });
  });

  describe('addCustomPage', () => {
    it('should add page optimistically and persist to objects table', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);
      const page = await useSettingsStore.getState().addCustomPage('acc-1', 'My Page', 'star');
      expect(page.name).toBe('My Page');
      expect(page.iconId).toBe('star');
      expect(useSettingsStore.getState().settings.customPages).toHaveLength(1);
      expect(invoke).toHaveBeenCalledWith('object_create', expect.objectContaining({
        input: expect.objectContaining({
          accountId: 'acc-1',
          name: 'My Page',
          collectionType: 'page',
          iconName: 'star',
        }),
      }));
    });

    it('should rollback on creation failure', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('db locked'));
      await useSettingsStore.getState().addCustomPage('acc-1', 'Fail Page');
      expect(useSettingsStore.getState().settings.customPages).toHaveLength(0);
    });
  });

  describe('removeCustomPage', () => {
    it('should remove page optimistically and call page_delete', async () => {
      useSettingsStore.setState({
        settings: {
          ...useSettingsStore.getState().settings,
          customPages: [
            { id: 'p1', name: 'Page1', iconId: 'document', createdAt: '2024-01-01', sortOrder: 0 },
            { id: 'p2', name: 'Page2', iconId: 'star', createdAt: '2024-01-02', sortOrder: 1 },
          ],
        },
      });
      vi.mocked(invoke).mockResolvedValue(undefined);
      await useSettingsStore.getState().removeCustomPage('acc-1', 'p1');
      expect(useSettingsStore.getState().settings.customPages).toHaveLength(1);
      expect(useSettingsStore.getState().settings.customPages[0].id).toBe('p2');
      expect(invoke).toHaveBeenCalledWith('page_delete', { accountId: 'acc-1', sectionType: 'custom', pageObjectId: 'p1' });
    });

    it('should rollback on deletion failure', async () => {
      useSettingsStore.setState({
        settings: {
          ...useSettingsStore.getState().settings,
          customPages: [{ id: 'p1', name: 'Page1', iconId: 'document', createdAt: '2024-01-01', sortOrder: 0 }],
        },
      });
      vi.mocked(invoke).mockRejectedValue(new Error('not found'));
      await useSettingsStore.getState().removeCustomPage('acc-1', 'p1');
      expect(useSettingsStore.getState().settings.customPages).toHaveLength(1);
    });
  });

  describe('clearOnVaultLock', () => {
    it('should reset settings to defaults', () => {
      useSettingsStore.setState({
        settings: {
          ...useSettingsStore.getState().settings,
          theme: 'dark',
          customPages: [{ id: 'p1', name: 'Page1', iconId: 'document', createdAt: '2024-01-01', sortOrder: 0 }],
        },
        isLoading: true,
      });
      useSettingsStore.getState().clearOnVaultLock();
      expect(useSettingsStore.getState().settings.theme).toBe('system');
      expect(useSettingsStore.getState().settings.customPages).toHaveLength(0);
      expect(useSettingsStore.getState().isLoading).toBe(false);
    });
  });
});
