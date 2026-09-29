import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore, type CustomPage } from '@/stores/settingsStore';
import { CustomPageEditPopover } from './CustomPageEditPopover';

vi.mock('./IconCategoryPicker', () => ({ IconCategoryPicker: () => null }));
const page: CustomPage = {
  id: 'page-a',
  name: 'Before',
  iconId: 'star',
  createdAt: '2026-01-01',
  sortOrder: 0,
};
function pending() {
  let resolve!: (value?: unknown) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<unknown>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
beforeEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useAuthStore.getState().completeUnlock({ id: 'acc-a', name: 'A' });
  useSettingsStore.setState((s) => ({ settings: { ...s.settings, customPages: [page] } }));
});
function edit(onClose: () => void) {
  render(<CustomPageEditPopover page={page} isOpen onClose={onClose} triggerRect={null} />);
  fireEvent.change(screen.getByDisplayValue('Before'), { target: { value: 'After' } });
  fireEvent.keyDown(screen.getByDisplayValue('After'), { key: 'Enter' });
}

describe('RF111 自定义页面只确认权威对象写入', () => {
  it('对象成功后更新投影并关闭，不再启动可能失败的第二次偏好写入', async () => {
    const close = vi.fn();
    edit(close);
    await waitFor(() => expect(close).toHaveBeenCalledTimes(1));
    expect(useSettingsStore.getState().settings.customPages[0].name).toBe('After');
    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === 'object_update'),
    ).toHaveLength(1);
    expect(
      vi.mocked(invoke).mock.calls.some(([command]) => command === 'user_data_update_preference'),
    ).toBe(false);
  });
  it('对象失败保留旧投影与编辑界面', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('Synthetic object write failure'));
    const close = vi.fn();
    edit(close);
    await waitFor(() =>
      expect(screen.getByDisplayValue('After')).toHaveAttribute('data-error', 'true'),
    );
    expect(close).not.toHaveBeenCalled();
    expect(useSettingsStore.getState().settings.customPages[0].name).toBe('Before');
  });
  it('对象写入期间切账户，迟到成功不修改新账户投影或关闭旧对话框', async () => {
    const write = pending();
    vi.mocked(invoke).mockImplementation((command) =>
      command === 'object_update' ? write.promise : Promise.resolve(undefined),
    );
    const close = vi.fn();
    edit(close);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('object_update', expect.anything()));
    act(() => {
      useAuthStore.getState().completeUnlock({ id: 'acc-b', name: 'B' });
      useSettingsStore.setState((s) => ({
        settings: { ...s.settings, customPages: [{ ...page, id: 'page-b', name: 'Account B' }] },
      }));
    });
    await act(async () => {
      write.resolve();
      await write.promise;
    });
    expect(useSettingsStore.getState().settings.customPages.map((p) => p.name)).toEqual([
      'Account B',
    ]);
    expect(close).not.toHaveBeenCalled();
  });
});

describe('RF-1014 自定义页面编辑提交互斥', () => {
  it('保存未完成时重复按 Enter 只写入一次对象', async () => {
    const write = pending();
    vi.mocked(invoke).mockImplementation((command) =>
      command === 'object_update' ? write.promise : Promise.resolve(undefined),
    );
    const close = vi.fn();
    edit(close);
    fireEvent.keyDown(screen.getByDisplayValue('After'), { key: 'Enter' });
    await waitFor(() =>
      expect(
        vi.mocked(invoke).mock.calls.filter(([command]) => command === 'object_update'),
      ).toHaveLength(1),
    );
    await act(async () => {
      write.resolve();
      await write.promise;
    });
    expect(close).toHaveBeenCalledOnce();
    expect(useSettingsStore.getState().settings.customPages[0].name).toBe('After');
  });

  it('保存失败后允许用户重试', async () => {
    vi.mocked(invoke)
      .mockRejectedValueOnce(new Error('Synthetic write failure'))
      .mockResolvedValue(undefined);
    const close = vi.fn();
    edit(close);
    await waitFor(() =>
      expect(screen.getByDisplayValue('After')).toHaveAttribute('data-error', 'true'),
    );
    fireEvent.keyDown(screen.getByDisplayValue('After'), { key: 'Enter' });
    await waitFor(() => expect(close).toHaveBeenCalledOnce());
    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === 'object_update'),
    ).toHaveLength(2);
  });
});
