import { capabilityFixture } from '@/lib/__fixtures__/platformCapabilityFixture';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { useUpdateStore } from '@/stores/updateStore';
import { AboutPage } from './AboutPage';
import { invoke } from '@tauri-apps/api/core';
import { getPlatformCapabilities } from '@/lib/platformCapabilities';

vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({ children, title }: { children: React.ReactNode; title: string }) => (
    <div data-testid="app-shell" data-title={title}>
      {children}
    </div>
  ),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const adapters = vi.hoisted(() => ({
  desktopCheck: vi.fn(),
  androidCheck: vi.fn(),
  androidCached: vi.fn(),
  desktopDownload: vi.fn(),
  androidDownload: vi.fn(),
  androidInstall: vi.fn(),
}));
const mockDesktopCheckForUpdate = adapters.desktopCheck;
vi.mock('@/lib/updater', () => ({
  checkForUpdate: adapters.desktopCheck,
  androidCheckForUpdate: adapters.androidCheck,
  androidCachedUpdate: adapters.androidCached,
  downloadDesktopUpdate: adapters.desktopDownload,
  ensureApkDownloaded: adapters.androidDownload,
  androidInstallApk: adapters.androidInstall,
}));
vi.mock('@/lib/platformCapabilities', () => ({ getPlatformCapabilities: vi.fn() }));
vi.mock('@/lib/platform', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/platform')>()),
}));

describe('AboutPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getPlatformCapabilities).mockResolvedValue(capabilityFixture('windows'));
    useUpdateStore.setState(useUpdateStore.getInitialState(), true);
    localStorage.clear();
  });

  it('iOS 关于页保留安装版本并说明分发渠道，不显示更新状态或操作', async () => {
    vi.mocked(getPlatformCapabilities).mockResolvedValue(capabilityFixture('ios'));
    vi.mocked(invoke).mockResolvedValue({
      appName: 'SoloSoul',
      version: '2.13.1',
      os: 'ios',
      arch: 'aarch64',
    });
    // 这些模拟适配器一旦误被调用就会产生网络错误；unsupported 必须在调用前决定。
    for (const adapter of Object.values(adapters)) {
      adapter.mockRejectedValue(new Error('unexpected update adapter'));
    }
    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>,
    );
    await waitFor(() =>
      expect(screen.getByText('settings:update_in_app_unsupported')).toBeVisible(),
    );
    expect(screen.getByText('v2.13.1')).toBeVisible();
    expect(screen.queryByText('settings:update_check_failed')).not.toBeInTheDocument();
    expect(screen.queryByText('settings:latest_version')).not.toBeInTheDocument();
    expect(screen.queryByText(/settings:update_available/)).not.toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    expect(useUpdateStore.getState()).toMatchObject({
      unsupportedReason: 'ios',
      updateState: { kind: 'hidden' },
      checking: false,
    });
    for (const adapter of Object.values(adapters)) expect(adapter).not.toHaveBeenCalled();
  });

  it('renders loading placeholder initially', () => {
    vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));
    mockDesktopCheckForUpdate.mockImplementation(() => new Promise(() => {}));
    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>,
    );
    expect(screen.getByTestId('loading-placeholder')).toBeInTheDocument();
    expect(screen.queryByText('settings:loading')).not.toBeInTheDocument();
  });

  it('renders app info after loading', async () => {
    vi.mocked(invoke).mockResolvedValue({
      appName: 'SoloSoul',
      version: '1.2.3',
      os: 'macos',
      arch: 'aarch64',
    });
    mockDesktopCheckForUpdate.mockResolvedValue({ kind: 'up-to-date' });

    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('v1.2.3')).toBeInTheDocument();
    });

    expect(screen.getByText('SoloSoul')).toBeInTheDocument();
    expect(screen.getByText('macOS')).toBeInTheDocument();
    expect(screen.getByText('settings:latest_version')).toBeInTheDocument();
  });

  it('shows update available badge when new version exists', async () => {
    vi.mocked(invoke).mockResolvedValue({
      appName: 'SoloSoul',
      version: '1.0.0',
      os: 'windows',
      arch: 'x86_64',
    });
    mockDesktopCheckForUpdate.mockResolvedValue({
      kind: 'available',
      info: { version: '1.2.0', body: 'New features' },
      update: { version: '1.2.0', close: vi.fn() },
    });

    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText(/settings:update_available/)).toBeInTheDocument();
    });

    expect(screen.getByText('Windows')).toBeInTheDocument();
  });

  it('shows the installed version while the network check is still pending', async () => {
    vi.mocked(invoke).mockResolvedValue({
      appName: 'SoloSoul',
      version: '2.13.1',
      os: 'macos',
      arch: 'aarch64',
    });
    mockDesktopCheckForUpdate.mockReturnValue(new Promise(() => {}));
    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText('v2.13.1')).toBeInTheDocument());
    expect(screen.queryByTestId('loading-placeholder')).not.toBeInTheDocument();
  });

  it('renders external links', async () => {
    vi.mocked(invoke).mockResolvedValue({
      appName: 'SoloSoul',
      version: '1.0.0',
      os: 'linux',
      arch: 'x86_64',
    });
    mockDesktopCheckForUpdate.mockResolvedValue({ kind: 'up-to-date' });

    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('settings:github_repo')).toBeInTheDocument();
    });

    expect(screen.getByText('settings:privacy_policy')).toBeInTheDocument();
    expect(screen.getByText('settings:terms_of_service')).toBeInTheDocument();
  });

  it('handles fetch errors gracefully', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('backend offline'));
    mockDesktopCheckForUpdate.mockResolvedValue({ kind: 'error', message: 'network error' });

    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('settings:could_not_load')).toBeInTheDocument();
    });
  });

  it('shows update check failed badge when update check errors', async () => {
    vi.mocked(invoke).mockResolvedValue({
      appName: 'SoloSoul',
      version: '1.0.0',
      os: 'macos',
      arch: 'aarch64',
    });
    mockDesktopCheckForUpdate.mockResolvedValue({ kind: 'error', message: 'network error' });

    render(
      <MemoryRouter>
        <AboutPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('settings:update_check_failed')).toBeInTheDocument();
    });
  });
});
