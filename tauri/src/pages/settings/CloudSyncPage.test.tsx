/**
 * P007 拆分回归冒烟测试：设置-云同步页面必须可完整渲染。
 * 背景：核验发现拆分后真机渲染抛「Cannot set indexed properties on this object」，
 * 本测试在 jsdom 中复现渲染路径，防止再次回归。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
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

const listen = vi.fn().mockResolvedValue(() => {});
vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => listen(...(args as [])),
}));

import { invoke } from '@tauri-apps/api/core';

describe('CloudSyncPage 渲染冒烟', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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

  it('完整渲染不抛错，且各 section 均出现', async () => {
    render(
      <MemoryRouter>
        <CloudSyncPage />
      </MemoryRouter>,
    );

    // 配置加载完成后状态卡与各 section 出现
    await waitFor(() => {
      expect(screen.getByText(/WebDAV \(坚果云/)).toBeInTheDocument();
    });

    // 关键交互控件存在
    expect(screen.getByDisplayValue('https://dav.example.com/')).toBeInTheDocument();
    expect(screen.getByDisplayValue('u')).toBeInTheDocument();
  });
});
