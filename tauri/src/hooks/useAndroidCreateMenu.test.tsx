import { act, renderHook, waitFor } from '@testing-library/react';
import { addPluginListener, type PluginListener } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAuthStore } from '@/stores/authStore';
import { useAndroidCreateMenu } from './useAndroidCreateMenu';

const mocks = vi.hoisted(() => ({
  mode: 'enhanced',
  invoke: vi.fn(),
  requestMenu: vi.fn(),
  onAction: vi.fn(),
}));

vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('./useAndroidGlass', () => ({
  useAndroidGlassMode: () => mocks.mode,
  useMediaPreference: () => false,
}));
vi.mock('@/lib/androidGlass', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/androidGlass')>()),
  requestAndroidGlassMenu: mocks.requestMenu,
}));
vi.mock('react-router-dom', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-router-dom')>()),
  useLocation: () => ({ key: 'home' }),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

const account = { id: 'account-a', name: 'Alice' };
const pluginListener = (unregister = vi.fn().mockResolvedValue(undefined)): PluginListener => ({
  plugin: 'android-glass',
  event: 'capabilities-changed',
  channelId: 1,
  unregister,
});

beforeEach(() => {
  vi.clearAllMocks();
  mocks.mode = 'enhanced';
  mocks.invoke.mockResolvedValue({ apiLevel: 35, windowBlur: true, webViewVersion: '1' });
  vi.mocked(addPluginListener).mockResolvedValue(pluginListener());
  useAuthStore.setState({ isAuthenticated: true, currentAccount: account });
});

describe('Android native create menu lifecycle', () => {
  it('uses the web menu when the glass mode is local', async () => {
    mocks.mode = 'local';
    const { result } = renderHook(() => useAndroidCreateMenu(mocks.onAction));

    await act(async () => result.current.open(null));

    expect(mocks.onAction).toHaveBeenCalledExactlyOnceWith('unavailable');
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(result.current.busy).toBe(false);
  });

  it('uses the web menu when native blur is unavailable', async () => {
    mocks.invoke.mockResolvedValue({ apiLevel: 30, windowBlur: false, webViewVersion: '1' });
    const { result } = renderHook(() => useAndroidCreateMenu(mocks.onAction));

    await act(async () => result.current.open(null));

    expect(mocks.onAction).toHaveBeenCalledExactlyOnceWith('unavailable');
    expect(addPluginListener).not.toHaveBeenCalled();
    expect(mocks.requestMenu).not.toHaveBeenCalled();
    expect(result.current.busy).toBe(false);
  });

  it('dispatches one native selection and releases the listener', async () => {
    const action = deferred<'page'>();
    const unregister = vi.fn().mockResolvedValue(undefined);
    vi.mocked(addPluginListener).mockResolvedValue(pluginListener(unregister));
    mocks.requestMenu.mockReturnValue({ result: action.promise, cancel: vi.fn() });
    const trigger = document.createElement('button');
    const focus = vi.spyOn(trigger, 'focus');
    const { result } = renderHook(() => useAndroidCreateMenu(mocks.onAction));
    let opening!: Promise<void>;

    act(() => {
      opening = result.current.open(trigger);
    });
    await waitFor(() => expect(mocks.requestMenu).toHaveBeenCalledTimes(1));
    expect(result.current.busy).toBe(true);
    await act(async () => result.current.open(trigger));
    expect(mocks.requestMenu).toHaveBeenCalledTimes(1);

    await act(async () => {
      action.resolve('page');
      await opening;
    });

    expect(mocks.onAction).toHaveBeenCalledExactlyOnceWith('page');
    expect(unregister).toHaveBeenCalledTimes(1);
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
    expect(result.current.busy).toBe(false);
  });

  it('unregisters a listener only once when account changes during registration', async () => {
    const registration = deferred<PluginListener>();
    const unregister = vi.fn().mockResolvedValue(undefined);
    vi.mocked(addPluginListener).mockReturnValue(registration.promise);
    const { result } = renderHook(() => useAndroidCreateMenu(mocks.onAction));
    let opening!: Promise<void>;

    act(() => {
      opening = result.current.open(null);
    });
    await waitFor(() => expect(addPluginListener).toHaveBeenCalledTimes(1));
    act(() => {
      useAuthStore.setState({ currentAccount: { id: 'account-b', name: 'Bob' } });
    });
    await act(async () => {
      registration.resolve(pluginListener(unregister));
      await opening;
    });

    expect(unregister).toHaveBeenCalledTimes(1);
    expect(mocks.requestMenu).not.toHaveBeenCalled();
    expect(mocks.onAction).not.toHaveBeenCalled();
    expect(result.current.busy).toBe(false);
  });

  it('closes the native menu and ignores a late action after account change', async () => {
    const action = deferred<'scan'>();
    const cancel = vi.fn().mockResolvedValue(undefined);
    const unregister = vi.fn().mockResolvedValue(undefined);
    vi.mocked(addPluginListener).mockResolvedValue(pluginListener(unregister));
    mocks.requestMenu.mockReturnValue({ result: action.promise, cancel });
    const { result } = renderHook(() => useAndroidCreateMenu(mocks.onAction));
    let opening!: Promise<void>;

    act(() => {
      opening = result.current.open(null);
    });
    await waitFor(() => expect(mocks.requestMenu).toHaveBeenCalledTimes(1));
    act(() => {
      useAuthStore.setState({ currentAccount: { id: 'account-b', name: 'Bob' } });
    });
    await act(async () => {
      action.resolve('scan');
      await opening;
    });

    expect(cancel).toHaveBeenCalledTimes(1);
    expect(unregister).toHaveBeenCalledTimes(1);
    expect(mocks.onAction).not.toHaveBeenCalled();
    expect(result.current.busy).toBe(false);
  });
});
