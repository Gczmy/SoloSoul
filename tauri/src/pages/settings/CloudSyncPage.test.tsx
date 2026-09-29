/**
 * P007 拆分回归冒烟测试：设置-云同步页面必须可完整渲染。
 * 背景：核验发现拆分后真机渲染抛「Cannot set indexed properties on this object」，
 * 本测试在 jsdom 中复现渲染路径，防止再次回归。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { useAuthStore } from '@/stores/authStore';
import { CloudSyncPage } from './CloudSyncPage';

// PageShell/PageContainer 简化为透传
vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));
vi.mock('@/components/layout/PageContainer', () => ({
  PageContainer: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@/components/forms/PasswordVerificationDialog', () => ({
  PasswordVerificationDialog: ({
    open,
    onVerify,
  }: {
    open: boolean;
    onVerify: (password: string) => Promise<boolean>;
  }) =>
    open ? (
      <button type="button" onClick={() => void onVerify('entered-master-password')}>
        提交主密码
      </button>
    ) : null,
}));

const listen = vi.fn().mockResolvedValue(() => {});
vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => listen(...(args as [])),
}));

import { invoke } from '@tauri-apps/api/core';

describe('CloudSyncPage 渲染冒烟', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      isAuthenticated: true,
      currentAccount: { id: 'account-a', name: 'Alice' },
    });
    listen.mockResolvedValue(() => {});
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'cloud_sync_get_config') {
        return Promise.resolve({
          connectorType: 'webdav',
          configJson: {
            baseUrl: 'https://dav.example.com/',
            username: 'u',
            password: 'p',
            rootPrefix: '/SoloSoul/',
          },
          enabled: true,
          intervalSecs: 3600,
          wifiOnly: true,
          autoImport: false,
          retention: { recentFull: 10, daily: true, weekly: true, monthly: true },
          lastSyncAt: '2026-08-24T00:00:00Z',
        });
      }
      if (cmd === 'cloud_sync_list_incoming') return Promise.resolve([]);
      return Promise.resolve(null);
    });
  });

  afterEach(() => {
    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
  });

  it('完整渲染不抛错，且各 section 均出现', async () => {
    render(
      <MemoryRouter>
        <CloudSyncPage />
      </MemoryRouter>,
    );

    // 等待异步配置真正回填；WebDAV 选项在加载前已存在，不能作为完成标志。
    await waitFor(() => {
      expect(screen.getByDisplayValue('https://dav.example.com/')).toBeInTheDocument();
      expect(screen.getByDisplayValue('u')).toBeInTheDocument();
    });
    expect(screen.getByRole('option', { name: /WebDAV \(坚果云/ })).toBeInTheDocument();
  });

  it('保存命令拒绝错误主密码时保持验证对话框', async () => {
    const originalInvoke = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation((cmd: string, args) => {
      if (cmd === 'cloud_sync_save_config') return Promise.resolve(false);
      return originalInvoke(cmd, args);
    });
    render(
      <MemoryRouter>
        <CloudSyncPage />
      </MemoryRouter>,
    );

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'settings:cloud_sync_update' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'settings:cloud_sync_update' }));
    fireEvent.click(screen.getByRole('button', { name: '提交主密码' }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('cloud_sync_save_config', {
        payload: expect.objectContaining({ accountId: 'account-a' }),
        password: 'entered-master-password',
      }),
    );
    expect(screen.getByRole('button', { name: '提交主密码' })).toBeInTheDocument();
  });

  it('保存命令接受正确主密码后关闭验证对话框', async () => {
    const originalInvoke = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation((cmd: string, args) => {
      if (cmd === 'cloud_sync_save_config') return Promise.resolve(true);
      return originalInvoke(cmd, args);
    });
    render(
      <MemoryRouter>
        <CloudSyncPage />
      </MemoryRouter>,
    );

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'settings:cloud_sync_update' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'settings:cloud_sync_update' }));
    fireEvent.click(screen.getByRole('button', { name: '提交主密码' }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('cloud_sync_save_config', {
        payload: expect.objectContaining({
          accountId: 'account-a',
          connectorType: 'webdav',
          configJson: expect.objectContaining({ baseUrl: 'https://dav.example.com/' }),
        }),
        password: 'entered-master-password',
      }),
    );
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: '提交主密码' })).not.toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'settings:cloud_sync_update' }));
    expect(screen.getByRole('button', { name: '提交主密码' })).toBeInTheDocument();
  });

  it('连接测试按 Rust 命令签名传入 payload', async () => {
    render(
      <MemoryRouter>
        <CloudSyncPage />
      </MemoryRouter>,
    );

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'settings:cloud_sync_test' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'settings:cloud_sync_test' }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('cloud_sync_test_connection', {
        payload: expect.objectContaining({
          accountId: 'account-a',
          connectorType: 'webdav',
          configJson: expect.objectContaining({ username: 'u' }),
        }),
      }),
    );
  });

  it('旧账户连接测试完成后不覆盖新账户的结果', async () => {
    let resolveConnection!: () => void;
    const originalInvoke = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation((cmd: string, args) => {
      if (cmd === 'cloud_sync_test_connection')
        return new Promise<void>((resolve) => {
          resolveConnection = resolve;
        });
      return originalInvoke(cmd, args);
    });
    render(
      <MemoryRouter>
        <CloudSyncPage />
      </MemoryRouter>,
    );

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'settings:cloud_sync_test' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'settings:cloud_sync_test' }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('cloud_sync_test_connection', expect.anything()),
    );
    act(() => useAuthStore.setState({ currentAccount: { id: 'account-b', name: 'Bob' } }));
    resolveConnection();
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(screen.queryByText('settings:cloud_sync_test_success')).not.toBeInTheDocument();
  });

  it('旧账户连接测试失败后不覆盖新账户的错误状态', async () => {
    let rejectConnection!: (error: Error) => void;
    const originalInvoke = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation((cmd: string, args) => {
      if (cmd === 'cloud_sync_test_connection')
        return new Promise<void>((_, reject) => {
          rejectConnection = reject;
        });
      return originalInvoke(cmd, args);
    });
    render(
      <MemoryRouter>
        <CloudSyncPage />
      </MemoryRouter>,
    );

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'settings:cloud_sync_test' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'settings:cloud_sync_test' }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('cloud_sync_test_connection', expect.anything()),
    );
    act(() => useAuthStore.setState({ currentAccount: { id: 'account-b', name: 'Bob' } }));
    rejectConnection(new Error('old-account-offline'));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(screen.queryByText(/old-account-offline/)).not.toBeInTheDocument();
  });

  it('旧账户删除完成后不清空新账户的配置表单', async () => {
    let resolveDelete!: () => void;
    const originalInvoke = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation((cmd: string, args) => {
      if (cmd === 'cloud_sync_delete_config')
        return new Promise<void>((resolve) => {
          resolveDelete = resolve;
        });
      return originalInvoke(cmd, args);
    });
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    try {
      render(
        <MemoryRouter>
          <CloudSyncPage />
        </MemoryRouter>,
      );
      await waitFor(() =>
        expect(screen.getByRole('button', { name: 'settings:cloud_sync_delete' })).toBeEnabled(),
      );
      fireEvent.click(screen.getByRole('button', { name: 'settings:cloud_sync_delete' }));
      await waitFor(() =>
        expect(invoke).toHaveBeenCalledWith('cloud_sync_delete_config', {
          accountId: 'account-a',
        }),
      );

      act(() => useAuthStore.setState({ currentAccount: { id: 'account-b', name: 'Bob' } }));
      await waitFor(() =>
        expect(screen.getByDisplayValue('https://dav.example.com/')).toBeInTheDocument(),
      );
      resolveDelete();
      await new Promise((resolve) => setTimeout(resolve, 0));

      expect(screen.getByDisplayValue('https://dav.example.com/')).toBeInTheDocument();
    } finally {
      confirmSpy.mockRestore();
    }
  });

  it('旧账户迟到的密码验证不能授权新账户保存', async () => {
    let resolveVerification!: (ok: boolean) => void;
    const originalInvoke = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation((cmd: string, args) => {
      if (cmd === 'cloud_sync_save_config')
        return new Promise<boolean>((resolve) => {
          resolveVerification = resolve;
        });
      return originalInvoke(cmd, args);
    });
    render(
      <MemoryRouter>
        <CloudSyncPage />
      </MemoryRouter>,
    );

    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'settings:cloud_sync_update' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'settings:cloud_sync_update' }));
    fireEvent.click(screen.getByRole('button', { name: '提交主密码' }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('cloud_sync_save_config', expect.anything()),
    );
    act(() => useAuthStore.setState({ currentAccount: { id: 'account-b', name: 'Bob' } }));
    resolveVerification(true);
    await Promise.resolve();

    expect(screen.queryByRole('button', { name: '提交主密码' })).not.toBeInTheDocument();
    expect(
      vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === 'cloud_sync_save_config'),
    ).toHaveLength(1);
  });
});
