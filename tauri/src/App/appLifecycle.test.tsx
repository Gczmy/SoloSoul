import { StrictMode } from 'react';
import { act, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, useLocation, useNavigate } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useSafSyncStore } from '@/stores/safSyncStore';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useProfileStore } from '@/stores/profileStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUiStore } from '@/stores/uiStore';
import { useLlmStore } from '@/stores/llmStore';
import { applyTheme, listenForSystemTheme } from '@/lib/theme';
import { useNativeAppEvents } from './useNativeAppEvents';
import { useSessionLifecycle } from './useSessionLifecycle';

vi.mock('@/lib/nativeWindow', () => ({
  observeNativeWindowLayout: () => () => {},
  refreshNativeAppearance: vi.fn(),
}));
vi.mock('@/lib/theme', () => ({
  listenForSystemTheme: vi.fn(async () => () => {}),
  applyTheme: vi.fn(),
  getSystemTheme: vi.fn(),
}));
vi.mock('@/lib/vaultDirectory', () => ({ checkVaultDirectory: vi.fn().mockResolvedValue(true) }));
vi.mock('@/lib/notification', () => ({ initLlmNotificationListener: vi.fn(async () => () => {}) }));
vi.mock('@/lib/prefetch/warmup', () => ({
  warmupPrefetchRegistry: vi.fn(),
  resetPrefetchRegistry: vi.fn(),
}));
vi.mock('@/lib/startupScreen', () => ({ dismissStartupScreen: vi.fn(() => () => {}) }));
vi.mock('@/hooks/useAutoLock', () => ({ useAutoLock: () => {} }));
vi.mock('@/hooks/useApplyThemeFromSettings', () => ({ useApplyThemeFromSettings: () => {} }));

function NativeHarness() {
  const navigate = useNavigate();
  useNativeAppEvents({ navigate, isAuthenticated: true });
  return null;
}

function ThemeHarness() {
  const navigate = useNavigate();
  useNativeAppEvents({ navigate, isAuthenticated: false });
  return null;
}

function SessionHarness() {
  const navigate = useNavigate();
  const location = useLocation();
  const auth = useAuthStore();
  useSessionLifecycle({
    navigate,
    checkHasAccount: auth.checkHasAccount,
    hasAccount: auth.hasAccount,
    backendError: auth.backendError,
    isAuthenticated: auth.isAuthenticated,
    accountId: auth.currentAccount?.id,
  });
  return <div data-testid="location">{location.pathname}</div>;
}

afterEach(() => {
  useSafSyncStore.getState().stopListening();
  for (const toast of useUiStore.getState().toasts) {
    if (toast.timeoutId) clearTimeout(toast.timeoutId);
  }
  useUiStore.setState({ toasts: [], safAuthRevoked: false, safAuthToastShown: false });
  vi.mocked(listen).mockReset();
  vi.mocked(invoke).mockReset();
  vi.mocked(listenForSystemTheme)
    .mockReset()
    .mockResolvedValue(() => {});
  vi.restoreAllMocks();
});

describe('RF201 系统事件不会覆盖显式主题', () => {
  it.each(['light', 'dark', 'system'] as const)('%s 设置下处理切色与卸载', async (preset) => {
    const previous = useSettingsStore.getState().settings;
    const callbacks: Array<Parameters<typeof listenForSystemTheme>[0]> = [];
    vi.mocked(listen).mockResolvedValue(() => {});
    vi.mocked(applyTheme).mockClear();
    vi.mocked(listenForSystemTheme).mockImplementation(async (callback) => {
      callbacks.push(callback);
      return () => {};
    });
    useSettingsStore.setState({ settings: { ...previous, theme: preset } });
    const view = render(
      <MemoryRouter>
        <ThemeHarness />
      </MemoryRouter>,
    );
    try {
      await waitFor(() => expect(callbacks).toHaveLength(1));
      act(() => {
        callbacks[0]('dark');
        callbacks[0]('light');
      });
      if (preset === 'system') {
        expect(
          vi.mocked(applyTheme).mock.calls.map(([config]) => config.resolvedSystemTheme),
        ).toEqual(['dark', 'light']);
      } else {
        expect(applyTheme).not.toHaveBeenCalled();
      }
      view.unmount();
      act(() => callbacks[0]('dark'));
      expect(applyTheme).toHaveBeenCalledTimes(preset === 'system' ? 2 : 0);
    } finally {
      view.unmount();
      useSettingsStore.setState({ settings: previous });
    }
  });
});

describe('RF-114 应用生命周期', () => {
  it('LLM 通知监听在异步注册完成前卸载时释放旧句柄，不影响新监听', async () => {
    const { initLlmNotificationListener } =
      await vi.importActual<typeof import('@/lib/notification')>('@/lib/notification');
    const { trackAsyncListener } = await import('@/lib/asyncListener');
    const registrations: {
      unlisten: () => void;
      onChange: () => void;
    }[] = [];
    const subscribe = useLlmStore.subscribe;
    vi.spyOn(useLlmStore, 'subscribe').mockImplementation((callback) => {
      const onChange = vi.fn(() => {});
      const release = subscribe((...args) => {
        onChange();
        callback(...args);
      });
      const unlisten = vi.fn(release);
      registrations.push({ unlisten, onChange });
      return unlisten;
    });

    // 实际通知入口同步建立 Store 订阅，异步交付退订句柄。
    const disposeOld = trackAsyncListener(initLlmNotificationListener());
    const disposeCurrent = trackAsyncListener(initLlmNotificationListener());
    try {
      expect(registrations).toHaveLength(2);
      disposeOld();
      expect(registrations[0].unlisten).not.toHaveBeenCalled();
      await act(async () => {});
      expect(registrations[0].unlisten).toHaveBeenCalledTimes(1);
      expect(registrations[1].unlisten).not.toHaveBeenCalled();

      act(() => useLlmStore.setState((state) => ({ streams: { ...state.streams } })));
      expect(registrations[0].onChange).not.toHaveBeenCalled();
      expect(registrations[1].onChange).toHaveBeenCalledTimes(1);

      disposeCurrent();
      expect(registrations[1].unlisten).toHaveBeenCalledTimes(1);
      act(() => useLlmStore.setState((state) => ({ streams: { ...state.streams } })));
      expect(registrations[1].onChange).toHaveBeenCalledTimes(1);
      disposeOld();
      disposeCurrent();
      expect(registrations[0].unlisten).toHaveBeenCalledTimes(1);
      expect(registrations[1].unlisten).toHaveBeenCalledTimes(1);
    } finally {
      disposeOld();
      disposeCurrent();
    }
  });

  it('StrictMode 反序完成的 SAF 注册只保留新监听，卸载后全部释放', async () => {
    const registrations: {
      name: string;
      unlisten: () => void;
      resolve: (unlisten: () => void) => void;
    }[] = [];
    vi.mocked(listen).mockImplementation(
      (name) =>
        new Promise((resolve) => {
          registrations.push({ name: String(name), unlisten: vi.fn(() => {}), resolve });
        }),
    );

    const view = render(
      <StrictMode>
        <MemoryRouter>
          <NativeHarness />
        </MemoryRouter>
      </StrictMode>,
    );
    const sync = registrations.filter((registration) => registration.name === 'sync-progress');
    expect(sync).toHaveLength(2);

    await act(async () => sync[1].resolve(sync[1].unlisten));
    await act(async () => sync[0].resolve(sync[0].unlisten));
    expect(sync[0].unlisten).toHaveBeenCalledTimes(1);
    expect(sync[1].unlisten).not.toHaveBeenCalled();

    view.unmount();
    expect(sync[0].unlisten).toHaveBeenCalledTimes(1);
    expect(sync[1].unlisten).toHaveBeenCalledTimes(1);

    await act(async () => {
      for (const registration of registrations.filter((item) => item.name !== 'sync-progress')) {
        registration.resolve(registration.unlisten);
      }
    });
    for (const registration of registrations.filter((item) => item.name !== 'sync-progress')) {
      expect(registration.unlisten).toHaveBeenCalledTimes(1);
    }
  });

  it('SAF 授权撤销事件一会话只提示一次，卸载后旧回调不再写入状态', async () => {
    const registrations: {
      name: string;
      emit: () => void;
      unlisten: () => void;
    }[] = [];
    vi.mocked(listen).mockImplementation(async (name, callback) => {
      const unlisten = vi.fn(() => {});
      registrations.push({
        name: String(name),
        emit: () => callback({ event: String(name), id: 1, payload: null as never }),
        unlisten,
      });
      return unlisten;
    });
    const view = render(
      <MemoryRouter>
        <NativeHarness />
      </MemoryRouter>,
    );
    const revoked = registrations.find((registration) => registration.name === 'saf-auth-revoked');
    expect(revoked).toBeDefined();
    await act(async () => {});

    act(() => {
      revoked!.emit();
      revoked!.emit();
    });
    expect(useUiStore.getState().safAuthRevoked).toBe(true);
    expect(useUiStore.getState().toasts).toHaveLength(1);

    view.unmount();
    expect(revoked!.unlisten).toHaveBeenCalledTimes(1);
    useUiStore.setState({ safAuthToastShown: false });
    revoked!.emit();
    expect(useUiStore.getState().toasts).toHaveLength(1);
  });

  it('vault-locked 重复事件只清理一次会话并导航一次', async () => {
    const registrations: {
      name: string;
      emit: () => void;
      unlisten: () => void;
    }[] = [];
    vi.mocked(listen).mockImplementation(async (name, callback) => {
      const unlisten = vi.fn(() => {});
      registrations.push({
        name: String(name),
        emit: () => callback({ event: String(name), id: 1, payload: null as never }),
        unlisten,
      });
      return unlisten;
    });
    vi.mocked(invoke).mockImplementation(async (command) =>
      command === 'check_has_account' ? true : undefined,
    );
    vi.spyOn(useProfileStore.getState(), 'loadProfile').mockResolvedValue();
    vi.spyOn(useSettingsStore.getState(), 'loadSettings').mockResolvedValue();
    vi.spyOn(useSettingsStore.getState(), 'loadCustomPages').mockResolvedValue();

    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
    useAuthStore.getState().completeUnlock({ id: 'acc-a', name: 'A' });
    const clearObjects = vi.spyOn(useObjectStore.getState(), 'clearOnVaultLock');
    const view = render(
      <MemoryRouter initialEntries={['/']}>
        <SessionHarness />
      </MemoryRouter>,
    );
    const locked = registrations.find((registration) => registration.name === 'vault-locked');
    expect(locked).toBeDefined();

    act(() => {
      locked!.emit();
      locked!.emit();
    });
    await waitFor(() => expect(screen.getByTestId('location')).toHaveTextContent('/login'));
    expect(clearObjects).toHaveBeenCalledTimes(1);
    expect(vi.mocked(invoke).mock.calls.filter(([command]) => command === 'logout')).toHaveLength(
      1,
    );

    view.unmount();
    expect(locked!.unlisten).toHaveBeenCalledTimes(1);
  });

  it('认证后先读取设置和 Profile，再应用主题与加载自定义页面', async () => {
    vi.mocked(listen).mockResolvedValue(() => {});
    vi.mocked(invoke).mockResolvedValue(true);
    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
    useAuthStore.getState().completeUnlock({ id: 'acc-b', name: 'B' });
    useSettingsStore.setState((state) => ({
      settings: { ...state.settings, theme: 'dark' },
    }));

    const order: string[] = [];
    let finishSettings!: () => void;
    vi.spyOn(useProfileStore.getState(), 'loadProfile').mockImplementation(async () => {
      order.push('profile');
    });
    vi.spyOn(useSettingsStore.getState(), 'loadSettings').mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          order.push('settings');
          finishSettings = resolve;
        }),
    );
    vi.mocked(applyTheme).mockImplementation(async () => {
      order.push('theme');
    });
    vi.spyOn(useSettingsStore.getState(), 'loadCustomPages').mockImplementation(async () => {
      order.push('custom-pages');
    });

    const view = render(
      <MemoryRouter>
        <SessionHarness />
      </MemoryRouter>,
    );
    expect(order).toEqual(['profile', 'settings']);

    await act(async () => finishSettings());
    await waitFor(() => expect(order).toEqual(['profile', 'settings', 'theme', 'custom-pages']));
    view.unmount();
  });
});
