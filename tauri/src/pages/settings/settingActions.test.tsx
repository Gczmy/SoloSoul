import type { ReactNode } from 'react';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useApplyThemeFromSettings } from '@/hooks/useApplyThemeFromSettings';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { applyTheme, getSystemTheme } from '@/lib/theme';
import { getSchemeById, type ThemeScheme } from '@/lib/themeSchemes';
import { AndroidAppearance } from '@/components/android/AndroidAppearance';
import { AppearanceSettingsPage } from './AppearanceSettingsPage';
import { SecuritySettingsPage } from './SecuritySettingsPage';

const { showToast } = vi.hoisted(() => ({ showToast: vi.fn() }));
vi.mock('@/stores/uiStore', () => ({
  useUiStore: (select: (state: { showToast: typeof showToast }) => unknown) =>
    select({ showToast }),
}));
vi.mock('@/lib/i18n', () => ({
  default: { changeLanguage: vi.fn(async () => {}) },
  detectSystemLanguage: () => 'en-US',
}));
vi.mock('@/lib/theme', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/theme')>()),
  applyTheme: vi.fn(async () => {}),
  getSystemTheme: vi.fn(async () => 'light' as const),
  listenForSystemTheme: vi.fn(async () => () => {}),
}));
vi.mock('@/lib/platform', () => ({
  isMobilePlatformSync: () => false,
  isAndroidSync: () => false,
}));
vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));
vi.mock('@/components/settings/BiometricSection', () => ({ BiometricSection: () => null }));
vi.mock('@/components/settings/PinSection', () => ({ PinSection: () => null }));
vi.mock('@/components/settings/PasswordChangeForm', () => ({ PasswordChangeForm: () => null }));
vi.mock('@/components/settings/ThemeSchemePanel', () => ({
  ThemeSchemePanel: ({ onSelectScheme }: { onSelectScheme: (scheme: ThemeScheme) => void }) => (
    <button onClick={() => onSelectScheme(getSchemeById('deep-ocean')!)}>select deep-ocean</button>
  ),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

const defaults = useSettingsStore.getInitialState().settings;
const mockInvoke = vi.mocked(invoke);
function ThemeOwner() {
  useApplyThemeFromSettings();
  return null;
}
function renderPage(page: ReactNode) {
  const view = render(
    <>
      <ThemeOwner />
      <MemoryRouter>{page}</MemoryRouter>
    </>,
  );
  // 初始确认主题已应用；下面检验的是用户编辑触发的交付。
  vi.mocked(applyTheme).mockClear();
  return view;
}
function writes() {
  return mockInvoke.mock.calls.filter(([cmd]) => cmd === 'user_data_update_preference');
}

beforeEach(() => {
  vi.clearAllMocks();
  useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
  useAuthStore.setState({
    isAuthenticated: true,
    currentAccount: { id: 'account', name: 'Account' },
  });
  useSettingsStore.setState({ settings: { ...defaults, theme: 'light' }, isLoading: false });
  localStorage.clear();
  mockInvoke.mockResolvedValue(undefined);
  vi.mocked(getSystemTheme).mockResolvedValue('light');
});
afterEach(() => {
  cleanup();
  useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
});

describe('RF111 真实 Store 设置交互', () => {
  it('Android 自定义颜色保存失败时不启用该颜色', async () => {
    const write = deferred<void>();
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'user_data_update_preference') return write.promise;
    });
    renderPage(<AndroidAppearance />);
    const input = screen.getByRole('textbox', { name: 'settings:accent_custom' });
    fireEvent.change(input, { target: { value: '#112233' } });
    fireEvent.submit(input.closest('form')!);
    await waitFor(() => expect(writes()).toHaveLength(1));
    await act(async () => write.reject(new Error('disk full')));
    expect(writes()).toHaveLength(1);
    expect(useSettingsStore.getState().settings.accentColor).toBe('ocean');
    expect(showToast).toHaveBeenCalledTimes(1);
  });

  it('Android 自定义颜色保存期间选择预设，迟到结果不覆盖预设', async () => {
    const write = deferred<void>();
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd !== 'user_data_update_preference') return;
      const { preferences } = (args as { payload: { preferences: Record<string, unknown> } })
        .payload;
      if ('customAccentHex' in preferences) return write.promise;
    });
    renderPage(<AndroidAppearance />);
    const input = screen.getByRole('textbox', { name: 'settings:accent_custom' });
    fireEvent.change(input, { target: { value: '#112233' } });
    fireEvent.submit(input.closest('form')!);
    await waitFor(() => expect(writes()).toHaveLength(1));
    fireEvent.click(screen.getByRole('button', { name: 'settings:accent_forest' }));
    await waitFor(() => expect(writes()).toHaveLength(2));
    await act(async () => write.resolve());
    expect(writes()).toHaveLength(2);
    expect(useSettingsStore.getState().settings.accentColor).toBe('forest');
  });

  it('Android 自定义颜色保存期间切换账户，不交付到新账户', async () => {
    const write = deferred<void>();
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'user_data_update_preference') return write.promise;
    });
    renderPage(<AndroidAppearance />);
    const input = screen.getByRole('textbox', { name: 'settings:accent_custom' });
    fireEvent.change(input, { target: { value: '#112233' } });
    fireEvent.submit(input.closest('form')!);
    await waitFor(() => expect(writes()).toHaveLength(1));
    act(() => {
      useAuthStore.setState({ currentAccount: { id: 'other', name: 'Other' } });
      useSettingsStore.setState({ settings: { ...defaults, accentColor: 'rose' } });
    });
    await act(async () => write.resolve());
    expect(writes()).toHaveLength(1);
    expect(useSettingsStore.getState().settings.accentColor).toBe('rose');
    expect(showToast).not.toHaveBeenCalled();
  });

  it('外观保存失败提示一次、回滚单选值，不应用主题或写成功缓存', async () => {
    const write = deferred<void>();
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'user_data_update_preference') return write.promise;
    });
    localStorage.setItem('solosoul_ui_prefs', 'original-cache');
    renderPage(<AppearanceSettingsPage />);
    const dark = screen.getByRole('radio', { name: /common:theme.dark/ });
    fireEvent.click(dark);
    expect(dark).toBeChecked();
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect(applyTheme).not.toHaveBeenCalled();
    await act(async () => write.reject(new Error('disk full')));
    await waitFor(() => expect(showToast).toHaveBeenCalledTimes(1));
    expect(showToast).toHaveBeenCalledWith({ type: 'error', message: 'common:save_failed' });
    expect(screen.getByRole('radio', { name: /common:theme.light/ })).toBeChecked();
    expect(applyTheme).not.toHaveBeenCalled();
    expect(mockInvoke.mock.calls.some(([cmd]) => cmd === 'ui_update_preference')).toBe(false);
    expect(localStorage.getItem('solosoul_ui_prefs')).toBe('original-cache');
  });

  it('主题保存成功时不带入仍挂起且随后失败的强调色', async () => {
    const themeWrite = deferred<void>();
    const accentWrite = deferred<void>();
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd !== 'user_data_update_preference') return;
      const { preferences } = (args as { payload: { preferences: Record<string, unknown> } })
        .payload;
      return 'theme' in preferences ? themeWrite.promise : accentWrite.promise;
    });
    renderPage(<AppearanceSettingsPage />);
    fireEvent.click(screen.getByRole('radio', { name: /common:theme.dark/ }));
    fireEvent.click(screen.getByTitle('settings:accent_forest'));
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(useSettingsStore.getState().settings.accentColor).toBe('forest');
    expect(applyTheme).not.toHaveBeenCalled();

    await act(async () => themeWrite.resolve());
    await waitFor(() => expect(applyTheme).toHaveBeenCalledTimes(1));
    await act(async () => accentWrite.reject(new Error('disk full')));
    await waitFor(() => expect(showToast).toHaveBeenCalledTimes(1));

    expect(applyTheme).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({ preset: 'warm-stone-dark', accentColor: 'ocean' }),
      expect.any(Function),
    );
    expect(useSettingsStore.getState().settings).toMatchObject({
      theme: 'dark',
      accentColor: 'ocean',
    });
    expect(showToast).toHaveBeenCalledWith({ type: 'error', message: 'common:save_failed' });
  });

  it('色板首键失败不继续改模式，也不提前应用未保存色板', async () => {
    mockInvoke.mockRejectedValue(new Error('disk full'));
    renderPage(<AppearanceSettingsPage />);
    fireEvent.click(screen.getByRole('button', { name: 'select deep-ocean' }));
    await waitFor(() => expect(showToast).toHaveBeenCalledTimes(1));
    expect(writes()).toHaveLength(1);
    expect(useSettingsStore.getState().settings).toMatchObject({
      theme: 'light',
      defaultDarkTheme: defaults.defaultDarkTheme,
    });
    expect(applyTheme).not.toHaveBeenCalled();
  });

  it('色板保存成功而模式失败时保留部分成功，只应用回滚后的实际模式', async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      const preferences = (args as { payload?: { preferences?: { theme?: string } } } | undefined)
        ?.payload?.preferences;
      if (cmd === 'user_data_update_preference' && preferences?.theme) throw new Error('disk full');
    });
    renderPage(<AppearanceSettingsPage />);
    fireEvent.click(screen.getByRole('button', { name: 'select deep-ocean' }));
    await waitFor(() => expect(showToast).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(applyTheme).toHaveBeenCalledTimes(1));
    expect(writes()).toHaveLength(2);
    expect(useSettingsStore.getState().settings).toMatchObject({
      theme: 'light',
      defaultDarkTheme: 'deep-ocean',
    });
    expect(applyTheme).toHaveBeenCalledWith(
      expect.objectContaining({ preset: 'warm-stone-light', defaultDarkTheme: 'deep-ocean' }),
      expect.any(Function),
    );
    expect(screen.getByRole('radio', { name: /common:theme.light/ })).toBeChecked();
  });

  it('保存后等待系统模式时切换账户，迟到结果不能应用旧主题', async () => {
    const mode = deferred<'light' | 'dark'>();
    vi.mocked(getSystemTheme).mockReturnValue(mode.promise);
    renderPage(<AppearanceSettingsPage />);
    fireEvent.click(screen.getByRole('radio', { name: 'common:theme.system' }));
    await waitFor(() => expect(getSystemTheme).toHaveBeenCalledTimes(1));
    act(() => {
      useAuthStore.setState({
        isAuthenticated: true,
        currentAccount: { id: 'other', name: 'Other' },
      });
      useSettingsStore.setState({ settings: { ...defaults, theme: 'dark', accentColor: 'rose' } });
    });
    await act(async () => mode.resolve('light'));
    expect(vi.mocked(applyTheme).mock.calls.some(([config]) => config.preset === 'system')).toBe(
      false,
    );
    expect(applyTheme).toHaveBeenLastCalledWith(
      expect.objectContaining({ preset: 'warm-stone-dark', accentColor: 'rose' }),
      expect.any(Function),
    );
    expect(showToast).not.toHaveBeenCalled();
    expect(useSettingsStore.getState().settings).toMatchObject({
      theme: 'dark',
      accentColor: 'rose',
    });
  });

  it('等待系统模式时同键新保存使旧成功结果失效', async () => {
    const mode = deferred<'light' | 'dark'>();
    vi.mocked(getSystemTheme).mockReturnValue(mode.promise);
    renderPage(<AppearanceSettingsPage />);
    fireEvent.click(screen.getByRole('radio', { name: 'common:theme.system' }));
    await waitFor(() => expect(getSystemTheme).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByRole('radio', { name: /common:theme.dark/ }));
    await waitFor(() => expect(applyTheme).toHaveBeenCalledTimes(1));
    await act(async () => mode.resolve('light'));
    expect(applyTheme).toHaveBeenCalledTimes(1);
    expect(applyTheme).toHaveBeenCalledWith(
      expect.objectContaining({ preset: 'warm-stone-dark' }),
      expect.any(Function),
    );
    expect(showToast).not.toHaveBeenCalled();
  });

  it('系统解析后使用当前 Store 的其他键，不覆盖已保存的新强调色', async () => {
    const mode = deferred<'light' | 'dark'>();
    vi.mocked(getSystemTheme).mockReturnValue(mode.promise);
    renderPage(<AppearanceSettingsPage />);
    fireEvent.click(screen.getByRole('radio', { name: 'common:theme.system' }));
    await waitFor(() => expect(getSystemTheme).toHaveBeenCalledTimes(1));
    await act(async () => {
      await useSettingsStore.getState().updateSetting('account', 'accentColor', 'forest');
    });
    await act(async () => mode.resolve('dark'));
    expect(applyTheme).toHaveBeenCalledWith(
      expect.objectContaining({
        preset: 'system',
        accentColor: 'forest',
        resolvedSystemTheme: 'dark',
      }),
      expect.any(Function),
    );
    expect(showToast).not.toHaveBeenCalled();
  });

  it('安全设置失败提示一次并回滚值，不触发语言或主题成功副作用', async () => {
    const write = deferred<void>();
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'user_data_update_preference') return write.promise;
    });
    renderPage(<SecuritySettingsPage />);
    const timeout = screen.getByRole('combobox');
    fireEvent.change(timeout, { target: { value: '0' } });
    expect(timeout).toHaveValue('0');
    expect(screen.getByText('settings:auto_lock_never_warning')).toBeVisible();
    await waitFor(() => expect(writes()).toHaveLength(1));
    await act(async () => write.reject(new Error('disk full')));
    await waitFor(() => expect(showToast).toHaveBeenCalledTimes(1));
    expect(showToast).toHaveBeenCalledWith({ type: 'error', message: 'common:save_failed' });
    expect(timeout).toHaveValue('5');
    expect(screen.queryByText('settings:auto_lock_never_warning')).not.toBeInTheDocument();
    expect(writes()).toHaveLength(1);
    expect(applyTheme).not.toHaveBeenCalled();
    expect(mockInvoke.mock.calls.some(([cmd]) => cmd === 'ui_update_preference')).toBe(false);
  });

  it('旧账户失败不提示，也不撤销新账户的安全设置', async () => {
    const write = deferred<void>();
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'user_data_update_preference') return write.promise;
    });
    renderPage(<SecuritySettingsPage />);
    fireEvent.change(screen.getByRole('combobox'), { target: { value: '0' } });
    await waitFor(() => expect(writes()).toHaveLength(1));
    act(() => {
      useAuthStore.setState({
        isAuthenticated: true,
        currentAccount: { id: 'other', name: 'Other' },
      });
      useSettingsStore.setState({ settings: { ...defaults, autoLockTimeoutMinutes: 30 } });
    });
    await act(async () => write.reject(new Error('old account locked')));
    expect(screen.getByRole('combobox')).toHaveValue('30');
    expect(showToast).not.toHaveBeenCalled();
  });

  it('Android 仅应用确认主题，保存失败恢复选中状态且不交付失败主题', async () => {
    const write = deferred<void>();
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'user_data_update_preference') return write.promise;
    });
    renderPage(<AndroidAppearance />);
    fireEvent.click(screen.getByRole('button', { name: 'material.theme_dark' }));
    expect(screen.getByRole('button', { name: 'material.theme_dark' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect(applyTheme).not.toHaveBeenCalled();
    await act(async () => write.reject(new Error('disk full')));
    await waitFor(() => expect(showToast).toHaveBeenCalledTimes(1));
    expect(screen.getByRole('button', { name: 'material.theme_light' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(applyTheme).not.toHaveBeenCalled();
    expect(mockInvoke.mock.calls.some(([cmd]) => cmd === 'ui_update_preference')).toBe(false);
  });
});
