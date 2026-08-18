import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AboutPage } from './AboutPage';
import { invoke } from '@tauri-apps/api/core';

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

const mockDesktopCheckForUpdate = vi.fn();
vi.mock('@/lib/updater', () => ({
  desktopCheckForUpdate: () => mockDesktopCheckForUpdate(),
  downloadAndInstallUpdate: vi.fn(),
}));

describe('AboutPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
      info: {
        latestVersion: '1.2.0',
        currentVersion: '1.0.0',
        mandatory: false,
        releaseNotes: 'New features',
        publishedAt: null,
      },
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
